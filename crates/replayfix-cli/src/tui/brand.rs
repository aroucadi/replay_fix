//! ReplayFix Signature Brand Identity (`tui/brand.rs`).
//!
//! Captures the visual soul of ReplayFix — Video Replay (Keyframe / Playback `▶`)
//! meets Autonomous Bug Healing (Substrate / Spark / Verification `󰒓` / `⬢`):
//! - **Hero Splash Banner (Startup)**: Geometric Unicode block logotype with an algorithmic
//!   4-stop 24-bit Truecolor horizontal gradient:
//!   Electric Cyan (`#00d2ff`) ──> Cyber Violet (`#7928ca`) ──> Phosphor Amber (`#f59e0b`) ──> Substrate Emerald (`#10b981`),
//!   paired with the branded metadata ribbon.
//! - **Compact HUD Emblem (Running State)**: Dual-pill micro-emblem for the Bento Top Deck
//!   and interactive prompt (`▶⚡ REPLAYFIX` + version badge).
//! - **Graceful Terminal Degradation**:
//!   * Collapses Hero Splash into the Compact HUD Emblem when `cols < 80`.
//!   * Degrades cleanly to unformatted ASCII/Unicode text when `NO_COLOR` or `!color_enabled`.

use crate::ui::{style, Rgb, Theme};

/// 4-stop horizontal brand gradient stops:
/// 1. Electric Cyan (`#00d2ff` -> `Rgb(0, 210, 255)`)
/// 2. Cyber Violet (`#7928ca` -> `Rgb(121, 40, 202)`)
/// 3. Phosphor Amber (`#f59e0b` -> `Rgb(245, 158, 11)`)
/// 4. Substrate Emerald (`#10b981` -> `Rgb(16, 185, 129)`)
pub const GRADIENT_ELECTRIC_CYAN: Rgb = Rgb(0, 210, 255);
pub const GRADIENT_CYBER_VIOLET: Rgb = Rgb(121, 40, 202);
pub const GRADIENT_PHOSPHOR_AMBER: Rgb = Rgb(245, 158, 11);
pub const GRADIENT_SUBSTRATE_EMERALD: Rgb = Rgb(16, 185, 129);

/// Geometric Unicode block art lines for the ReplayFix Hero Splash Banner
/// combining the playback triangle (`▶`) with the causal verification hex/gear (`⬢` / `󰒓`).
pub const HERO_LOGO_LINES: [&str; 6] = [
    "  ╭──────╮    ██████╗ ███████╗██████╗ ██╗      █████╗ ██╗   ██╗███████╗██╗██╗  ██╗",
    "  │ ▶  ⬢ │    ██╔══██╗██╔════╝██╔══██╗██║     ██╔══██╗╚██╗ ██╔╝██╔════╝██║╚██╗██╔╝",
    "  │ 󰒓  ⚡ │    ██████╔╝█████╗  ██████╔╝██║     ███████║ ╚████╔╝ █████╗  ██║ ╚███╔╝ ",
    "  │ █▉▊▋ │    ██╔══██╗██╔══╝  ██╔═══╝ ██║     ██╔══██║  ╚██╔╝  ██╔══╝  ██║ ██╔██╗ ",
    "  ╰──────╯    ██║  ██║███████╗██║     ███████╗██║  ██║   ██║   ██║     ██║██╔╝ ██╗",
    "              ╚═╝  ╚═╝╚══════╝╚═╝     ╚══════╝╚═╝  ╚═╝   ╚═╝   ╚═╝     ╚═╝╚═╝  ╚═╝",
];

/// Branded tagline displayed in the Hero Splash metadata ribbon.
pub const BRAND_TAGLINE: &str =
    "▶ REPLAYFIX // Autonomous Bug Reproduction, Causal Verification & Repair";

/// Default version label used in the branded metadata ribbon and compact HUD emblem.
pub const BRAND_VERSION_TAG: &str = "v1.9.5";

/// Compute the 24-bit Truecolor RGB value at normalized horizontal position `t` in `[0.0, 1.0]`
/// across the 4-stop ReplayFix signature gradient:
/// Electric Cyan (`#00d2ff`) -> Cyber Violet (`#7928ca`) -> Phosphor Amber (`#f59e0b`) -> Substrate Emerald (`#10b981`).
#[must_use]
pub fn sample_brand_gradient(t: f64) -> Rgb {
    let clamped = t.clamp(0.0, 1.0);
    if clamped < (1.0 / 3.0) {
        let local = clamped * 3.0;
        GRADIENT_ELECTRIC_CYAN.lerp(GRADIENT_CYBER_VIOLET, local)
    } else if clamped < (2.0 / 3.0) {
        let local = (clamped - (1.0 / 3.0)) * 3.0;
        GRADIENT_CYBER_VIOLET.lerp(GRADIENT_PHOSPHOR_AMBER, local)
    } else {
        let local = (clamped - (2.0 / 3.0)) * 3.0;
        GRADIENT_PHOSPHOR_AMBER.lerp(GRADIENT_SUBSTRATE_EMERALD, local)
    }
}

/// Apply the 4-stop horizontal 24-bit Truecolor gradient across a single line of text.
#[must_use]
pub fn render_gradient_line(line: &str, color_enabled: bool) -> String {
    if !color_enabled || line.is_empty() {
        return line.to_string();
    }
    let total_chars = line.chars().count();
    let denom = total_chars.saturating_sub(1).max(1) as f64;
    let mut out = String::with_capacity(line.len().saturating_mul(20));

    for (col_idx, ch) in line.chars().enumerate() {
        if ch == ' ' {
            out.push(' ');
            continue;
        }
        let t = (col_idx as f64) / denom;
        let rgb = sample_brand_gradient(t);
        out.push_str(&format!("\x1b[1;38;2;{};{};{}m{}", rgb.0, rgb.1, rgb.2, ch));
    }
    out.push_str(style::RESET);
    out
}

