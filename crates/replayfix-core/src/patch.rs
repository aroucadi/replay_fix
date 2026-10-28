//! Dependency-free, in-memory unified diff applier.
//!
//! Deliberately strict: single file, exact hunk offsets, exact context, no
//! fuzz. A patch that only applies "approximately" must never be verifiable.

use crate::error::{CoreError, Result};
use crate::manifest::TaskManifest;
use regex::Regex;
use std::sync::OnceLock;

fn hunk_header_regex() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@").ok())
        .as_ref()
}

/// Result of applying a unified diff in memory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchOutcome {
    pub target_path: String,
    pub patched: String,
    pub insertions: usize,
    pub deletions: usize,
    pub files_changed: usize,
}

impl PatchOutcome {
    /// True when the patch leaves the source byte-identical.
    pub fn is_noop(&self, original: &str) -> bool {
        self.patched == original
    }
}

/// Shorten text echoed in error messages so a mistaken `--mock-patch` pointing
/// at an unrelated file cannot dump its contents into logs or reports.
fn snippet(text: &str) -> String {
    const MAX: usize = 60;
    let mut shown: String = text.chars().take(MAX).collect();
    if text.chars().count() > MAX {
        shown.push('…');
    }
    format!("{shown:?}")
}

struct Hunk {
    old_start: usize,
    old_count: usize,
    lines: Vec<(char, String)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UnifiedPatcher;

impl UnifiedPatcher {
    /// Apply a single-file unified diff to `original` without touching the filesystem.
    pub fn apply_diff(original: &str, diff: &str) -> Result<PatchOutcome> {
        let (target_path, hunks) = Self::parse(diff)?;

        let eol = if original.contains("\r\n") {
            "\r\n"
        } else {
            "\n"
        };
        let source: Vec<&str> = original.lines().collect();
        let mut output: Vec<String> = Vec::with_capacity(source.len());
        let mut cursor = 0usize;
        let (mut insertions, mut deletions) = (0usize, 0usize);

        for (n, hunk) in hunks.iter().enumerate() {
            let label = n + 1;
            let start = if hunk.old_count == 0 {
                hunk.old_start
            } else if hunk.old_start == 0 {
                return Err(CoreError::Patch(format!(
                    "Hunk {label}: start line 0 is invalid for a non-empty range"
                )));
            } else {
                hunk.old_start - 1
            };

            if start < cursor {
                return Err(CoreError::Patch(format!(
                    "Hunk {label}: overlaps or precedes the previous hunk"
                )));
            }
            if start > source.len() {
                return Err(CoreError::Patch(format!(
                    "Hunk {label}: starts beyond the end of the file"
                )));
            }

            output.extend(source[cursor..start].iter().map(|s| s.to_string()));
            let mut idx = start;

            for (kind, text) in &hunk.lines {
                match kind {
                    ' ' | '-' => {
                        let actual = source.get(idx).ok_or_else(|| {
                            CoreError::Patch(format!("Hunk {label}: runs past the end of the file"))
                        })?;
                        if actual != text {
                            return Err(CoreError::Patch(format!(
                                "Hunk {label}: context mismatch at line {}: expected {}, found {}",
                                idx + 1,
                                snippet(text),
                                snippet(actual)
                            )));
                        }
                        if *kind == ' ' {
                            output.push(text.clone());
                        } else {
                            deletions += 1;
                        }
                        idx += 1;
                    }
                    _ => {
                        output.push(text.clone());
                        insertions += 1;
                    }
                }
            }
            cursor = idx;
        }

        output.extend(source[cursor..].iter().map(|s| s.to_string()));

        let mut patched = output.join(eol);
        if original.ends_with('\n') && !patched.is_empty() {
            patched.push_str(eol);
        }

        Ok(PatchOutcome {
            target_path,
            patched,
            insertions,
            deletions,
            files_changed: 1,
        })
    }

