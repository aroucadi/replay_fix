use crate::error::Result;
use crate::platform::Platform;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Stdio;

/// Configuration for Playwright trace collection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceConfig {
    pub enabled: bool,
    pub output_path: PathBuf,
    pub screenshots: bool,
    pub snapshots: bool,
    pub sources: bool,
}

impl Default for TraceConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            output_path: PathBuf::from(".replayfix/artifacts/playwright.trace.zip"),
            screenshots: true,
            snapshots: true,
            sources: true,
        }
    }
}

/// Outcome of attempting to launch the external Playwright trace viewer.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TraceLaunchOutcome {
    pub launched: bool,
    pub pid: Option<u32>,
    pub reason: String,
}

/// Check if running within a Continuous Integration (CI) environment.
#[must_use]
pub fn is_ci_environment() -> bool {
    const CI_VARS: &[&str] = &[
        "CI",
        "CONTINUOUS_INTEGRATION",
        "GITHUB_ACTIONS",
        "TRAVIS",
        "CIRCLECI",
        "GITLAB_CI",
        "TF_BUILD",
        "BUILDKITE",
    ];

    CI_VARS.iter().any(|var| {
        std::env::var(var)
            .map(|val| val == "true" || val == "1")
            .unwrap_or(false)
    })
}

/// Launch the Playwright trace viewer detached from the harness process.
///
/// Guarantees:
/// - Skips launch cleanly in CI or non-interactive environments to prevent hung runners.
/// - Spawns with detached stdio (null) so harness exit doesn't terminate or hang the GUI.
/// - Returns structured `TraceLaunchOutcome` without panicking.
pub fn launch_trace_viewer(
    trace_path: &Path,
    is_interactive: bool,
    is_ci: bool,
) -> Result<TraceLaunchOutcome> {
    if is_ci {
        return Ok(TraceLaunchOutcome {
            launched: false,
            pid: None,
            reason: "Trace viewer auto-launch skipped in CI environment".to_string(),
        });
    }

    if !is_interactive {
        return Ok(TraceLaunchOutcome {
            launched: false,
            pid: None,
            reason: "Trace viewer auto-launch skipped in non-interactive session".to_string(),
        });
    }

    if !trace_path.exists() {
        return Ok(TraceLaunchOutcome {
            launched: false,
            pid: None,
            reason: format!("Trace archive not found at '{}'", trace_path.display()),
        });
    }

    let mut cmd = Platform::create_command("npx");
    cmd.arg("playwright")
        .arg("show-trace")
        .arg(trace_path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    match cmd.spawn() {
        Ok(child) => Ok(TraceLaunchOutcome {
            launched: true,
            pid: Some(child.id()),
            reason: format!(
                "Launched Playwright Trace Viewer for '{}'",
                trace_path.display()
            ),
        }),
        Err(e) => Ok(TraceLaunchOutcome {
            launched: false,
            pid: None,
            reason: format!("Failed to spawn 'npx playwright show-trace': {}", e),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn trace_config_defaults() {
        let config = TraceConfig::default();
        assert!(!config.enabled);
        assert!(config.screenshots);
        assert!(config.snapshots);
    }

    #[test]
    fn skips_in_ci() {
        let temp_dir = std::env::temp_dir().join("replayfix_trace_test_ci");
        let _ = fs::create_dir_all(&temp_dir);
        let trace_file = temp_dir.join("test.trace.zip");
        let _ = fs::write(&trace_file, b"PK00mocktrace");

        let outcome = launch_trace_viewer(&trace_file, true, true).unwrap();
        assert!(!outcome.launched);
        assert!(outcome.reason.contains("skipped in CI"));

        let _ = fs::remove_file(&trace_file);
    }

    #[test]
    fn skips_in_non_interactive() {
        let temp_dir = std::env::temp_dir().join("replayfix_trace_test_noninter");
        let _ = fs::create_dir_all(&temp_dir);
        let trace_file = temp_dir.join("test.trace.zip");
        let _ = fs::write(&trace_file, b"PK00mocktrace");

        let outcome = launch_trace_viewer(&trace_file, false, false).unwrap();
        assert!(!outcome.launched);
        assert!(outcome.reason.contains("non-interactive session"));

        let _ = fs::remove_file(&trace_file);
    }

    #[test]
    fn reports_missing_trace_file() {
        let missing = PathBuf::from("does_not_exist_at_all.zip");
        let outcome = launch_trace_viewer(&missing, true, false).unwrap();
        assert!(!outcome.launched);
        assert!(outcome.reason.contains("Trace archive not found"));
    }
}
