use crate::error::Result;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OsFamily {
    Windows,
    Unix,
}

pub struct Platform;

impl Platform {
    #[must_use]
    pub const fn os_family() -> OsFamily {
        if cfg!(windows) {
            OsFamily::Windows
        } else {
            OsFamily::Unix
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_os_family() {
        if cfg!(windows) {
            assert_eq!(Platform::os_family(), OsFamily::Windows);
        } else {
            assert_eq!(Platform::os_family(), OsFamily::Unix);
        }
    }
}