    fn parse(diff: &str) -> Result<(String, Vec<Hunk>)> {
        let lines: Vec<&str> = diff.lines().collect();

        let header = lines
            .iter()
            .position(|l| l.starts_with("--- "))
            .ok_or_else(|| CoreError::Patch("Missing '---' file header".to_string()))?;
        let new_line = lines
            .get(header + 1)
            .filter(|l| l.starts_with("+++ "))
            .ok_or_else(|| CoreError::Patch("Missing '+++' file header".to_string()))?;

        let old_path = Self::header_path(&lines[header][4..])?;
        let new_path = Self::header_path(&new_line[4..])?;
        if old_path != new_path {
            return Err(CoreError::Patch(
                "Renames are not supported: '---' and '+++' paths differ".to_string(),
            ));
        }
        if crate::repair::TestFreezeGuard::is_protected_test_or_config_path(&new_path) {
            return Err(CoreError::SecurityViolation(format!(
                "TEST_MUTATION_PROHIBITED: unified diff targets protected test or configuration path '{new_path}'"
            )));
        }
        TaskManifest::assert_safe_path(&new_path, "patch header")?;

        let mut hunks = Vec::new();
        let mut i = header + 2;
        while i < lines.len() {
            let line = lines[i];
            let Some(caps) = hunk_header_regex().and_then(|re| re.captures(line)) else {
                if line.starts_with("--- ") {
                    return Err(CoreError::Patch(
                        "Multi-file patches are not supported".to_string(),
                    ));
                }
                if line.trim().is_empty() || line.starts_with('\\') {
                    i += 1;
                    continue;
                }
                return Err(CoreError::Patch(format!(
                    "Unexpected line outside a hunk: {}",
                    snippet(line)
                )));
            };

            let num = |idx: usize, default: usize| -> Result<usize> {
                caps.get(idx).map_or(Ok(default), |m| {
                    m.as_str().parse::<usize>().map_err(|_| {
                        CoreError::Patch(format!("Hunk header number out of range: {line:?}"))
                    })
                })
            };
            let old_start = num(1, 0)?;
            let old_count = num(2, 1)?;
            let new_count = num(4, 1)?;

            i += 1;
            let (mut seen_old, mut seen_new) = (0usize, 0usize);
            let mut body = Vec::new();
            while seen_old < old_count || seen_new < new_count {
                let line = lines.get(i).ok_or_else(|| {
                    CoreError::Patch("Hunk is shorter than its header declares".to_string())
                })?;
                i += 1;
                let (kind, text) = match line.chars().next() {
                    Some(' ') => (' ', &line[1..]),
                    Some('-') => ('-', &line[1..]),
                    Some('+') => ('+', &line[1..]),
                    Some('\\') => continue,
                    None => (' ', ""),
                    Some(_) => {
                        return Err(CoreError::Patch(format!(
                            "Malformed hunk line (expected ' ', '-' or '+'): {}",
                            snippet(line)
                        )))
                    }
                };
                match kind {
                    ' ' => {
                        seen_old += 1;
                        seen_new += 1;
                    }
                    '-' => seen_old += 1,
                    _ => seen_new += 1,
                }
                if seen_old > old_count || seen_new > new_count {
                    return Err(CoreError::Patch(
                        "Hunk is longer than its header declares".to_string(),
                    ));
                }
                body.push((kind, text.to_string()));
            }

            hunks.push(Hunk {
                old_start,
                old_count,
                lines: body,
            });
        }

        if hunks.is_empty() {
            return Err(CoreError::Patch("Patch contains no hunks".to_string()));
        }
        Ok((new_path, hunks))
    }

