use crate::platform::Platform;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

static VERSION_RE: OnceLock<Regex> = OnceLock::new();
static GIT_RE: OnceLock<Regex> = OnceLock::new();
static NODE_RE: OnceLock<Regex> = OnceLock::new();
static PLAYWRIGHT_RE: OnceLock<Regex> = OnceLock::new();
static FFMPEG_RE: OnceLock<Regex> = OnceLock::new();
static FFMPEG_FALLBACK_RE: OnceLock<Regex> = OnceLock::new();

fn get_version_re() -> &'static Regex {
    VERSION_RE.get_or_init(|| Regex::new(r"(\d+)(?:\.(\d+))?(?:\.(\d+))?").expect("valid regex"))
}

fn get_git_re() -> &'static Regex {
    GIT_RE.get_or_init(|| Regex::new(r"git version (\d+\.\d+(?:\.\d+)?)").expect("valid regex"))
}

fn get_node_re() -> &'static Regex {
    NODE_RE.get_or_init(|| Regex::new(r"v?(\d+\.\d+(?:\.\d+)?)").expect("valid regex"))
}

fn get_playwright_re() -> &'static Regex {
    PLAYWRIGHT_RE.get_or_init(|| Regex::new(r"(\d+\.\d+(?:\.\d+)?)").expect("valid regex"))
}

fn get_ffmpeg_re() -> &'static Regex {
    FFMPEG_RE
        .get_or_init(|| Regex::new(r"ffmpeg version n?(\d+\.\d+(?:\.\d+)?)").expect("valid regex"))
}

