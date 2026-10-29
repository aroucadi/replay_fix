use crate::digest::canonical_json;
use crate::error::{CoreError, Result};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static ATOMIC_WRITE_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Write bytes to a target file atomically using a unique temporary file and rename.
///
/// On rename failure, the temporary file is unlinked immediately to prevent orphaned temp files.
pub fn atomic_write_file(path: impl AsRef<Path>, data: &[u8]) -> Result<()> {
    let path = path.as_ref();
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    if !parent.exists() {
        fs::create_dir_all(parent)?;
    }

    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("file");
    let count = ATOMIC_WRITE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let temp_name = format!(".{}_{}_{}.tmp", file_name, std::process::id(), count);
    let temp_path = parent.join(temp_name);

    fs::write(&temp_path, data)?;
    if let Err(e) = fs::rename(&temp_path, path) {
        let _ = fs::remove_file(&temp_path);
        return Err(e.into());
    }

    Ok(())
}

/// Validate that an artifact file name does not contain path traversal or directory separators.
fn validate_file_name(file_name: &str) -> Result<()> {
    if file_name.is_empty()
        || file_name.contains('/')
        || file_name.contains('\\')
        || file_name.contains('\0')
        || file_name == "."
        || file_name == ".."
        || file_name.starts_with("..")
    {
        return Err(CoreError::SecurityViolation(format!(
            "Invalid artifact file name '{file_name}': directory separators and traversal are prohibited"
        )));
    }
    Ok(())
}

/// Manager for disaggregated verification artifacts stored in `.replayfix/artifacts/`.
#[derive(Debug, Clone)]
pub struct ArtifactStore {
    root_dir: PathBuf,
}

impl ArtifactStore {
    /// Initialize the artifact store at the specified directory, creating it if needed.
    pub fn new(root: impl AsRef<Path>) -> Result<Self> {
        let root_dir = root.as_ref().to_path_buf();
        if !root_dir.exists() {
            fs::create_dir_all(&root_dir).map_err(|e| {
                CoreError::Io(std::io::Error::new(
                    e.kind(),
                    format!("Failed to create artifacts directory: {}", e),
                ))
            })?;
        }
        Ok(Self { root_dir })
    }

    /// Path to root artifacts directory.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root_dir
    }

    /// Write raw text content atomically (e.g. `candidate.patch`).
    pub fn write_text(&self, file_name: &str, content: &str) -> Result<PathBuf> {
        validate_file_name(file_name)?;
        let target_path = self.root_dir.join(file_name);
        atomic_write_file(&target_path, content.as_bytes())?;
        Ok(target_path)
    }

    /// Write a serializable value as formatted, canonical compact JSON atomically.
    pub fn write_canonical_json<T: Serialize>(
        &self,
        file_name: &str,
        value: &T,
    ) -> Result<PathBuf> {
        validate_file_name(file_name)?;
        let json_val = serde_json::to_value(value)?;
        let json_str = canonical_json(&json_val)?;
        let target_path = self.root_dir.join(file_name);
        atomic_write_file(&target_path, json_str.as_bytes())?;
        Ok(target_path)
    }

    /// Write a serializable value as formatted, indented JSON atomically.
    pub fn write_json_pretty<T: Serialize>(&self, file_name: &str, value: &T) -> Result<PathBuf> {
        validate_file_name(file_name)?;
        let mut json_bytes = serde_json::to_vec_pretty(value)?;
        json_bytes.push(b'\n');
        let target_path = self.root_dir.join(file_name);
        atomic_write_file(&target_path, &json_bytes)?;
        Ok(target_path)
    }

    /// Write a serializable value as JSON atomically (formatted by default).
    pub fn write_json<T: Serialize>(&self, file_name: &str, value: &T) -> Result<PathBuf> {
        self.write_json_pretty(file_name, value)
    }

    /// Write binary data atomically (e.g. `playwright.trace.zip`).
    pub fn write_bytes(&self, file_name: &str, data: &[u8]) -> Result<PathBuf> {
        validate_file_name(file_name)?;
        let target_path = self.root_dir.join(file_name);
        atomic_write_file(&target_path, data)?;
        Ok(target_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_creates_dir_and_writes_atomically() {
        let temp_dir = std::env::temp_dir().join("replayfix_artifact_store_test");
        let _ = fs::remove_dir_all(&temp_dir);

        let store = ArtifactStore::new(&temp_dir).unwrap();
        assert!(temp_dir.exists());

        let patch_path = store
            .write_text("candidate.patch", "--- a\n+++ b\n")
            .unwrap();
        assert!(patch_path.exists());
        assert_eq!(fs::read_to_string(&patch_path).unwrap(), "--- a\n+++ b\n");

        // Test atomic overwrite when file already exists
        let patch_path2 = store
            .write_text("candidate.patch", "--- a\n+++ b\nupdated\n")
            .unwrap();
        assert_eq!(patch_path, patch_path2);
        assert_eq!(
            fs::read_to_string(&patch_path).unwrap(),
            "--- a\n+++ b\nupdated\n"
        );

        let json_data = serde_json::json!({ "status": "VERIFIED" });
        let json_path = store.write_json("status.json", &json_data).unwrap();
        assert!(json_path.exists());
        let read_back: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&json_path).unwrap()).unwrap();
        assert_eq!(read_back["status"], "VERIFIED");

        let canonical_path = store
            .write_canonical_json("canonical.json", &json_data)
            .unwrap();
        assert!(canonical_path.exists());

        let bin_path = store.write_bytes("test.bin", b"binarydata").unwrap();
        assert!(bin_path.exists());
        assert_eq!(fs::read(&bin_path).unwrap(), b"binarydata");

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_file_name_traversal_is_rejected() {
        let temp_dir = std::env::temp_dir().join("replayfix_artifact_traversal_test");
        let _ = fs::remove_dir_all(&temp_dir);
        let store = ArtifactStore::new(&temp_dir).unwrap();

        assert!(store.write_text("../escaped.txt", "evil").is_err());
        assert!(store.write_text("sub/dir.txt", "evil").is_err());
        assert!(store.write_text("sub\\dir.txt", "evil").is_err());
        assert!(store.write_text("..", "evil").is_err());
        assert!(store.write_text("", "evil").is_err());

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
