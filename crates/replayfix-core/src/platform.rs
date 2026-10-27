use crate::error::{CoreError, Result};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OsFamily {
    Windows,
    MacOS,
    Linux,
    OtherUnix,
}

impl OsFamily {
    #[must_use]
    pub const fn is_windows(&self) -> bool {
        matches!(self, OsFamily::Windows)
    }

    #[must_use]
    pub const fn is_unix(&self) -> bool {
        !self.is_windows()
    }
}

/// An OS-assigned ephemeral localhost TCP port (`127.0.0.1:0`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EphemeralPort(u16);

impl EphemeralPort {
    /// Bind `127.0.0.1:0` to obtain a free port from the OS kernel.
    pub fn allocate() -> Result<Self> {
        let listener = TcpListener::bind(("127.0.0.1", 0))?;
        let port = listener.local_addr()?.port();
        if port == 0 {
            return Err(CoreError::Execution(
                "OS returned port 0 for ephemeral TCP listener".to_string(),
            ));
        }
        Ok(Self(port))
    }

    /// Underlying non-zero TCP port number.
    #[must_use]
    pub const fn get(self) -> u16 {
        self.0
    }
}

impl From<EphemeralPort> for u16 {
    fn from(port: EphemeralPort) -> Self {
        port.0
    }
}

impl fmt::Display for EphemeralPort {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Platform;

impl Platform {
    #[must_use]
    pub const fn os_family() -> OsFamily {
        if cfg!(windows) {
            OsFamily::Windows
        } else if cfg!(target_os = "macos") {
            OsFamily::MacOS
        } else if cfg!(target_os = "linux") {
            OsFamily::Linux
        } else {
            OsFamily::OtherUnix
        }
    }

    /// Run a command adapting for Windows vs Unix differences cleanly.
    ///
    /// On Windows, batch and shell scripts (.cmd, .bat) installed by npm/Node (such as npx.cmd,
    /// playwright.cmd, npm.cmd) are not Win32 PE .exe binaries and cannot be spawned directly by
    /// CreateProcessW without their .cmd extension.
    pub fn create_command(program: &str) -> Command {
        if cfg!(windows)
            && !program.ends_with(".exe")
            && !program.ends_with(".cmd")
            && !program.ends_with(".bat")
            && (program == "npx" || program == "npm" || program == "playwright")
        {
            return Command::new(format!("{program}.cmd"));
        }
        Command::new(program)
    }

    /// Resolve absolute path safely across Windows and Unix
    pub fn canonicalize_path(path: impl AsRef<Path>) -> Result<PathBuf> {
        let canonical = std::fs::canonicalize(path)?;
        Ok(canonical)
    }

    /// Terminate a child process and its descendants. Already-exited PIDs succeed idempotently.
    pub fn terminate_process_tree(pid: u32) -> Result<()> {
        if pid == 0 || pid == std::process::id() {
            return Err(CoreError::Execution(format!(
                "Refusing to terminate protected process ID {pid}"
            )));
        }

        if Self::os_family().is_windows() {
            let pid_arg = pid.to_string();
            let output = Command::new("taskkill")
                .args(["/F", "/T", "/PID", &pid_arg])
                .output()?;
            if output.status.success() {
                return Ok(());
            }
            let combined = format!(
                "{} {}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            )
            .to_ascii_lowercase();
            // Exit code 128 or "not found" indicates the process has already exited.
            if output.status.code() == Some(128)
                || combined.contains("not found")
                || combined.contains("no running instance")
            {
                return Ok(());
            }
            Err(CoreError::Execution(format!(
                "taskkill failed for PID {pid}: {}",
                combined.trim()
            )))
        } else {
            let pid_arg = pid.to_string();
            let _ = Command::new("pkill")
                .args(["-KILL", "-P", &pid_arg])
                .output();
            let output = Command::new("kill").args(["-KILL", &pid_arg]).output()?;
            if output.status.success() {
                return Ok(());
            }
            let stderr = String::from_utf8_lossy(&output.stderr).to_ascii_lowercase();
            if stderr.contains("no such process") || stderr.contains("not found") {
                return Ok(());
            }
            Err(CoreError::Execution(format!(
                "kill -KILL failed for PID {pid}: {}",
                stderr.trim()
            )))
        }
    }

    /// Platform-aware remediation suggestion for Git
    #[must_use]
    pub fn suggest_git_install() -> &'static str {
        match Self::os_family() {
            OsFamily::Windows => "Install Git from https://git-scm.com or run 'winget install Git.Git'",
            OsFamily::MacOS => "Install Git via Homebrew ('brew install git') or Xcode Command Line Tools ('xcode-select --install')",
            OsFamily::Linux => "Install Git via your package manager ('sudo apt install git' or 'sudo dnf install git')",
            OsFamily::OtherUnix => "Install Git from https://git-scm.com or your system package manager",
        }
    }

