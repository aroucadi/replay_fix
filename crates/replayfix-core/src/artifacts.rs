use crate::digest::canonical_json;
use crate::error::{CoreError, Result};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

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
        let target_path = self.root_dir.join(file_name);
        let temp_path = self.root_dir.join(format!("{}.tmp", file_name));

        fs::write(&temp_path, content.as_bytes())?;
        fs::rename(&temp_path, &target_path)?;

        Ok(target_path)
    }

    /// Write a serializable value as formatted, canonical JSON atomically.
    pub fn write_json<T: Serialize>(&self, file_name: &str, value: &T) -> Result<PathBuf> {
        let json_val = serde_json::to_value(value)?;
        let json_str = canonical_json(&json_val)?;
        let target_path = self.root_dir.join(file_name);
        let temp_path = self.root_dir.join(format!("{}.tmp", file_name));

        fs::write(&temp_path, json_str.as_bytes())?;
        fs::rename(&temp_path, &target_path)?;

        Ok(target_path)
    }

    /// Write binary data atomically (e.g. `playwright.trace.zip`).
    pub fn write_bytes(&self, file_name: &str, data: &[u8]) -> Result<PathBuf> {
        let target_path = self.root_dir.join(file_name);
        let temp_path = self.root_dir.join(format!("{}.tmp", file_name));

        fs::write(&temp_path, data)?;
        fs::rename(&temp_path, &target_path)?;

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

        let json_data = serde_json::json!({ "status": "VERIFIED" });
        let json_path = store.write_json("status.json", &json_data).unwrap();
        assert!(json_path.exists());

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
