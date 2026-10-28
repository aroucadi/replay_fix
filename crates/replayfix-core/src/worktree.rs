//! Ephemeral scratch workspace isolation and dual-layer repository integrity ($\Delta \text{repo} = 0$).
//!
//! `WorktreeManifest` captures a deterministic fingerprint of a directory tree
//! using both a SHA-256 content tree (Layer 1) and `git status --porcelain=v1`
//! when `.git` is present (Layer 2).
//!
//! `ScratchSandbox` clones a target repository's working files into an isolated
//! temporary directory, enforces path boundaries on file reads/writes, tracks
//! spawned child PIDs, and guarantees clean teardown on Windows and Unix even
//! when files are marked read-only or briefly held by exiting child processes.

use crate::digest::{canonical_json, sha256_hex};
use crate::error::{CoreError, Result};
use crate::manifest::TaskManifest;
use crate::platform::Platform;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::Duration;

static SANDBOX_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Maximum single file size (32 MiB) copied or hashed during workspace snapshot to prevent OOM DoS.
const MAX_WORKSPACE_FILE_BYTES: u64 = 32 * 1024 * 1024;

/// Top-level directory names always excluded from workspace snapshots and copies.
const DEFAULT_EXCLUDED_DIRS: [&str; 4] = [".git", ".replayfix", "node_modules", "target"];

/// Bounded retry schedule for directory removal when Windows processes release file handles.
const REMOVE_RETRY_DELAYS_MS: [u64; 5] = [10, 25, 50, 100, 200];

/// Deterministic metadata and SHA-256 content digest for a single regular file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileEntry {
    /// Forward-slash (`/`) normalized path relative to the workspace root.
    pub path: String,
    /// File length in bytes.
    pub size: u64,
    /// Lowercase 64-character hexadecimal SHA-256 digest of the file's bytes.
    pub sha256: String,
}

/// Dual-layer integrity snapshot of a repository directory.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorktreeManifest {
    /// Sorted list of regular files in the workspace.
    pub entries: Vec<FileEntry>,
    /// SHA-256 digest of the RFC 8785 canonical JSON array of `entries`.
    pub tree_sha256: String,
    /// Captured `git status --porcelain=v1 -uall` output if `.git` exists in the root.
    pub git_porcelain: Option<String>,
}

impl WorktreeManifest {
    /// Capture a dual-layer snapshot of `root` using default directory exclusions.
    pub fn capture(root: &Path) -> Result<Self> {
        Self::capture_with_excludes(root, &[])
    }

    /// Capture a dual-layer snapshot of `root`, excluding `extra_excludes` in addition
    /// to `.git`, `.replayfix`, `node_modules`, and `target`.
    pub fn capture_with_excludes(root: &Path, extra_excludes: &[PathBuf]) -> Result<Self> {
        if !root.exists() || !root.is_dir() {
            return Err(CoreError::Worktree(format!(
                "Workspace root '{}' does not exist or is not a directory",
                root.display()
            )));
        }

        let excluded_rel = normalize_excludes(root, extra_excludes);
        let mut entries = Vec::new();
        collect_entries(root, root, &excluded_rel, &mut entries)?;
        entries.sort_by(|a, b| a.path.cmp(&b.path));

        let entries_json = serde_json::to_value(&entries)?;
        let canonical = canonical_json(&entries_json)?;
        let tree_sha256 = sha256_hex(canonical.as_bytes());
        let git_porcelain = capture_git_porcelain(root, &excluded_rel);

        Ok(Self {
            entries,
            tree_sha256,
            git_porcelain,
        })
    }

    /// Verify that `after` is bit-for-bit identical to `self` across both Layer 1
    /// (`tree_sha256` and file entries) and Layer 2 (`git status --porcelain`).
    pub fn verify_unchanged(&self, after: &Self) -> Result<()> {
        if self.tree_sha256 != after.tree_sha256 || self.entries != after.entries {
            let detail = describe_entry_diff(&self.entries, &after.entries);
            return Err(CoreError::SecurityViolation(format!(
                "Base repository mutation detected (Layer 1 content tree SHA-256 changed from {} to {}): {}",
                short_hex(&self.tree_sha256),
                short_hex(&after.tree_sha256),
                detail
            )));
        }

        if let (Some(pre_git), Some(post_git)) = (&self.git_porcelain, &after.git_porcelain) {
            if pre_git != post_git {
                return Err(CoreError::SecurityViolation(
                    "Base repository mutation detected (Layer 2 git status --porcelain changed during run)"
                        .to_string(),
                ));
            }
        }

        Ok(())
    }
}