    fn header_path(raw: &str) -> Result<String> {
        let path = raw.split('\t').next().unwrap_or("").trim();
        if path.is_empty() {
            return Err(CoreError::Patch("Empty path in file header".to_string()));
        }
        if path == "/dev/null" {
            return Err(CoreError::Patch(
                "File creation and deletion are not supported".to_string(),
            ));
        }
        // Diffs authored on Windows may use backslashes; compare paths in one form.
        let path = path.replace('\\', "/");
        let stripped = path
            .strip_prefix("a/")
            .or_else(|| path.strip_prefix("b/"))
            .unwrap_or(&path);
        Ok(stripped.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &str = "line1\nline2\nline3\nline4\nline5\n";

    fn diff(body: &str) -> String {
        format!("--- a/f.txt\n+++ b/f.txt\n{body}")
    }

    #[test]
    fn test_apply_simple_replacement() {
        let d = diff("@@ -2,3 +2,3 @@\n line2\n-line3\n+LINE3\n line4\n");
        let out = UnifiedPatcher::apply_diff(SRC, &d).unwrap();
        assert_eq!(out.patched, "line1\nline2\nLINE3\nline4\nline5\n");
        assert_eq!((out.insertions, out.deletions), (1, 1));
        assert_eq!(out.target_path, "f.txt");
        assert_eq!(out.files_changed, 1);
    }

    #[test]
    fn test_apply_multiple_hunks() {
        let d = diff("@@ -1,2 +1,2 @@\n-line1\n+first\n line2\n@@ -5 +5,2 @@\n line5\n+line6\n");
        let out = UnifiedPatcher::apply_diff(SRC, &d).unwrap();
        assert_eq!(out.patched, "first\nline2\nline3\nline4\nline5\nline6\n");
        assert_eq!((out.insertions, out.deletions), (2, 1));
    }

    #[test]
    fn test_apply_pure_insertion_with_zero_old_count() {
        let d = diff("@@ -2,0 +3 @@\n+inserted\n");
        let out = UnifiedPatcher::apply_diff(SRC, &d).unwrap();
        assert_eq!(out.patched, "line1\nline2\ninserted\nline3\nline4\nline5\n");
    }

    #[test]
    fn test_context_mismatch_is_rejected() {
        let d = diff("@@ -2,2 +2,2 @@\n line2\n-WRONG\n+x\n");
        let err = UnifiedPatcher::apply_diff(SRC, &d).unwrap_err();
        assert!(matches!(err, CoreError::Patch(m) if m.contains("context mismatch")));
    }

    #[test]
    fn test_wrong_offset_is_not_searched_for() {
        // Correct text, wrong line number: exact offsets only, no fuzz.
        let d = diff("@@ -4,1 +4,1 @@\n-line2\n+x\n");
        assert!(UnifiedPatcher::apply_diff(SRC, &d).is_err());
    }

    #[test]
    fn test_missing_headers_are_rejected() {
        assert!(UnifiedPatcher::apply_diff(SRC, "@@ -1 +1 @@\n-a\n+b\n").is_err());
        assert!(UnifiedPatcher::apply_diff(SRC, "--- a/f.txt\n@@ -1 +1 @@\n").is_err());
    }

    #[test]
    fn test_no_hunks_is_rejected() {
        let err = UnifiedPatcher::apply_diff(SRC, "--- a/f.txt\n+++ b/f.txt\n").unwrap_err();
        assert!(matches!(err, CoreError::Patch(m) if m.contains("no hunks")));
    }

    #[test]
    fn test_hunk_count_mismatch_is_rejected() {
        let short = diff("@@ -1,3 +1,3 @@\n line1\n-line2\n");
        assert!(UnifiedPatcher::apply_diff(SRC, &short).is_err());
        let long = diff("@@ -1,1 +1,1 @@\n-line1\n+a\n+b\n");
        assert!(UnifiedPatcher::apply_diff(SRC, &long).is_err());
    }

    #[test]
    fn test_overlapping_hunks_are_rejected() {
        let d = diff("@@ -2,2 +2,2 @@\n-line2\n+a\n line3\n@@ -3,1 +3,1 @@\n-line3\n+b\n");
        let err = UnifiedPatcher::apply_diff(SRC, &d).unwrap_err();
        assert!(matches!(err, CoreError::Patch(m) if m.contains("overlaps")));
    }

    #[test]
    fn test_multi_file_patch_is_rejected() {
        let d = "--- a/f.txt\n+++ b/f.txt\n@@ -1 +1 @@\n-line1\n+x\n--- a/g.txt\n+++ b/g.txt\n@@ -1 +1 @@\n-a\n+b\n";
        let err = UnifiedPatcher::apply_diff(SRC, d).unwrap_err();
        assert!(matches!(err, CoreError::Patch(m) if m.contains("Multi-file")));
    }

    #[test]
    fn test_creation_and_rename_headers_are_rejected() {
        let create = "--- /dev/null\n+++ b/f.txt\n@@ -0,0 +1 @@\n+x\n";
        assert!(UnifiedPatcher::apply_diff("", create).is_err());
        let rename = "--- a/f.txt\n+++ b/g.txt\n@@ -1 +1 @@\n-line1\n+x\n";
        assert!(UnifiedPatcher::apply_diff(SRC, rename).is_err());
    }

    #[test]
    fn test_unsafe_header_paths_are_security_violations() {
        for path in [
            "../../etc/passwd",
            "//evil/share/x",
            "a/b/CON/x",
            "f.txt:ads",
        ] {
            let d = format!("--- a/{path}\n+++ b/{path}\n@@ -1 +1 @@\n-line1\n+x\n");
            let err = UnifiedPatcher::apply_diff(SRC, &d).unwrap_err();
            assert!(
                matches!(err, CoreError::SecurityViolation(_)),
                "{path} should be a security violation, got {err:?}"
            );
        }
    }

    #[test]
    fn test_crlf_sources_keep_their_line_endings() {
        let src = "a\r\nb\r\nc\r\n";
        let d = diff("@@ -2 +2 @@\n-b\n+B\n");
        let out = UnifiedPatcher::apply_diff(src, &d).unwrap();
        assert_eq!(out.patched, "a\r\nB\r\nc\r\n");
    }

    #[test]
    fn test_crlf_diff_text_is_accepted() {
        let d = "--- a/f.txt\r\n+++ b/f.txt\r\n@@ -1 +1 @@\r\n-line1\r\n+x\r\n";
        let out = UnifiedPatcher::apply_diff(SRC, d).unwrap();
        assert!(out.patched.starts_with("x\nline2"));
    }

    #[test]
    fn test_missing_trailing_newline_is_preserved() {
        let src = "a\nb";
        let d = diff("@@ -2 +2 @@\n-b\n+c\n\\ No newline at end of file\n");
        let out = UnifiedPatcher::apply_diff(src, &d).unwrap();
        assert_eq!(out.patched, "a\nc");
    }

    #[test]
    fn test_backslash_header_paths_are_normalized() {
        let d = "--- a\\src\\f.txt\n+++ b\\src\\f.txt\n@@ -1 +1 @@\n-line1\n+x\n";
        let out = UnifiedPatcher::apply_diff(SRC, d).unwrap();
        assert_eq!(out.target_path, "src/f.txt");
    }

    #[test]
    fn test_error_messages_do_not_echo_long_content() {
        let long = "S".repeat(500);
        let err =
            UnifiedPatcher::apply_diff(SRC, &format!("--- a/f\n+++ b/f\n{long}\n")).unwrap_err();
        assert!(err.to_string().len() < 200, "message too long: {err}");
    }

    #[test]
    fn test_net_zero_patch_is_detected_as_noop() {
        let d = diff("@@ -2 +2 @@\n-line2\n+line2\n");
        let out = UnifiedPatcher::apply_diff(SRC, &d).unwrap();
        assert!(out.is_noop(SRC));
    }

    #[test]
    fn test_deleted_line_that_looks_like_a_header_is_not_misparsed() {
        let src = "-- comment\nkeep\n";
        let d = diff("@@ -1,2 +1 @@\n--- comment\n keep\n");
        let out = UnifiedPatcher::apply_diff(src, &d).unwrap();
        assert_eq!(out.patched, "keep\n");
    }
}