fn get_ffmpeg_fallback_re() -> &'static Regex {
    FFMPEG_FALLBACK_RE.get_or_init(|| Regex::new(r"(\d+\.\d+)").expect("valid regex"))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DependencyStatus {
    Compatible,
    IncompatibleVersion { found: String, required: String },
    NotFound,
    PermissionDenied,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyCheck {
    pub name: String,
    pub executable: String,
    pub found_version: Option<String>,
    pub required_version: String,
    pub status: DependencyStatus,
    pub remediation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoctorReport {
    pub overall_status: bool,
    pub host_os: String,
    pub checks: Vec<DependencyCheck>,
    pub timestamp: String,
}

pub struct Doctor;

impl Doctor {
    /// Parse a semantic version tuple (major, minor, patch) from string
    pub fn parse_version_tuple(version_str: &str) -> Option<(u64, u64, u64)> {
        let caps = get_version_re().captures(version_str)?;

        let major = caps.get(1)?.as_str().parse::<u64>().ok()?;
        let minor = caps
            .get(2)
            .map_or(0, |m| m.as_str().parse::<u64>().unwrap_or(0));
        let patch = caps
            .get(3)
            .map_or(0, |m| m.as_str().parse::<u64>().unwrap_or(0));

        Some((major, minor, patch))
    }

    /// Compare if found version meets or exceeds required version
    pub fn is_compatible(found: &str, required: &str) -> bool {
        match (
            Self::parse_version_tuple(found),
            Self::parse_version_tuple(required),
        ) {
            (Some(f), Some(r)) => f >= r,
            _ => false,
        }
    }

    /// Probe a tool command and return its stdout
    fn run_version_command(cmd_name: &str, args: &[&str]) -> Result<String, DependencyStatus> {
        let mut cmd = Platform::create_command(cmd_name);
        cmd.args(args);

        match cmd.output() {
            Ok(output) => {
                if output.status.success() {
                    let text = String::from_utf8_lossy(&output.stdout).to_string();
                    if text.trim().is_empty() {
                        Ok(String::from_utf8_lossy(&output.stderr).to_string())
                    } else {
                        Ok(text)
                    }
                } else {
                    let err_text = String::from_utf8_lossy(&output.stderr).to_string();
                    if err_text.contains("Permission denied") {
                        Err(DependencyStatus::PermissionDenied)
                    } else {
                        Err(DependencyStatus::NotFound)
                    }
                }
            }
            Err(e) => {
                if e.kind() == std::io::ErrorKind::PermissionDenied {
                    Err(DependencyStatus::PermissionDenied)
                } else {
                    Err(DependencyStatus::NotFound)
                }
            }
        }
    }

    /// Check Git presence and version
    pub fn check_git() -> DependencyCheck {
        let name = "Git".to_string();
        let executable = "git".to_string();
        let required_version = "2.30.0".to_string();
        let remediation = Some(
            "Install Git from https://git-scm.com or run 'winget install Git.Git'".to_string(),
        );

        match Self::run_version_command("git", &["--version"]) {
            Ok(out) => {
                let found = get_git_re()
                    .captures(&out)
                    .and_then(|c| c.get(1))
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_else(|| out.trim().to_string());

                if Self::is_compatible(&found, &required_version) {
                    DependencyCheck {
                        name,
                        executable,
                        found_version: Some(found),
                        required_version,
                        status: DependencyStatus::Compatible,
                        remediation: None,
                    }
                } else {
                    DependencyCheck {
                        name,
                        executable,
                        found_version: Some(found.clone()),
                        required_version: required_version.clone(),
                        status: DependencyStatus::IncompatibleVersion {
                            found,
                            required: required_version,
                        },
                        remediation,
                    }
                }
            }
            Err(status) => DependencyCheck {
                name,
                executable,
                found_version: None,
                required_version,
                status,
                remediation,
            },
        }
    }

    /// Check Node.js presence and version
    pub fn check_node() -> DependencyCheck {
        let name = "Node.js".to_string();
        let executable = "node".to_string();
        let required_version = "18.0.0".to_string();
        let remediation = Some(
            "Install Node.js 18+ from https://nodejs.org or run 'winget install OpenJS.NodeJS'"
                .to_string(),
        );

        match Self::run_version_command("node", &["--version"]) {
            Ok(out) => {
                let found = get_node_re()
                    .captures(&out)
                    .and_then(|c| c.get(1))
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_else(|| out.trim().to_string());

                if Self::is_compatible(&found, &required_version) {
                    DependencyCheck {
                        name,
                        executable,
                        found_version: Some(found),
                        required_version,
                        status: DependencyStatus::Compatible,
                        remediation: None,
                    }
                } else {
                    DependencyCheck {
                        name,
                        executable,
                        found_version: Some(found.clone()),
                        required_version: required_version.clone(),
                        status: DependencyStatus::IncompatibleVersion {
                            found,
                            required: required_version,
                        },
                        remediation,
                    }
                }
            }
            Err(status) => DependencyCheck {
                name,
                executable,
                found_version: None,
                required_version,
                status,
                remediation,
            },
        }
    }

    /// Check Playwright presence and version
    pub fn check_playwright() -> DependencyCheck {
        let name = "Playwright".to_string();
        let executable = "playwright".to_string();
        let required_version = "1.40.0".to_string();
        let remediation = Some("Install Playwright via 'npm install -D playwright' and install browsers via 'npx playwright install chromium'".to_string());

        // Probe directly via npx or playwright CLI
        let probe = Self::run_version_command("npx", &["--no-install", "playwright", "--version"])
            .or_else(|_| Self::run_version_command("playwright", &["--version"]));

        match probe {
            Ok(out) => {
                let found = get_playwright_re()
                    .captures(&out)
                    .and_then(|c| c.get(1))
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_else(|| out.trim().to_string());

                if Self::is_compatible(&found, &required_version) {
                    DependencyCheck {
                        name,
                        executable,
                        found_version: Some(found),
                        required_version,
                        status: DependencyStatus::Compatible,
                        remediation: None,
                    }
                } else {
                    DependencyCheck {
                        name,
                        executable,
                        found_version: Some(found.clone()),
                        required_version: required_version.clone(),
                        status: DependencyStatus::IncompatibleVersion {
                            found,
                            required: required_version,
                        },
                        remediation,
                    }
                }
            }
            Err(status) => DependencyCheck {
                name,
                executable,
                found_version: None,
                required_version,
                status,
                remediation,
            },
        }
    }

    /// Check FFmpeg presence and version
    pub fn check_ffmpeg() -> DependencyCheck {
        let name = "FFmpeg".to_string();
        let executable = "ffmpeg".to_string();
        let required_version = "4.4.0".to_string();
        let remediation = Some(
            "Install FFmpeg via 'winget install Gyan.FFmpeg' or your system package manager"
                .to_string(),
        );

        match Self::run_version_command("ffmpeg", &["-version"]) {
            Ok(out) => {
                let found = get_ffmpeg_re()
                    .captures(&out)
                    .and_then(|c| c.get(1))
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_else(|| {
                        get_ffmpeg_fallback_re()
                            .captures(&out)
                            .and_then(|c| c.get(1))
                            .map(|m| m.as_str().to_string())
                            .unwrap_or_else(|| out.trim().to_string())
                    });

                if Self::is_compatible(&found, &required_version) {
                    DependencyCheck {
                        name,
                        executable,
                        found_version: Some(found),
                        required_version,
                        status: DependencyStatus::Compatible,
                        remediation: None,
                    }
                } else {
                    DependencyCheck {
                        name,
                        executable,
                        found_version: Some(found.clone()),
                        required_version: required_version.clone(),
                        status: DependencyStatus::IncompatibleVersion {
                            found,
                            required: required_version,
                        },
                        remediation,
                    }
                }
            }
            Err(status) => DependencyCheck {
                name,
                executable,
                found_version: None,
                required_version,
                status,
                remediation,
            },
        }
    }

    /// Run full diagnostic check across all dependencies
    pub fn check_environment() -> DoctorReport {
        let checks = vec![
            Self::check_git(),
            Self::check_node(),
            Self::check_playwright(),
            Self::check_ffmpeg(),
        ];

        let overall_status = checks
            .iter()
            .all(|c| c.status == DependencyStatus::Compatible);

        DoctorReport {
            overall_status,
            host_os: format!("{:?}", Platform::os_family()),
            checks,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_tuple_parsing() {
        assert_eq!(Doctor::parse_version_tuple("2.55.0"), Some((2, 55, 0)));
        assert_eq!(Doctor::parse_version_tuple("18.2.1"), Some((18, 2, 1)));
        assert_eq!(Doctor::parse_version_tuple("1.45"), Some((1, 45, 0)));
        assert_eq!(Doctor::parse_version_tuple("v24.19.0"), Some((24, 19, 0)));
    }

    #[test]
    fn test_version_compatibility() {
        assert!(Doctor::is_compatible("2.55.0", "2.30.0"));
        assert!(Doctor::is_compatible("18.0.0", "18.0.0"));
        assert!(!Doctor::is_compatible("16.14.0", "18.0.0"));
        assert!(Doctor::is_compatible("7.1", "4.4.0"));
    }

    #[test]
    fn test_doctor_check_structure() {
        let report = Doctor::check_environment();
        assert_eq!(report.checks.len(), 4);
        assert!(report.checks.iter().any(|c| c.name == "Git"));
        assert!(report.checks.iter().any(|c| c.name == "Node.js"));
        assert!(report.checks.iter().any(|c| c.name == "Playwright"));
        assert!(report.checks.iter().any(|c| c.name == "FFmpeg"));
    }
}