/// Ephemeral scratch directory for isolated Gate 1 -> Repair -> Gate 2 execution.
///
/// Guarantees $\Delta \text{repo} = 0$ on the user's base repository and automatically
/// terminates registered child processes and deletes the temporary directory on drop.
#[derive(Debug)]
pub struct ScratchSandbox {
    root: PathBuf,
    child_pids: Vec<u32>,
    cleaned: bool,
}

impl ScratchSandbox {
    /// Create a new isolated scratch sandbox in the OS temporary directory and copy
    /// all non-excluded working files from `base_repo` into it.
    pub fn create(base_repo: &Path, task_id: &str) -> Result<Self> {
        Self::create_with_excludes(base_repo, task_id, &[])
    }

    /// Create a new isolated scratch sandbox, excluding `extra_excludes` when copying `base_repo`.
    pub fn create_with_excludes(
        base_repo: &Path,
        task_id: &str,
        extra_excludes: &[PathBuf],
    ) -> Result<Self> {
        if !base_repo.exists() || !base_repo.is_dir() {
            return Err(CoreError::Worktree(format!(
                "Base repository '{}' does not exist or is not a directory",
                base_repo.display()
            )));
        }

        let safe_task: String = task_id
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        let seq = SANDBOX_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir_name = format!(
            "replayfix-sandbox-{}-{}-{}",
            safe_task,
            std::process::id(),
            seq
        );
        let sandbox_root = std::env::temp_dir().join(dir_name);

        if sandbox_root.exists() {
            remove_dir_robust(&sandbox_root)?;
        }
        fs::create_dir_all(&sandbox_root)?;

        let excluded_rel = normalize_excludes(base_repo, extra_excludes);
        if let Err(e) = copy_tree(base_repo, base_repo, &sandbox_root, &excluded_rel) {
            let _ = remove_dir_robust(&sandbox_root);
            return Err(e);
        }

        Ok(Self {
            root: sandbox_root,
            child_pids: Vec::new(),
            cleaned: false,
        })
    }

    /// Path to the scratch sandbox root directory.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Register a spawned child process PID so its entire process tree is terminated
    /// before the scratch directory is removed.
    pub fn register_child_pid(&mut self, pid: u32) {
        if !self.child_pids.contains(&pid) {
            self.child_pids.push(pid);
        }
    }

    /// Resolve and validate a relative path strictly inside the sandbox root.
    pub fn resolve_safe_path(&self, rel_path: &str) -> Result<PathBuf> {
        let validated = TaskManifest::assert_safe_path(rel_path, "sandbox_path")?;
        if validated.is_absolute() {
            return Err(CoreError::SecurityViolation(format!(
                "Sandbox path '{rel_path}' must be relative to the sandbox root"
            )));
        }
        for comp in validated.components() {
            match comp {
                Component::Normal(_) | Component::CurDir => {}
                _ => {
                    return Err(CoreError::SecurityViolation(format!(
                        "Sandbox path '{rel_path}' contains disallowed component"
                    )));
                }
            }
        }
        Ok(self.root.join(validated))
    }

    /// Read a UTF-8 file relative to the sandbox root.
    pub fn read_file(&self, rel_path: &str) -> Result<String> {
        let full = self.resolve_safe_path(rel_path)?;
        reject_symlink_chain(&self.root, &full)?;
        Ok(fs::read_to_string(full)?)
    }

    /// Write UTF-8 content to a file relative to the sandbox root, creating parent directories as needed.
    pub fn write_file(&self, rel_path: &str, content: &str) -> Result<PathBuf> {
        let full = self.resolve_safe_path(rel_path)?;
        reject_symlink_chain(&self.root, &full)?;
        if let Some(parent) = full.parent() {
            fs::create_dir_all(parent)?;
            reject_symlink_chain(&self.root, &full)?;
            if let (Ok(canon_root), Ok(canon_parent)) =
                (fs::canonicalize(&self.root), fs::canonicalize(parent))
            {
                if !canon_parent.starts_with(&canon_root) {
                    return Err(CoreError::SecurityViolation(format!(
                        "Sandbox write parent '{}' escaped canonical sandbox root '{}'",
                        canon_parent.display(),
                        canon_root.display()
                    )));
                }
            }
        }
        if let Ok(meta) = fs::symlink_metadata(&full) {
            if is_symlink_or_reparse_point(&meta) {
                return Err(CoreError::SecurityViolation(format!(
                    "Symlink escape detected inside sandbox at '{}'",
                    full.display()
                )));
            }
            let mut perms = meta.permissions();
            if perms.readonly() {
                #[allow(clippy::permissions_set_readonly_false)]
                perms.set_readonly(false);
                let _ = fs::set_permissions(&full, perms);
            }
        }
        fs::write(&full, content)?;
        Ok(full)
    }

