//! 60 FPS Sub-Character Living Motion Engine (`tui/motion.rs`).
//!
//! Provides high-frequency, non-blocking visual dynamics for the ReplayFix
//! Living Cybernetic Command Deck:
//! - Fractional progress bars using 8-level Unicode blocks (`▏▎▍▌▋▊▉█`).
//! - Traveling 24-bit RGB sine-wave shimmer across active deliberation and tool strings.
//! - Breathing 1.5s sinusoidal luminance pulse on the `❯` prompt character.
//! - Animated emerald row flash transitions when verification gates pass.

use crate::ui::{palette, style, Rgb, Symbols};
use std::f64::consts::TAU;
use std::time::Duration;

/// Target frame cadence for the 60 FPS motion loop (~16.67ms).
pub const FRAME_DURATION: Duration = Duration::from_millis(16);

/// Number of frames in one 1.5-second breathing cycle at 60 FPS (`1.5 * 60 = 90`).
pub const BREATHING_PERIOD_FRAMES: u64 = 90;

/// Number of frames for a gate pass emerald flash transition to decay (`0.5s` at 60 FPS).
pub const GATE_FLASH_DURATION_FRAMES: u64 = 30;

/// 8-level sub-character horizontal block elements from 1/8th (`▏`) to full (`█`).
pub const FRACTIONAL_BLOCKS: [char; 8] = ['▏', '▎', '▍', '▌', '▋', '▊', '▉', '█'];

/// Gate transition flash variant (Gate 1 RED confirmation vs. Gate 2 GREEN verification).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateFlashKind {
    Gate1RedConfirmed,
    Gate2GreenPassed,
}

/// 60 FPS deterministic tick counter and animation coordinator.
#[derive(Debug, Clone)]
pub struct MotionEngine {
    tick: u64,
    active_flash: Option<GateFlashAnimation>,
}

impl Default for MotionEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl MotionEngine {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            tick: 0,
            active_flash: None,
        }
    }

    /// Return current 60 FPS frame tick.
    #[must_use]
    pub const fn tick(&self) -> u64 {
        self.tick
    }

    /// Advance the motion clock by one 60 FPS frame and step any active transition flash.
    pub fn step_frame(&mut self) -> u64 {
        self.tick = self.tick.wrapping_add(1);
        if let Some(flash) = &mut self.active_flash {
            flash.step();
            if flash.is_finished() {
                self.active_flash = None;
            }
        }
        self.tick
    }

    /// Trigger a luminous crimson flash when Gate 1 RED confirms causal reproduction.
    pub fn trigger_gate1_red_flash(
        &mut self,
        gate_label: impl Into<String>,
        summary: impl Into<String>,
    ) {
        self.active_flash = Some(GateFlashAnimation::with_kind(
            GateFlashKind::Gate1RedConfirmed,
            gate_label.into(),
            summary.into(),
        ));
    }

    /// Trigger an emerald row flash animation when a verification gate passes.
    pub fn trigger_gate_pass_flash(
        &mut self,
        gate_label: impl Into<String>,
        summary: impl Into<String>,
    ) {
        self.active_flash = Some(GateFlashAnimation::new(gate_label.into(), summary.into()));
    }

    /// Return reference to the active gate flash animation, if any.
    #[must_use]
    pub const fn active_flash(&self) -> Option<&GateFlashAnimation> {
        self.active_flash.as_ref()
    }
}

/// Render a fractional progress bar using 8-level sub-character Unicode blocks (`▏▎▍▌▋▊▉█`).
///
/// `progress` is clamped to `[0.0, 1.0]`, providing `width_cells * 8` discrete sub-character steps.
#[must_use]
pub fn render_fractional_bar(progress: f64, width_cells: usize, color_enabled: bool) -> String {
    if width_cells == 0 {
        return String::new();
    }
    let clamped = progress.clamp(0.0, 1.0);
    let total_eighths_f = (clamped * (width_cells as f64) * 8.0).round();
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let total_eighths = (total_eighths_f as usize).min(width_cells.saturating_mul(8));

    let full_cells = total_eighths / 8;
    let remainder = total_eighths % 8;

    let mut bar = String::with_capacity(width_cells * 24);
    for cell_idx in 0..width_cells {
        let t = if width_cells > 1 {
            (cell_idx as f64) / ((width_cells - 1) as f64)
        } else {
            1.0
        };
        let fg = palette::CYBER_CYAN.lerp(palette::EMERALD_BRIGHT, t);

        if cell_idx < full_cells {
            if color_enabled {
                bar.push_str(&fg.fg_ansi());
            }
            bar.push(FRACTIONAL_BLOCKS[7]);
        } else if cell_idx == full_cells && remainder > 0 {
            if color_enabled {
                bar.push_str(&fg.fg_ansi());
            }
            bar.push(FRACTIONAL_BLOCKS[remainder - 1]);
        } else {
            if color_enabled {
                bar.push_str(&palette::DOCK_PILL_BG.fg_ansi());
            }
            bar.push('─');
        }
    }

    if color_enabled {
        bar.push_str(style::RESET);
    }
    bar
}