    /// Platform-aware remediation suggestion for Node.js
    #[must_use]
    pub fn suggest_node_install() -> &'static str {
        match Self::os_family() {
            OsFamily::Windows => "Install Node.js 18+ from https://nodejs.org or run 'winget install OpenJS.NodeJS'",
            OsFamily::MacOS => "Install Node.js 18+ via Homebrew ('brew install node') or nvm ('nvm install 20')",
            OsFamily::Linux => "Install Node.js 18+ via your package manager ('sudo apt install nodejs npm') or NodeSource/nvm",
            OsFamily::OtherUnix => "Install Node.js 18+ from https://nodejs.org",
        }
    }

    /// Platform-aware remediation suggestion for FFmpeg
    #[must_use]
    pub fn suggest_ffmpeg_install() -> &'static str {
        match Self::os_family() {
            OsFamily::Windows => "Install FFmpeg via 'winget install Gyan.FFmpeg' or your system package manager",
            OsFamily::MacOS => "Install FFmpeg via Homebrew ('brew install ffmpeg')",
            OsFamily::Linux => "Install FFmpeg via your package manager ('sudo apt install ffmpeg' or 'sudo dnf install ffmpeg')",
            OsFamily::OtherUnix => "Install FFmpeg from https://ffmpeg.org or your system package manager",
        }
    }

    /// Cross-platform remediation suggestion for Playwright
    #[must_use]
    pub const fn suggest_playwright_install() -> &'static str {
        "Install Playwright via 'npm install -D playwright' and install browsers via 'npx playwright install chromium'"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Stdio;

    #[test]
    fn test_platform_os_family() {
        if cfg!(windows) {
            assert_eq!(Platform::os_family(), OsFamily::Windows);
            assert!(Platform::os_family().is_windows());
            assert!(!Platform::os_family().is_unix());
        } else if cfg!(target_os = "macos") {
            assert_eq!(Platform::os_family(), OsFamily::MacOS);
            assert!(Platform::os_family().is_unix());
            assert!(!Platform::os_family().is_windows());
        } else if cfg!(target_os = "linux") {
            assert_eq!(Platform::os_family(), OsFamily::Linux);
            assert!(Platform::os_family().is_unix());
            assert!(!Platform::os_family().is_windows());
        } else {
            assert_eq!(Platform::os_family(), OsFamily::OtherUnix);
            assert!(Platform::os_family().is_unix());
            assert!(!Platform::os_family().is_windows());
        }
    }

    #[test]
    fn test_platform_remediation_suggestions() {
        assert!(!Platform::suggest_git_install().is_empty());
        assert!(!Platform::suggest_node_install().is_empty());
        assert!(!Platform::suggest_ffmpeg_install().is_empty());
        assert!(!Platform::suggest_playwright_install().is_empty());
    }

    #[test]
    fn test_ephemeral_port_allocation() {
        let p1 = EphemeralPort::allocate().unwrap();
        let p2 = EphemeralPort::allocate().unwrap();
        assert!(p1.get() > 0);
        assert!(u16::from(p2) > 0);
        assert_eq!(p1.to_string(), p1.get().to_string());
    }

    #[test]
    fn test_terminate_process_tree_rejects_zero_and_self() {
        assert!(Platform::terminate_process_tree(0).is_err());
        assert!(Platform::terminate_process_tree(std::process::id()).is_err());
    }

    #[test]
    fn test_terminate_process_tree_kills_spawned_child_and_is_idempotent() {
        let mut child = if Platform::os_family().is_windows() {
            Command::new("ping")
                .args(["-n", "30", "127.0.0.1"])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap()
        } else {
            Command::new("sleep")
                .arg("30")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap()
        };

        let pid = child.id();
        Platform::terminate_process_tree(pid).unwrap();
        let _ = child.wait();
        // Calling terminate a second time on the already-exited PID must succeed idempotently.
        Platform::terminate_process_tree(pid).unwrap();
    }
}