    /// Explicitly terminate all registered child process trees and remove the sandbox directory.
    ///
    /// Idempotent: calling `teardown` multiple times is a safe no-op.
    pub fn teardown(&mut self) -> Result<()> {
        if self.cleaned {
            return Ok(());
        }
        for &pid in &self.child_pids {
            let _ = Platform::terminate_process_tree(pid);
        }
        self.child_pids.clear();

        if self.root.exists() {
            remove_dir_robust(&self.root)?;
        }
        self.cleaned = true;
        Ok(())
    }
}

impl Drop for ScratchSandbox {
    fn drop(&mut self) {
        let _ = self.teardown();
    }
}

fn short_hex(digest: &str) -> &str {
    digest.get(..12).unwrap_or(digest)
}

fn to_forward_slash(path: &Path) -> String {
    let mut parts = Vec::new();
    for comp in path.components() {
        if let Component::Normal(os) = comp {
            parts.push(os.to_string_lossy().into_owned());
        }
    }
    parts.join("/")
}

fn normalize_excludes(root: &Path, extra_excludes: &[PathBuf]) -> BTreeSet<String> {
    let mut set: BTreeSet<String> = DEFAULT_EXCLUDED_DIRS
        .iter()
        .map(|s| (*s).to_string())
        .collect();

    let canonical_root = fs::canonicalize(root).ok();

    for extra in extra_excludes {
        if extra.is_absolute() {
            if let Ok(stripped) = extra.strip_prefix(root) {
                let rel = to_forward_slash(stripped);
                if !rel.is_empty() {
                    set.insert(rel);
                }
            } else if let (Some(c_root), Ok(c_extra)) = (&canonical_root, fs::canonicalize(extra)) {
                if let Ok(stripped) = c_extra.strip_prefix(c_root) {
                    let rel = to_forward_slash(stripped);
                    if !rel.is_empty() {
                        set.insert(rel);
                    }
                }
            }
        } else {
            let rel = to_forward_slash(extra);
            if !rel.is_empty() {
                set.insert(rel);
            }
        }
    }
    set
}

fn is_excluded(rel_slash: &str, excluded_rel: &BTreeSet<String>) -> bool {
    if rel_slash.is_empty() {
        return false;
    }
    for ex in excluded_rel {
        if rel_slash == ex
            || rel_slash
                .strip_prefix(ex.as_str())
                .is_some_and(|rest| rest.starts_with('/'))
        {
            return true;
        }
    }
    false
}

fn collect_entries(
    root: &Path,
    current: &Path,
    excluded_rel: &BTreeSet<String>,
    out: &mut Vec<FileEntry>,
) -> Result<()> {
    let mut dir_entries: Vec<fs::DirEntry> =
        fs::read_dir(current)?.collect::<std::io::Result<Vec<_>>>()?;
    dir_entries.sort_by_key(|e| e.file_name());

    for entry in dir_entries {
        let path = entry.path();
        let rel = path.strip_prefix(root).map_err(|e| {
            CoreError::Worktree(format!("Failed to strip root prefix from path: {e}"))
        })?;
        let rel_slash = to_forward_slash(rel);
        if is_excluded(&rel_slash, excluded_rel) {
            continue;
        }

        let symlink_meta = fs::symlink_metadata(&path)?;
        if is_symlink_or_reparse_point(&symlink_meta) {
            continue;
        } else if symlink_meta.is_dir() {
            collect_entries(root, &path, excluded_rel, out)?;
        } else if symlink_meta.is_file() {
            let size = entry.metadata()?.len();
            if size > MAX_WORKSPACE_FILE_BYTES {
                return Err(CoreError::Worktree(format!(
                    "Workspace file '{}' is {} bytes, exceeding the {} byte safety limit",
                    rel_slash, size, MAX_WORKSPACE_FILE_BYTES
                )));
            }
            let bytes = fs::read(&path)?;
            let sha256 = sha256_hex(&bytes);
            out.push(FileEntry {
                path: rel_slash,
                size: bytes.len() as u64,
                sha256,
            });
        }
    }
    Ok(())
}