/// Render the Compact HUD Emblem for running state, Bento Top Deck, or narrow terminals (`< 80` cols).
///
/// When `color_enabled` is true, emits the dual-segment 24-bit Truecolor pill:
/// `\x1b[48;2;18;24;38m\x1b[38;2;56;189;248m ▶⚡ REPLAYFIX \x1b[48;2;30;41;59m\x1b[38;2;148;163;184m v1.9.0 \x1b[0m`
/// (with the specified `version` string, e.g. `"v1.9.0"` or `"v1.9.5"`).
#[must_use]
pub fn render_compact_hud_emblem_with_version(version: &str, color_enabled: bool) -> String {
    if color_enabled {
        format!(
            "\x1b[48;2;18;24;38m\x1b[38;2;56;189;248m ▶⚡ REPLAYFIX \x1b[48;2;30;41;59m\x1b[38;2;148;163;184m {version} \x1b[0m"
        )
    } else {
        format!("[ ▶⚡ REPLAYFIX {version} ]")
    }
}

/// Render the Compact HUD Emblem using the canonical v1.9.0/v1.9.5 badge signature.
#[must_use]
pub fn render_compact_hud_emblem(color_enabled: bool) -> String {
    render_compact_hud_emblem_with_version("v1.9.0", color_enabled)
}

/// Render the ReplayFix Hero Splash Banner for startup.
///
/// - When `cols < 80`, automatically collapses into the Compact HUD Emblem.
/// - When `!theme.color_enabled`, degrades to clean ASCII/Unicode text without ANSI escapes.
#[must_use]
pub fn render_hero_splash(cols: u16, theme: &Theme) -> String {
    if cols < 80 {
        return render_compact_hud_emblem(theme.color_enabled);
    }

    let mut out = String::with_capacity(2048);
    out.push('\n');
    for line in &HERO_LOGO_LINES {
        out.push_str(&render_gradient_line(line, theme.color_enabled));
        out.push('\n');
    }
    out.push('\n');

    let tagline_styled = render_gradient_line(BRAND_TAGLINE, theme.color_enabled);
    out.push_str(&format!("  {tagline_styled}\n"));

    if theme.color_enabled {
        let pill_ver = theme.pill("v1.9.0", Rgb(248, 250, 252), Rgb(18, 24, 38));
        let pill_sub = theme.pill("Rust Substrate: 60 FPS", Rgb(16, 185, 129), Rgb(6, 48, 36));
        let pill_ai = theme.pill(
            "Meta AI: Spark 1.3 + Wonder 1.0",
            Rgb(56, 189, 248),
            Rgb(12, 54, 88),
        );
        out.push_str(&format!("  {pill_ver} {pill_sub} {pill_ai}\n"));
    } else {
        out.push_str(
            "  [ v1.9.0 ] [ Rust Substrate: 60 FPS ] [ Meta AI: Spark 1.3 + Wonder 1.0 ]\n",
        );
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::{ColorOption, OutputMode};

    #[test]
    fn test_brand_gradient_samples_four_canonical_stops() {
        assert_eq!(sample_brand_gradient(0.0), GRADIENT_ELECTRIC_CYAN);
        assert_eq!(sample_brand_gradient(1.0 / 3.0), GRADIENT_CYBER_VIOLET);
        assert_eq!(sample_brand_gradient(2.0 / 3.0), GRADIENT_PHOSPHOR_AMBER);
        assert_eq!(sample_brand_gradient(1.0), GRADIENT_SUBSTRATE_EMERALD);
    }

    #[test]
    fn test_compact_hud_emblem_truecolor_and_plain_fallback() {
        let colored = render_compact_hud_emblem(true);
        assert_eq!(
            colored,
            "\x1b[48;2;18;24;38m\x1b[38;2;56;189;248m ▶⚡ REPLAYFIX \x1b[48;2;30;41;59m\x1b[38;2;148;163;184m v1.9.0 \x1b[0m"
        );

        let plain = render_compact_hud_emblem(false);
        assert_eq!(plain, "[ ▶⚡ REPLAYFIX v1.9.0 ]");
        assert!(!plain.contains('\x1b'));
    }

    #[test]
    fn test_hero_splash_banner_and_narrow_terminal_degradation() {
        let theme_color = Theme {
            color_enabled: true,
            mode: OutputMode::Human,
        };
        let splash = render_hero_splash(100, &theme_color);
        assert!(splash.contains("▶"));
        assert!(splash.contains("⬢"));
        assert!(splash.contains("󰒓"));
        assert!(splash.contains("Rust Substrate: 60 FPS"));
        assert!(splash.contains("Meta AI: Spark 1.3 + Wonder 1.0"));
        assert!(splash.contains("\x1b[1;38;2;0;210;255m"));

        // Narrow terminal (< 80 cols) collapses into compact HUD emblem
        let collapsed = render_hero_splash(72, &theme_color);
        assert_eq!(collapsed, render_compact_hud_emblem(true));

        // Plain fallback has zero ANSI escapes and preserves full tagline
        let theme_plain = Theme::for_stdout(ColorOption::Never, OutputMode::Human);
        let splash_plain = render_hero_splash(100, &theme_plain);
        assert!(!splash_plain.contains('\x1b'));
        assert!(splash_plain
            .contains("▶ REPLAYFIX // Autonomous Bug Reproduction, Causal Verification & Repair"));
        assert!(splash_plain
            .contains("[ v1.9.0 ] [ Rust Substrate: 60 FPS ] [ Meta AI: Spark 1.3 + Wonder 1.0 ]"));
    }
}
