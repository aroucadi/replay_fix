//! Animated 60 FPS Progress Spinner, Wave Shimmer & Gate Flash Status Line (`tui/spinner.rs`).

use crate::tui::motion::{
    render_fractional_bar, render_wave_shimmer, GateFlashAnimation, MotionEngine,
};
use crate::ui::{CyberPalette, StepStatus, Symbols, Theme};
use std::io::{stdout, IsTerminal, Write};
use std::time::Instant;

const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// Interactive in-place status line powered by the 60 FPS Living Motion Engine.
#[derive(Debug)]
pub struct StatusSpinner {
    phase: String,
    detail: String,
    start_time: Instant,
    frame_index: usize,
    motion: MotionEngine,
    progress: f64,
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
            motion: MotionEngine::new(),
            progress: 0.15,
            active: true,
        }
    }

    /// Advance spinner animation frame at 60 FPS with RGB wave shimmer and fractional bar.
    pub fn tick(&mut self, theme: &Theme) {
        if !self.active || !theme.color_enabled || !stdout().is_terminal() {
            return;
        }

        let tick = self.motion.step_frame();
        let frame = SPINNER_FRAMES[self.frame_index % SPINNER_FRAMES.len()];
        self.frame_index = self.frame_index.wrapping_add(1);
        self.progress = (self.progress + 0.025).fract();
        let elapsed = self.start_time.elapsed().as_secs();

        let bar = render_fractional_bar(self.progress, 6, theme.color_enabled);
        let shimmer_detail = render_wave_shimmer(
            &self.detail,
            tick,
            CyberPalette::MUTED_STEEL,
            CyberPalette::CYAN,
            theme.color_enabled,
        );

        print!(
            "\x1b[1G  {} {: <14} {} {} {} ({:02}:{:02}s)\x1b[K",
            theme.info(frame),
            theme.bold(&self.phase),
            bar,
            theme.muted(Symbols::STEP),
            shimmer_detail,
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

    /// Complete with an animated emerald flash transition settling into the passed state.
    pub fn complete(&mut self, theme: &Theme, summary: &str) {
        if !self.active {
            return;
        }
        self.active = false;
        let elapsed = self.start_time.elapsed().as_millis();

        if theme.color_enabled && stdout().is_terminal() {
            let flash = GateFlashAnimation::new(
                self.phase.clone(),
                format!("{summary} ({elapsed}ms)"),
            );
            print!("\x1b[1G{}\x1b[K\x1b[1G", flash.render_row(true));
            let _ = stdout().flush();
        }

        println!(
            "\x1b[1G  {} {: <14} {} {} ({}ms)\x1b[K",
            theme.success(Symbols::CHECK),
            theme.bold(&self.phase),
            theme.status_pill(StepStatus::Done),
            summary,
            elapsed
        );
        let _ = stdout().flush();
    }

    /// Complete with failure pill badge.
    pub fn fail(&mut self, theme: &Theme, reason: &str) {
        if !self.active {
            return;
        }
        self.active = false;
        let elapsed = self.start_time.elapsed().as_millis();
        println!(
            "\x1b[1G  {} {: <14} {} {} ({}ms)\x1b[K",
            theme.error(Symbols::CROSS),
            theme.error(&self.phase),
            theme.status_pill(StepStatus::Fail),
            reason,
            elapsed
        );
        let _ = stdout().flush();
    }
}
