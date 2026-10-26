use crate::error::{CoreError, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

static ID_REGEX: OnceLock<Regex> = OnceLock::new();
static REF_REGEX: OnceLock<Regex> = OnceLock::new();

fn get_id_regex() -> &'static Regex {
    ID_REGEX.get_or_init(|| Regex::new(r"^[a-zA-Z0-9_-]+$").expect("valid id regex"))
}

fn get_ref_regex() -> &'static Regex {
    REF_REGEX.get_or_init(|| Regex::new(r"^[a-zA-Z0-9_\-\./]+$").expect("valid ref regex"))
}

fn default_port_env_var() -> String {
    "PORT".to_string()
}

fn default_healthcheck_path() -> String {
    "/api/health".to_string()
}

fn default_startup_timeout_ms() -> u64 {
    10000
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnvironmentAdapter {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_cmd: Option<String>,

    #[serde(default = "default_port_env_var")]
    pub port_env_var: String,

    #[serde(default = "default_healthcheck_path")]
    pub healthcheck_path: String,

    #[serde(default = "default_startup_timeout_ms")]
    pub startup_timeout_ms: u64,
}

impl Default for EnvironmentAdapter {
    fn default() -> Self {
        Self {
            server_cmd: None,
            port_env_var: default_port_env_var(),
            healthcheck_path: default_healthcheck_path(),
            startup_timeout_ms: default_startup_timeout_ms(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskManifest {
    pub task_id: String,
    pub title: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,

    pub repo_path: String,
    pub bug_ref: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub clean_ref: Option<String>,

    pub video_path: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_path: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_behavior: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment_adapter: Option<EnvironmentAdapter>,

    #[serde(default)]
    pub headed: bool,
}

impl TaskManifest {
    /// Load and validate a task manifest from a JSON file
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        if !path.exists() {
            return Err(CoreError::Manifest(format!(
                "Manifest file not found: {}",
                path.display()
            )));
        }

        let content = std::fs::read_to_string(path)?;
        let manifest: TaskManifest = serde_json::from_str(&content)
            .map_err(|e| CoreError::Manifest(format!("Invalid manifest JSON schema: {}", e)))?;

        manifest.validate()?;
        Ok(manifest)
    }

    /// Validate the manifest according to canonical task_manifest schema rules
    pub fn validate(&self) -> Result<()> {
        if !get_id_regex().is_match(&self.task_id) {
            return Err(CoreError::Manifest(format!(
                "Invalid task_id '{}': must match ^[a-zA-Z0-9_-]+$",
                self.task_id
            )));
        }

        if self.title.trim().is_empty() {
            return Err(CoreError::Manifest("Manifest title cannot be empty".to_string()));
        }

        if !get_ref_regex().is_match(&self.bug_ref) {
            return Err(CoreError::Manifest(format!(
                "Invalid bug_ref '{}': must match ^[a-zA-Z0-9_\\-\\./]+$",
                self.bug_ref
            )));
        }

        if let Some(clean_ref) = &self.clean_ref {
            if !get_ref_regex().is_match(clean_ref) {
                return Err(CoreError::Manifest(format!(
                    "Invalid clean_ref '{}': must match ^[a-zA-Z0-9_\\-\\./]+$",
                    clean_ref
                )));
            }
        }

        // Security check on paths: reject directory traversal, UNC, DOS devices, and null bytes
        Self::assert_safe_path(&self.repo_path, "repo_path")?;
        Self::assert_safe_path(&self.video_path, "video_path")?;

        if let Some(audio) = &self.audio_path {
            Self::assert_safe_path(audio, "audio_path")?;
        }

        Ok(())
    }

    /// Check for illegal directory traversal, UNC network injection, DOS device names, or null bytes
    pub fn assert_safe_path(path_str: &str, field_name: &str) -> Result<PathBuf> {
        if path_str.contains('\0') {
            return Err(CoreError::SecurityViolation(format!(
                "Null byte detected in {} path",
                field_name
            )));
        }

        // Disallow UNC network paths (SMB credential coercion defense: \\evil\share or //evil/share)
        if path_str.starts_with(r"\\") || path_str.starts_with("//") {
            return Err(CoreError::SecurityViolation(format!(
                "UNC network paths are prohibited in {}: potential credential coercion",
                field_name
            )));
        }

        let path = Path::new(path_str);

        // Check for reserved DOS device names (CON, PRN, AUX, NUL, COM1-9, LPT1-9)
        if let Some(file_name) = path.file_stem().and_then(|s| s.to_str()) {
            let upper = file_name.to_ascii_uppercase();
            let is_dos_device = matches!(
                upper.as_str(),
                "CON" | "PRN" | "AUX" | "NUL"
                    | "COM1" | "COM2" | "COM3" | "COM4" | "COM5" | "COM6" | "COM7" | "COM8" | "COM9"
                    | "LPT1" | "LPT2" | "LPT3" | "LPT4" | "LPT5" | "LPT6" | "LPT7" | "LPT8" | "LPT9"
            );
            if is_dos_device {
                return Err(CoreError::SecurityViolation(format!(
                    "Reserved DOS device name prohibited in {}: '{}'",
                    field_name, upper
                )));
            }
        }

        // Disallow '..' components that escape root boundary
        let mut depth: isize = 0;
        for component in path.components() {
            match component {
                std::path::Component::ParentDir => {
                    depth -= 1;
                    if depth < 0 {
                        return Err(CoreError::SecurityViolation(format!(
                            "Path traversal detected in {}: escapes boundary",
                            field_name
                        )));
                    }
                }
                std::path::Component::Normal(_) => {
                    depth += 1;
                }
                _ => {}
            }
        }

        Ok(path.to_path_buf())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_manifest_deserialization() {
        let json = r#"{
            "task_id": "RF-01",
            "title": "Quote Repost Crash",
            "repo_path": "./MetaPulse",
            "bug_ref": "bug/RF-01",
            "clean_ref": "main",
            "video_path": "recordings/RF-01.mp4",
            "headed": true,
            "environment_adapter": {
                "server_cmd": "npm run dev",
                "port_env_var": "PORT",
                "healthcheck_path": "/api/health",
                "startup_timeout_ms": 15000
            }
        }"#;

        let manifest: TaskManifest = serde_json::from_str(json).unwrap();
        assert_eq!(manifest.task_id, "RF-01");
        assert_eq!(manifest.headed, true);
        assert!(manifest.validate().is_ok());

        let adapter = manifest.environment_adapter.unwrap();
        assert_eq!(adapter.startup_timeout_ms, 15000);
        assert_eq!(adapter.server_cmd.as_deref(), Some("npm run dev"));
    }

    #[test]
    fn test_manifest_validation_rejects_invalid_id() {
        let manifest = TaskManifest {
            task_id: "RF/01 with spaces".to_string(),
            title: "Test".to_string(),
            category: None,
            repo_path: ".".to_string(),
            bug_ref: "bug/RF-01".to_string(),
            clean_ref: None,
            video_path: "test.mp4".to_string(),
            audio_path: None,
            expected_behavior: None,
            environment_adapter: None,
            headed: false,
        };

        assert!(manifest.validate().is_err());
    }

    #[test]
    fn test_security_violation_on_path_traversal() {
        let manifest = TaskManifest {
            task_id: "RF-01".to_string(),
            title: "Test".to_string(),
            category: None,
            repo_path: "../../../etc/passwd".to_string(),
            bug_ref: "bug/RF-01".to_string(),
            clean_ref: None,
            video_path: "test.mp4".to_string(),
            audio_path: None,
            expected_behavior: None,
            environment_adapter: None,
            headed: false,
        };

        match manifest.validate() {
            Err(CoreError::SecurityViolation(msg)) => {
                assert!(msg.contains("Path traversal detected"));
            }
            res => panic!("Expected SecurityViolation, got {:?}", res),
        }
    }

    #[test]
    fn test_security_violation_on_unc_path() {
        let manifest = TaskManifest {
            task_id: "RF-01".to_string(),
            title: "Test".to_string(),
            category: None,
            repo_path: r"\\attacker.com\share\repo".to_string(),
            bug_ref: "bug/RF-01".to_string(),
            clean_ref: None,
            video_path: "test.mp4".to_string(),
            audio_path: None,
            expected_behavior: None,
            environment_adapter: None,
            headed: false,
        };

        match manifest.validate() {
            Err(CoreError::SecurityViolation(msg)) => {
                assert!(msg.contains("UNC network paths are prohibited"));
            }
            res => panic!("Expected SecurityViolation, got {:?}", res),
        }
    }

    #[test]
    fn test_security_violation_on_dos_device() {
        let manifest = TaskManifest {
            task_id: "RF-01".to_string(),
            title: "Test".to_string(),
            category: None,
            repo_path: "NUL".to_string(),
            bug_ref: "bug/RF-01".to_string(),
            clean_ref: None,
            video_path: "test.mp4".to_string(),
            audio_path: None,
            expected_behavior: None,
            environment_adapter: None,
            headed: false,
        };

        match manifest.validate() {
            Err(CoreError::SecurityViolation(msg)) => {
                assert!(msg.contains("Reserved DOS device name prohibited"));
            }
            res => panic!("Expected SecurityViolation, got {:?}", res),
        }
    }
}