/// Render a traveling 24-bit RGB sine-wave shimmer across `text`.
///
/// As `tick` increments at 60 FPS, a bright luminance crest travels left-to-right across the text.
#[must_use]
pub fn render_wave_shimmer(
    text: &str,
    tick: u64,
    base: Rgb,
    peak: Rgb,
    color_enabled: bool,
) -> String {
    if !color_enabled || text.is_empty() {
        return text.to_string();
    }

    let mut out = String::with_capacity(text.len().saturating_mul(20));
    let time_phase = (tick as f64) * 0.22;

    for (idx, ch) in text.chars().enumerate() {
        if ch.is_whitespace() {
            out.push(ch);
            continue;
        }
        let spatial_phase = (idx as f64) * 0.35;
        let wave = ((time_phase - spatial_phase).sin() + 1.0) * 0.5;
        // Sharpen the crest for a crisp cybernetic shimmer
        let intensity = wave * wave;
        let rgb = base.lerp(peak, intensity);
        out.push_str(&format!("\x1b[1;38;2;{};{};{}m{}", rgb.0, rgb.1, rgb.2, ch));
    }
    out.push_str(style::RESET);
    out
}

/// Render the breathing idle prompt (`❯`) with a smooth 1.5-second (90-frame at 60 FPS) luminance pulse.
#[must_use]
pub fn render_breathing_prompt(tick: u64, color_enabled: bool) -> String {
    if !color_enabled {
        return Symbols::PROMPT.to_string();
    }
    let phase = ((tick % BREATHING_PERIOD_FRAMES) as f64 / (BREATHING_PERIOD_FRAMES as f64)) * TAU;
    let factor = (1.0 - phase.cos()) * 0.5;
    let dim_cyan = Rgb(30, 100, 145);
    let bright_cyan = palette::CYBER_CYAN.lerp(palette::FROST_WHITE, 0.35);
    let current = dim_cyan.lerp(bright_cyan, factor);
    format!(
        "\x1b[1;38;2;{};{};{}m{}{}",
        current.0,
        current.1,
        current.2,
        Symbols::PROMPT,
        style::RESET
    )
}

/// State tracker for an animated gate status transition flash.
///
/// Flashes the row with a luminous crimson (Gate 1 RED) or emerald (Gate 2 GREEN)
/// highlight on frame 0 and smoothly decays into its stable state over `GATE_FLASH_DURATION_FRAMES`.
#[derive(Debug, Clone)]
pub struct GateFlashAnimation {
    pub kind: GateFlashKind,
    pub gate_label: String,
    pub summary: String,
    elapsed_frames: u64,
    total_frames: u64,
}

impl GateFlashAnimation {
    #[must_use]
    pub const fn new(gate_label: String, summary: String) -> Self {
        Self::with_kind(GateFlashKind::Gate2GreenPassed, gate_label, summary)
    }

    #[must_use]
    pub const fn with_kind(kind: GateFlashKind, gate_label: String, summary: String) -> Self {
        Self {
            kind,
            gate_label,
            summary,
            elapsed_frames: 0,
            total_frames: GATE_FLASH_DURATION_FRAMES,
        }
    }

    /// Advance the flash animation by one frame.
    pub fn step(&mut self) {
        if self.elapsed_frames < self.total_frames {
            self.elapsed_frames = self.elapsed_frames.saturating_add(1);
        }
    }

    /// Return true once the flash transition has settled into its steady passed state.
    #[must_use]
    pub const fn is_finished(&self) -> bool {
        self.elapsed_frames >= self.total_frames
    }

    /// Return current normalized decay progress in `[0.0, 1.0]` (`0.0` = peak emerald flash, `1.0` = settled).
    #[must_use]
    pub fn decay_progress(&self) -> f64 {
        if self.total_frames == 0 {
            return 1.0;
        }
        (self.elapsed_frames as f64 / self.total_frames as f64).clamp(0.0, 1.0)
    }