fn copy_tree(
    base_root: &Path,
    current_src: &Path,
    sandbox_root: &Path,
    excluded_rel: &BTreeSet<String>,
) -> Result<()> {
    let mut dir_entries: Vec<fs::DirEntry> =
        fs::read_dir(current_src)?.collect::<std::io::Result<Vec<_>>>()?;
    dir_entries.sort_by_key(|e| e.file_name());

    for entry in dir_entries {
        let src_path = entry.path();
        let rel = src_path.strip_prefix(base_root).map_err(|e| {
            CoreError::Worktree(format!("Failed to strip base root prefix during copy: {e}"))
        })?;
        let rel_slash = to_forward_slash(rel);
        if is_excluded(&rel_slash, excluded_rel) {
            continue;
        }

        let symlink_meta = fs::symlink_metadata(&src_path)?;
        if is_symlink_or_reparse_point(&symlink_meta) {
            continue;
        }
        let dst_path = sandbox_root.join(rel);
        if symlink_meta.is_dir() {
            fs::create_dir_all(&dst_path)?;
            copy_tree(base_root, &src_path, sandbox_root, excluded_rel)?;
        } else if symlink_meta.is_file() {
            let size = entry.metadata()?.len();
            if size > MAX_WORKSPACE_FILE_BYTES {
                return Err(CoreError::Worktree(format!(
                    "Workspace file '{}' is {} bytes, exceeding the {} byte safety limit",
                    rel_slash, size, MAX_WORKSPACE_FILE_BYTES
                )));
            }
            if let Some(parent) = dst_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

fn capture_git_porcelain(root: &Path, excluded_rel: &BTreeSet<String>) -> Option<String> {
    if !root.join(".git").exists() {
        return None;
    }
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["status", "--porcelain=v1", "-uall"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let raw = String::from_utf8_lossy(&output.stdout);
    let filtered: Vec<&str> = raw
        .lines()
        .filter(|line| {
            // Porcelain v1 format: "XY path" or "XY orig -> renamed"
            let path_part = line.get(3..).unwrap_or("").trim();
            let normalized = path_part.trim_matches('"').replace('\\', "/");
            !is_excluded(&normalized, excluded_rel)
        })
        .collect();
    Some(filtered.join("\n"))
}

fn describe_entry_diff(before: &[FileEntry], after: &[FileEntry]) -> String {
    for b in before {
        match after.iter().find(|a| a.path == b.path) {
            None => return format!("file '{}' was deleted", b.path),
            Some(a) if a.sha256 != b.sha256 || a.size != b.size => {
                return format!("file '{}' was modified", b.path);
            }
            Some(_) => {}
        }
    }
    for a in after {
        if !before.iter().any(|b| b.path == a.path) {
            return format!("unexpected file '{}' was created", a.path);
        }
    }
    "workspace file entries changed".to_string()
}

pub(crate) fn is_symlink_or_reparse_point(meta: &fs::Metadata) -> bool {
    if meta.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;
        if (meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT) != 0 {
            return true;
        }
    }
    false
}

fn reject_symlink_chain(sandbox_root: &Path, target: &Path) -> Result<()> {
    if let Ok(root_meta) = fs::symlink_metadata(sandbox_root) {
        if is_symlink_or_reparse_point(&root_meta) {
            return Err(CoreError::SecurityViolation(format!(
                "Symlink escape detected on sandbox root '{}'",
                sandbox_root.display()
            )));
        }
    }
    let mut cur = sandbox_root.to_path_buf();
    if let Ok(rel) = target.strip_prefix(sandbox_root) {
        for comp in rel.components() {
            cur.push(comp);
            if let Ok(meta) = fs::symlink_metadata(&cur) {
                if is_symlink_or_reparse_point(&meta) {
                    return Err(CoreError::SecurityViolation(format!(
                        "Symlink escape detected inside sandbox at '{}'",
                        cur.display()
                    )));
                }
            }
        }
    }
    Ok(())
}

fn clear_readonly_recursive(path: &Path) {
    if let Ok(meta) = fs::symlink_metadata(path) {
        let mut perms = meta.permissions();
        if perms.readonly() {
            #[allow(clippy::permissions_set_readonly_false)]
            perms.set_readonly(false);
            let _ = fs::set_permissions(path, perms);
        }
        if meta.file_type().is_dir() {
            if let Ok(entries) = fs::read_dir(path) {
                for entry in entries.flatten() {
                    clear_readonly_recursive(&entry.path());
                }
            }
        }
    }
}

fn remove_dir_robust(path: &Path) -> Result<()> {
    clear_readonly_recursive(path);
    let mut last_err = None;
    for &delay_ms in &REMOVE_RETRY_DELAYS_MS {
        if !path.exists() {
            return Ok(());
        }
        match fs::remove_dir_all(path) {
            Ok(()) => return Ok(()),
            Err(e) => {
                last_err = Some(e);
                clear_readonly_recursive(path);
                thread::sleep(Duration::from_millis(delay_ms));
            }
        }
    }
    if !path.exists() {
        return Ok(());
    }
    Err(CoreError::Worktree(format!(
        "Failed to remove scratch directory '{}' after retries: {}",
        path.display(),
        last_err
            .map(|e| e.to_string())
            .unwrap_or_else(|| "unknown error".to_string())
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_test_dir(label: &str) -> PathBuf {
        let seq = SANDBOX_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "replayfix-worktree-test-{}-{}-{}",
            label,
            std::process::id(),
            seq
        ));
        let _ = remove_dir_robust(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn test_worktree_manifest_deterministic_and_excludes_standard_dirs() {
        let dir = temp_test_dir("deterministic");
        fs::create_dir_all(dir.join("src")).unwrap();
        fs::write(dir.join("src/b.js"), "const b = 2;\n").unwrap();
        fs::write(dir.join("src/a.js"), "const a = 1;\n").unwrap();

        // Standard excluded directories should be ignored
        fs::create_dir_all(dir.join(".replayfix")).unwrap();
        fs::write(dir.join(".replayfix/audit_receipt.json"), "{}").unwrap();
        fs::create_dir_all(dir.join("node_modules/pkg")).unwrap();
        fs::write(
            dir.join("node_modules/pkg/index.js"),
            "module.exports = {};",
        )
        .unwrap();

        let m1 = WorktreeManifest::capture(&dir).unwrap();
        let m2 = WorktreeManifest::capture(&dir).unwrap();
        assert_eq!(m1, m2);
        assert_eq!(m1.entries.len(), 2);
        assert_eq!(m1.entries[0].path, "src/a.js");
        assert_eq!(m1.entries[1].path, "src/b.js");
        assert_eq!(m1.tree_sha256.len(), 64);
        assert!(m1.verify_unchanged(&m2).is_ok());

        let _ = remove_dir_robust(&dir);
    }

    #[test]
    fn test_worktree_manifest_detects_modify_add_and_delete() {
        let dir = temp_test_dir("mutations");
        fs::create_dir_all(dir.join("src")).unwrap();
        fs::write(dir.join("src/app.js"), "export const v = 1;\n").unwrap();

        let baseline = WorktreeManifest::capture(&dir).unwrap();

        // 1. Modify existing file
        fs::write(dir.join("src/app.js"), "export const v = 2;\n").unwrap();
        let modified = WorktreeManifest::capture(&dir).unwrap();
        let err = baseline.verify_unchanged(&modified).unwrap_err();
        assert!(err.to_string().contains("src/app.js"));
        assert!(err.to_string().contains("modified"));

        // Restore and add a new file
        fs::write(dir.join("src/app.js"), "export const v = 1;\n").unwrap();
        fs::write(dir.join("src/extra.js"), "// extra\n").unwrap();
        let added = WorktreeManifest::capture(&dir).unwrap();
        let err = baseline.verify_unchanged(&added).unwrap_err();
        assert!(err.to_string().contains("src/extra.js"));
        assert!(err.to_string().contains("created"));

        // Delete original file
        fs::remove_file(dir.join("src/extra.js")).unwrap();
        fs::remove_file(dir.join("src/app.js")).unwrap();
        let deleted = WorktreeManifest::capture(&dir).unwrap();
        let err = baseline.verify_unchanged(&deleted).unwrap_err();
        assert!(err.to_string().contains("src/app.js"));
        assert!(err.to_string().contains("deleted"));

        let _ = remove_dir_robust(&dir);
    }

    #[test]
    fn test_worktree_manifest_git_porcelain_layer2() {
        let dir = temp_test_dir("git-layer2");
        fs::write(dir.join("index.js"), "console.log('clean');\n").unwrap();

        let git_init = Command::new("git").arg("init").current_dir(&dir).output();
        if let Ok(out) = git_init {
            if out.status.success() {
                let m1 = WorktreeManifest::capture(&dir).unwrap();
                assert!(m1.git_porcelain.is_some());

                // Writing to .replayfix is excluded from both Layer 1 and Layer 2
                fs::create_dir_all(dir.join(".replayfix")).unwrap();
                fs::write(dir.join(".replayfix/audit_receipt.json"), "{}").unwrap();
                let m2 = WorktreeManifest::capture(&dir).unwrap();
                assert!(m1.verify_unchanged(&m2).is_ok());
            }
        }

        let _ = remove_dir_robust(&dir);
    }

    #[test]
    fn test_scratch_sandbox_isolates_writes_and_cleans_up_readonly_files() {
        let base = temp_test_dir("sandbox-base");
        fs::create_dir_all(base.join("src")).unwrap();
        fs::write(base.join("src/cart-summary.js"), "original-content\n").unwrap();

        let pre = WorktreeManifest::capture(&base).unwrap();
        let sandbox_path: PathBuf;
        {
            let mut sandbox = ScratchSandbox::create(&base, "RF-01").unwrap();
            sandbox_path = sandbox.root().to_path_buf();
            assert!(sandbox_path.exists());

            assert_eq!(
                sandbox.read_file("src/cart-summary.js").unwrap(),
                "original-content\n"
            );

            // Mutate file inside sandbox and mark it read-only to test Windows lock/attrib defense
            let written = sandbox
                .write_file("src/cart-summary.js", "patched-in-sandbox\n")
                .unwrap();
            let mut perms = fs::metadata(&written).unwrap().permissions();
            perms.set_readonly(true);
            fs::set_permissions(&written, perms).unwrap();

            // Verify traversal and trailing-dot/multi-dot DOS paths are rejected
            assert!(sandbox.read_file("../outside.txt").is_err());
            assert!(sandbox.write_file("../../escape.txt", "bad").is_err());
            assert!(sandbox.write_file("src/cart-summary.js.", "bad").is_err());
            assert!(sandbox.write_file("src./cart-summary.js", "bad").is_err());
            assert!(sandbox.write_file("src/CON.txt.bak", "bad").is_err());

            // On Windows, create a real NTFS directory junction (reparse point) without
            // needing elevation and verify read_file and write_file reject it fail-closed.
            #[cfg(windows)]
            {
                let outside_dir = temp_test_dir("junction-outside");
                fs::write(outside_dir.join("secret.txt"), "do-not-touch\n").unwrap();
                let junction_path = sandbox.root().join("src").join("linked_dir");
                let mklink = Command::new("cmd")
                    .args([
                        "/C",
                        "mklink",
                        "/J",
                        &junction_path.to_string_lossy(),
                        &outside_dir.to_string_lossy(),
                    ])
                    .output()
                    .unwrap();
                assert!(mklink.status.success(), "mklink /J should succeed");

                let read_err = sandbox.read_file("src/linked_dir/secret.txt").unwrap_err();
                assert!(matches!(read_err, CoreError::SecurityViolation(_)));
                let write_err = sandbox
                    .write_file("src/linked_dir/secret.txt", "overwritten")
                    .unwrap_err();
                assert!(matches!(write_err, CoreError::SecurityViolation(_)));
                assert_eq!(
                    fs::read_to_string(outside_dir.join("secret.txt")).unwrap(),
                    "do-not-touch\n"
                );
                let _ = fs::remove_dir(&junction_path);
                let _ = remove_dir_robust(&outside_dir);
            }

            // Base repository must remain completely untouched while sandbox is active
            let mid = WorktreeManifest::capture(&base).unwrap();
            assert!(pre.verify_unchanged(&mid).is_ok());
            assert_eq!(
                fs::read_to_string(base.join("src/cart-summary.js")).unwrap(),
                "original-content\n"
            );

            sandbox.teardown().unwrap();
            assert!(!sandbox_path.exists());
        }
        assert!(!sandbox_path.exists());

        let post = WorktreeManifest::capture(&base).unwrap();
        assert!(pre.verify_unchanged(&post).is_ok());

        let _ = remove_dir_robust(&base);
    }
}
