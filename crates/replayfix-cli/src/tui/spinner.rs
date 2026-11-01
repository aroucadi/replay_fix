//! Animated Progress Spinner & Status Line for Active Operations.

use crate::ui::{Symbols, Theme};
use std::io::{stdout, Write};
use std::time::Instant;

const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// Interactive in-place status line with animated spinner and timer.
#[derive(Debug)]
pub struct StatusSpinner {
    phase: String,
    detail: String,
    start_time: Instant,
    frame_index: usize,
    active: bool,
}

impl StatusSpinner {
    #[must_use]
    pub fn new(phase: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            phase: phase.into(),
            detail: detail.into(),
            start_time: Instant::now(),
            frame_index: 0,
            active: true,
        }
    }

    /// Advance spinner animation frame and re-render line.
    pub fn tick(&mut self, theme: &Theme) {
        if !self.active || !theme.color_enabled {
            return;
        }

        let frame = SPINNER_FRAMES[self.frame_index % SPINNER_FRAMES.len()];
        self.frame_index = self.frame_index.wrapping_add(1);
        let elapsed = self.start_time.elapsed().as_secs();

        // Print in-place using carriage return
        print!(
            "\r  {} {: <14} {} {} ({:02}:{:02}s)\x1b[K",
            theme.info(frame),
            theme.bold(&self.phase),
            theme.muted("›"),
            self.detail,
            elapsed / 60,
            elapsed % 60
        );
        let _ = stdout().flush();
    }

    /// Update current phase and detail.
    pub fn set_status(&mut self, phase: impl Into<String>, detail: impl Into<String>) {
        self.phase = phase.into();
        self.detail = detail.into();
    }

    /// Complete with success mark.
    pub fn complete(&mut self, theme: &Theme, summary: &str) {
        if !self.active {
            return;
        }
        self.active = false;
        let elapsed = self.start_time.elapsed().as_millis();
        println!(
            "\r  {} {: <14} [{}] {} ({}ms)\x1b[K",
            theme.success(Symbols::CHECK),
            theme.bold(&self.phase),
            theme.success("PASS"),
            summary,
            elapsed
        );
        let _ = stdout().flush();
    }

    /// Complete with failure mark.
    pub fn fail(&mut self, theme: &Theme, reason: &str) {
        if !self.active {
            return;
        }
        self.active = false;
        let elapsed = self.start_time.elapsed().as_millis();
        println!(
            "\r  {} {: <14} [{}] {} ({}ms)\x1b[K",
            theme.error(Symbols::CROSS),
            theme.error(&self.phase),
            theme.error("FAIL"),
            reason,
            elapsed
        );
        let _ = stdout().flush();
    }
}