    /// Render the animated status row for the current frame.
    #[must_use]
    pub fn render_row(&self, color_enabled: bool) -> String {
        let badge_text = match self.kind {
            GateFlashKind::Gate1RedConfirmed => "RED CONFIRMED",
            GateFlashKind::Gate2GreenPassed => "PASS",
        };
        if !color_enabled {
            return format!(
                "  {} {: <14}  {}  {}",
                Symbols::CHECK,
                self.gate_label,
                badge_text,
                self.summary
            );
        }

        let t = self.decay_progress();
        if t >= 1.0 {
            let muted_fg = palette::MUTED_STEEL.fg_ansi();
            let (check_fg, pill_code) = match self.kind {
                GateFlashKind::Gate1RedConfirmed => (palette::CRIMSON.fg_ansi(), style::PILL_FAIL),
                GateFlashKind::Gate2GreenPassed => (palette::EMERALD.fg_ansi(), style::PILL_PASS),
            };
            return format!(
                "  {}{}{} {}{: <14}{} {} {} {} {}{}{}",
                check_fg,
                Symbols::CHECK,
                style::RESET,
                muted_fg,
                self.gate_label,
                style::RESET,
                pill_code,
                badge_text,
                style::RESET,
                muted_fg,
                self.summary,
                style::RESET
            );
        }

        let peak_bg = match self.kind {
            GateFlashKind::Gate1RedConfirmed => Rgb(190, 24, 60),
            GateFlashKind::Gate2GreenPassed => Rgb(16, 145, 100),
        };
        let flash_bg = peak_bg.lerp(palette::SLATE_BG, t);
        let flash_fg = palette::FROST_WHITE.lerp(palette::MUTED_STEEL, t * 0.7);
        format!(
            "\x1b[1;38;2;{};{};{};48;2;{};{};{}m  {} {: <14}  {}  {} {}",
            flash_fg.0,
            flash_fg.1,
            flash_fg.2,
            flash_bg.0,
            flash_bg.1,
            flash_bg.2,
            Symbols::CHECK,
            self.gate_label,
            badge_text,
            self.summary,
            style::RESET
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fractional_progress_bar_uses_8_level_unicode_blocks() {
        // 1/8th of a 4-cell bar = 1 eighth -> '▏' in cell 0
        let bar_eighth = render_fractional_bar(1.0 / 32.0, 4, false);
        assert!(bar_eighth.starts_with('▏'));

        // Half of a 4-cell bar = 2 full blocks + 2 empty
        let bar_half = render_fractional_bar(0.5, 4, false);
        assert_eq!(bar_half, "██──");

        // Full bar = 4 full blocks '████'
        let bar_full = render_fractional_bar(1.0, 4, true);
        assert!(bar_full.contains('█'));
        assert!(bar_full.contains("\x1b[38;2;"));
    }

    #[test]
    fn test_wave_shimmer_interpolates_24bit_rgb_across_ticks() {
        let frame_0 = render_wave_shimmer(
            "Synthesizing AST repair...",
            0,
            palette::MUTED_STEEL,
            palette::CYBER_CYAN,
            true,
        );
        let frame_15 = render_wave_shimmer(
            "Synthesizing AST repair...",
            15,
            palette::MUTED_STEEL,
            palette::CYBER_CYAN,
            true,
        );
        assert!(frame_0.contains("\x1b[1;38;2;"));
        assert_ne!(frame_0, frame_15);

        let plain = render_wave_shimmer(
            "Synthesizing AST repair...",
            15,
            palette::MUTED_STEEL,
            palette::CYBER_CYAN,
            false,
        );
        assert_eq!(plain, "Synthesizing AST repair...");
    }

    #[test]
    fn test_breathing_prompt_90_frame_period() {
        let p0 = render_breathing_prompt(0, true);
        let p45 = render_breathing_prompt(45, true);
        let p90 = render_breathing_prompt(90, true);

        assert!(p0.contains(Symbols::PROMPT));
        assert!(p45.contains("\x1b[1;38;2;"));
        assert_ne!(p0, p45);
        assert_eq!(p0, p90);
    }

    #[test]
    fn test_gate_flash_animation_emerald_highlight_to_settled_muted() {
        let mut engine = MotionEngine::new();
        engine.trigger_gate_pass_flash("GATE 2 (GREEN)", "2 consecutive passes verified");

        let first_row = engine.active_flash().unwrap().render_row(true);
        assert!(first_row.contains("\x1b[1;38;2;"));
        assert!(first_row.contains(";48;2;16;145;100m"));

        for _ in 0..GATE_FLASH_DURATION_FRAMES {
            engine.step_frame();
        }
        assert!(engine.active_flash().is_none());

        let mut settled = GateFlashAnimation::new("GATE 2".to_string(), "Verified".to_string());
        for _ in 0..GATE_FLASH_DURATION_FRAMES {
            settled.step();
        }
        let settled_row = settled.render_row(true);
        assert!(settled_row.contains("\x1b[38;2;148;163;184m"));
    }
}
