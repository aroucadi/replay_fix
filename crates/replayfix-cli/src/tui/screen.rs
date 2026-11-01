//! Terminal Screen & Raw Mode Guard.
//!
//! Enforces RAII cleanup: disables raw mode, shows cursor, and restores
//! terminal state cleanly even on early return, cancellation, or panic.

use crossterm::cursor::{Hide, Show};
use crossterm::event::{self, Event, KeyEvent};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::QueueableCommand;
use std::io::{self, stdout, Write};
use std::time::Duration;

/// Safe RAII guard for raw terminal execution.
#[derive(Debug)]
pub struct ScreenGuard {
    in_alternate_screen: bool,
    raw_mode_enabled: bool,
}

impl ScreenGuard {
    /// Initialize raw mode without switching to alternate screen (in-place interactive mode).
    pub fn enter_raw() -> io::Result<Self> {
        enable_raw_mode()?;
        let mut out = stdout();
        out.queue(Hide)?;
        out.flush()?;
        Ok(Self {
            in_alternate_screen: false,
            raw_mode_enabled: true,
        })
    }

    /// Initialize full alternate screen with raw mode (for full-screen interactive views like diff review).
    pub fn enter_alternate() -> io::Result<Self> {
        enable_raw_mode()?;
        let mut out = stdout();
        out.queue(EnterAlternateScreen)?.queue(Hide)?;
        out.flush()?;
        Ok(Self {
            in_alternate_screen: true,
            raw_mode_enabled: true,
        })
    }

    /// Check if terminal size is at least (cols, rows).
    #[must_use]
    pub fn get_size() -> (u16, u16) {
        crossterm::terminal::size().unwrap_or((80, 24))
    }

    /// Non-blocking poll for keyboard or resize events.
    pub fn poll_key(timeout: Duration) -> io::Result<Option<KeyEvent>> {
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                return Ok(Some(key));
            }
        }
        Ok(None)
    }

    /// Read next key event, blocking until input is available.
    pub fn read_key() -> io::Result<KeyEvent> {
        loop {
            if let Event::Key(key) = event::read()? {
                return Ok(key);
            }
        }
    }
}

impl Drop for ScreenGuard {
    fn drop(&mut self) {
        let mut out = stdout();
        let _ = out.queue(Show);
        if self.in_alternate_screen {
            let _ = out.queue(LeaveAlternateScreen);
        }
        let _ = out.flush();
        if self.raw_mode_enabled {
            let _ = disable_raw_mode();
        }
    }
}
