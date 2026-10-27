use crate::error::Result;
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

    /// Run a command adapting for Windows vs Unix differences cleanly
    pub fn create_command(program: &str) -> Command {
        Command::new(program)
    }

    /// Resolve absolute path safely across Windows and Unix
    pub fn canonicalize_path(path: impl AsRef<Path>) -> Result<PathBuf> {
        let canonical = std::fs::canonicalize(path)?;
        Ok(canonical)
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
}
