//! Terminal UI design tokens, 24-bit Truecolor RGB policies, output modes,
//! rounded Unicode box geometry, and background-filled pill badges for ReplayFix CLI (v1.9.5).
//!
//! Implements the "Living Cybernetic Command Deck" inspired by Claude Code, Charm Lipgloss/Bubbles, and Lazygit:
//! - Three explicit output modes: Human, Quiet, and Machine.
//! - Deterministic color precedence: CLI flag `--color` > `NO_COLOR` > `CLICOLOR_FORCE` > `TERM=dumb` > `is_terminal()`.
//! - 24-bit Truecolor RGB foreground (`\x1b[38;2;R;G;Bm`) and background (`\x1b[48;2;R;G;Bm`) escapes (zero legacy 16-color escapes).
//! - Rounded Unicode box geometry (`╭───╮`, `╰───╯`) and background-filled pill badges.
//! - Automatic fallback to unformatted plain stream when `!stdout().is_terminal()`.

use std::env;
use std::fmt;
use std::io::{self, IsTerminal};

/// Explicit user-requested color policy via CLI flag or config.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColorOption {
    #[default]
    Auto,
    Always,
    Never,
}

impl std::str::FromStr for ColorOption {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "auto" => Ok(Self::Auto),
            "always" => Ok(Self::Always),
            "never" => Ok(Self::Never),
            other => Err(format!(
                "Invalid color option '{other}'; valid values are: auto, always, never"
            )),
        }
    }
}

/// Output verbosity and format mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputMode {
    #[default]
    Human,
    Quiet,
    Machine,
}

impl ColorOption {
    /// Evaluate active color state with strict, documented precedence:
    /// 1. `--color always` -> TRUE
    /// 2. `--color never` -> FALSE
    /// 3. `--color auto` (or default):
    ///    a. `NO_COLOR` set (and not empty) -> FALSE
    ///    b. `CLICOLOR_FORCE` set and != "0" -> TRUE
    ///    c. `TERM=dumb` -> FALSE
    ///    d. Target stream is a TTY (`is_terminal()`) -> TRUE
    ///    e. Otherwise -> FALSE
    #[must_use]
    pub fn resolve_stream(self, is_terminal: bool) -> bool {
        match self {
            Self::Always => true,
            Self::Never => false,
            Self::Auto => {
                if env::var_os("NO_COLOR").is_some() {
                    return false;
                }
                if let Ok(force) = env::var("CLICOLOR_FORCE") {
                    if force != "0" {
                        return true;
                    }
                }
                if let Ok(term) = env::var("TERM") {
                    if term == "dumb" {
                        return false;
                    }
                }
                is_terminal
            }
        }
    }
}

/// 24-bit Truecolor RGB triplet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    /// Linearly interpolate between `self` and `other` with factor `t` clamped to `[0.0, 1.0]`.
    #[must_use]
    pub fn lerp(self, other: Self, t: f64) -> Self {
        let clamped = t.clamp(0.0, 1.0);
        let r = f64::from(self.0) + (f64::from(other.0) - f64::from(self.0)) * clamped;
        let g = f64::from(self.1) + (f64::from(other.1) - f64::from(self.1)) * clamped;
        let b = f64::from(self.2) + (f64::from(other.2) - f64::from(self.2)) * clamped;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        Self(r.round() as u8, g.round() as u8, b.round() as u8)
    }

    /// Format as 24-bit Truecolor foreground escape (`\x1b[38;2;R;G;Bm`).
    #[must_use]
    pub fn fg_ansi(self) -> String {
        format!("\x1b[38;2;{};{};{}m", self.0, self.1, self.2)
    }

    /// Format as 24-bit Truecolor background escape (`\x1b[48;2;R;G;Bm`).
    #[must_use]
    pub fn bg_ansi(self) -> String {
        format!("\x1b[48;2;{};{};{}m", self.0, self.1, self.2)
    }
}

/// 24-bit Truecolor Cybernetic Command Deck Palette.
pub mod palette {
    use super::Rgb;

    pub const CYBER_CYAN: Rgb = Rgb(56, 189, 248);
    pub const EMERALD: Rgb = Rgb(16, 185, 129);
    pub const EMERALD_BRIGHT: Rgb = Rgb(52, 211, 153);
    pub const AMBER: Rgb = Rgb(245, 158, 11);
    pub const CRIMSON: Rgb = Rgb(244, 63, 94);
    pub const ELECTRIC_BLUE: Rgb = Rgb(96, 165, 250);
    pub const LAVENDER: Rgb = Rgb(167, 139, 250);
    pub const FROST_WHITE: Rgb = Rgb(248, 250, 252);
    pub const MUTED_STEEL: Rgb = Rgb(148, 163, 184);
    pub const SLATE_BG: Rgb = Rgb(15, 23, 42);
    pub const PILL_DARK_FG: Rgb = Rgb(9, 13, 22);
    pub const DOCK_PILL_BG: Rgb = Rgb(30, 41, 59);
}

/// Cybernetic Truecolor Palette struct for TUI layout and widget themes.
#[derive(Debug, Clone, Copy)]
pub struct CyberPalette;

impl CyberPalette {
    pub const CYAN: Rgb = Rgb(56, 189, 248);
    pub const CYAN_BG: Rgb = Rgb(12, 74, 110);
    pub const EMERALD: Rgb = Rgb(16, 185, 129);
    pub const EMERALD_BG: Rgb = Rgb(6, 78, 59);
    pub const AMBER: Rgb = Rgb(245, 158, 11);
    pub const AMBER_BG: Rgb = Rgb(120, 53, 15);
    pub const CRIMSON: Rgb = Rgb(244, 63, 94);
    pub const CRIMSON_BG: Rgb = Rgb(136, 19, 55);
    pub const VIOLET: Rgb = Rgb(168, 85, 247);
    pub const VIOLET_BG: Rgb = Rgb(59, 7, 100);
    pub const TITANIUM: Rgb = Rgb(248, 250, 252);
    pub const MUTED_STEEL: Rgb = Rgb(148, 163, 184);
    pub const SURFACE_BG: Rgb = Rgb(30, 41, 59);
    pub const VOID_BG: Rgb = Rgb(15, 23, 42);
}

/// 24-bit Truecolor ANSI styling primitives (zero legacy 16-color escapes).
pub mod style {
    pub const RESET: &str = "\x1b[0m";
    pub const BOLD: &str = "\x1b[1m";
    pub const DIM: &str = "\x1b[38;2;148;163;184m";
    pub const UNDERLINE: &str = "\x1b[4m";

    pub const RED: &str = "\x1b[38;2;244;63;94m";
    pub const GREEN: &str = "\x1b[38;2;16;185;129m";
    pub const YELLOW: &str = "\x1b[38;2;245;158;11m";
    pub const BLUE: &str = "\x1b[38;2;96;165;250m";
    pub const MAGENTA: &str = "\x1b[38;2;167;139;250m";
    pub const CYAN: &str = "\x1b[38;2;56;189;248m";
    pub const WHITE: &str = "\x1b[38;2;248;250;252m";

    pub const BOLD_RED: &str = "\x1b[1;38;2;244;63;94m";
    pub const BOLD_GREEN: &str = "\x1b[1;38;2;16;185;129m";
    pub const BOLD_YELLOW: &str = "\x1b[1;38;2;245;158;11m";
    pub const BOLD_BLUE: &str = "\x1b[1;38;2;96;165;250m";
    pub const BOLD_CYAN: &str = "\x1b[1;38;2;56;189;248m";
    pub const BOLD_WHITE: &str = "\x1b[1;38;2;248;250;252m";

    pub const PILL_PASS: &str = "\x1b[1;38;2;9;13;22;48;2;16;185;129m";
    pub const PILL_FAIL: &str = "\x1b[1;38;2;248;250;252;48;2;244;63;94m";
    pub const PILL_WARN: &str = "\x1b[1;38;2;9;13;22;48;2;245;158;11m";
    pub const PILL_RUN: &str = "\x1b[1;38;2;9;13;22;48;2;56;189;248m";
    pub const PILL_MUTED: &str = "\x1b[1;38;2;226;232;240;48;2;51;65;85m";
    pub const PILL_HOTKEY: &str = "\x1b[1;38;2;56;189;248;48;2;30;41;59m";
}

/// Semantic styling painter with 24-bit Truecolor & background-filled pill badges.
#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub color_enabled: bool,
    pub mode: OutputMode,
}

impl Theme {
    #[must_use]
    pub fn for_stdout(color: ColorOption, mode: OutputMode) -> Self {
        Self {
            color_enabled: color.resolve_stream(io::stdout().is_terminal()),
            mode,
        }
    }

    #[must_use]
    pub fn for_stderr(color: ColorOption, mode: OutputMode) -> Self {
        Self {
            color_enabled: color.resolve_stream(io::stderr().is_terminal()),
            mode,
        }
    }

    #[must_use]
    pub fn brand<'a>(&self, text: &'a str) -> Styled<'a> {
        self.apply(text, style::BOLD_CYAN)
    }

    #[must_use]
    pub fn success<'a>(&self, text: &'a str) -> Styled<'a> {
        self.apply(text, style::BOLD_GREEN)
    }

    #[must_use]
    pub fn warning<'a>(&self, text: &'a str) -> Styled<'a> {
        self.apply(text, style::BOLD_YELLOW)
    }

    #[must_use]
    pub fn error<'a>(&self, text: &'a str) -> Styled<'a> {
        self.apply(text, style::BOLD_RED)
    }

    #[must_use]
    pub fn info<'a>(&self, text: &'a str) -> Styled<'a> {
        self.apply(text, style::BOLD_BLUE)
    }

    #[must_use]
    pub fn bold<'a>(&self, text: &'a str) -> Styled<'a> {
        self.apply(text, style::BOLD)
    }

    #[must_use]
    pub fn muted<'a>(&self, text: &'a str) -> Styled<'a> {
        self.apply(text, style::DIM)
    }

    /// Render a background-filled Truecolor pill badge (`\x1b[48;2;...m LABEL \x1b[0m`).
    #[must_use]
    pub fn pill<'a>(&self, label: &'a str, fg: Rgb, bg: Rgb) -> Pill<'a> {
        Pill {
            label,
            fg,
            bg,
            enabled: self.color_enabled,
        }
    }

    /// Semantic emerald pass pill badge.
    #[must_use]
    pub fn pill_success<'a>(&self, label: &'a str) -> Pill<'a> {
        self.pill(label, palette::PILL_DARK_FG, palette::EMERALD)
    }

    /// Semantic crimson failure pill badge.
    #[must_use]
    pub fn pill_error<'a>(&self, label: &'a str) -> Pill<'a> {
        self.pill(label, palette::FROST_WHITE, palette::CRIMSON)
    }

    /// Semantic amber warning pill badge.
    #[must_use]
    pub fn pill_warning<'a>(&self, label: &'a str) -> Pill<'a> {
        self.pill(label, palette::PILL_DARK_FG, palette::AMBER)
    }

    /// Semantic cyber-cyan active/info pill badge.
    #[must_use]
    pub fn pill_info<'a>(&self, label: &'a str) -> Pill<'a> {
        self.pill(label, palette::PILL_DARK_FG, palette::CYBER_CYAN)
    }

    /// Semantic muted slate pill badge.
    #[must_use]
    pub fn pill_muted<'a>(&self, label: &'a str) -> Pill<'a> {
        self.pill(label, palette::FROST_WHITE, palette::DOCK_PILL_BG)
    }

    /// Semantic inverted hotkey command pill badge.
    #[must_use]
    pub fn pill_hotkey<'a>(&self, label: &'a str) -> Pill<'a> {
        self.pill(label, palette::CYBER_CYAN, palette::DOCK_PILL_BG)
    }

    /// Render a semantic status pill badge with background fill.
    #[must_use]
    pub fn status_pill(&self, status: StepStatus) -> Pill<'static> {
        match status {
            StepStatus::Done => self.pill_success("PASS"),
            StepStatus::Fail => self.pill_error("FAIL"),
            StepStatus::Warn => self.pill_warning("WARN"),
            StepStatus::Running => self.pill_info("RUN "),
            StepStatus::Skipped => self.pill_muted("SKIP"),
        }
    }

    /// Render an inverted background command pill for dock hotkeys.
    #[must_use]
    pub fn command_pill<'a>(&self, text: &'a str) -> Pill<'a> {
        self.pill_hotkey(text)
    }

    fn apply<'a>(&self, text: &'a str, code: &'static str) -> Styled<'a> {
        Styled {
            text,
            code,
            enabled: self.color_enabled,
        }
    }
}

/// A string slice tagged with a 24-bit Truecolor ANSI style code that conditionally formats based on `enabled`.
#[derive(Debug, Clone, Copy)]
pub struct Styled<'a> {
    text: &'a str,
    code: &'static str,
    enabled: bool,
}

impl<'a> fmt::Display for Styled<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.enabled {
            write!(f, "{}{}{}", self.code, self.text, style::RESET)
        } else {
            f.write_str(self.text)
        }
    }
}

/// Background-filled 24-bit Truecolor pill badge (`\x1b[1;38;2;R;G;B;48;2;R;G;Bm LABEL \x1b[0m`).
#[derive(Debug, Clone, Copy)]
pub struct Pill<'a> {
    label: &'a str,
    fg: Rgb,
    bg: Rgb,
    enabled: bool,
}

impl<'a> fmt::Display for Pill<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.enabled {
            write!(
                f,
                "\x1b[1;38;2;{};{};{};48;2;{};{};{}m {} {}",
                self.fg.0,
                self.fg.1,
                self.fg.2,
                self.bg.0,
                self.bg.1,
                self.bg.2,
                self.label,
                style::RESET
            )
        } else {
            write!(f, " {} ", self.label)
        }
    }
}

/// Accessible terminal & Nerd Font symbols with rounded Unicode box geometry.
#[derive(Debug, Clone, Copy)]
pub struct Symbols;

impl Symbols {
    pub const CHECK: &str = "✔";
    pub const CROSS: &str = "✖";
    pub const WARN: &str = "▲";
    pub const BULLET: &str = "◆";
    pub const ARROW_RIGHT: &str = "→";
    pub const STEP: &str = "›";
    pub const SHIELD: &str = "⛨";
    pub const PROMPT: &str = "❯";
    pub const BRANCH: &str = "󰊢";
    pub const SPEC: &str = "󰈚";
    pub const SPARK: &str = "⚡";
    pub const CORE: &str = "󰒓";
    pub const BOX_TL: &str = "╭";
    pub const BOX_TR: &str = "╮";
    pub const BOX_BL: &str = "╰";
    pub const BOX_BR: &str = "╯";
    pub const BOX_H: &str = "─";
    pub const BOX_V: &str = "│";
}

/// Format a rounded Unicode box container (`╭───╮ ... ╰───╯`) around a title and body lines.
#[must_use]
pub fn format_rounded_box(
    theme: &Theme,
    title: &str,
    lines: &[String],
    inner_width: usize,
) -> String {
    let mut out = String::new();
    let w = inner_width.max(title.chars().count().saturating_add(8));
    let top_rule = format!(
        "  {}{}{}",
        Symbols::BOX_TL,
        Symbols::BOX_H.repeat(w),
        Symbols::BOX_TR
    );
    let bot_rule = format!(
        "  {}{}{}",
        Symbols::BOX_BL,
        Symbols::BOX_H.repeat(w),
        Symbols::BOX_BR
    );

    out.push_str(&format!("{}", theme.muted(&top_rule)));
    out.push('\n');
    let header_plain = format!(" ReplayFix // {title}");
    let header_pad = w.saturating_sub(header_plain.chars().count());
    out.push_str(&format!(
        "  {} {} {}{}{}",
        theme.muted(Symbols::BOX_V),
        theme.brand("ReplayFix //"),
        theme.bold(title),
        " ".repeat(header_pad),
        theme.muted(Symbols::BOX_V)
    ));
    out.push('\n');

    if !lines.is_empty() {
        let mid_rule = format!("  ├{}┤", Symbols::BOX_H.repeat(w));
        out.push_str(&format!("{}", theme.muted(&mid_rule)));
        out.push('\n');
        for line in lines {
            let char_len = line.chars().count();
            let pad = w.saturating_sub(char_len.saturating_add(2));
            out.push_str(&format!(
                "  {} {}{} {}",
                theme.muted(Symbols::BOX_V),
                line,
                " ".repeat(pad),
                theme.muted(Symbols::BOX_V)
            ));
            out.push('\n');
        }
    }

    out.push_str(&format!("{}", theme.muted(&bot_rule)));
    out
}

/// Render a rounded Unicode section header banner (`╭───╮` / `╰───╯`).
pub fn print_banner(theme: &Theme, title: &str) {
    if theme.mode == OutputMode::Quiet || theme.mode == OutputMode::Machine {
        return;
    }
    let inner_width = 62usize.max(title.chars().count().saturating_add(18));
    let horizontal = Symbols::BOX_H.repeat(inner_width);
    let top = format!("  {}{}{}", Symbols::BOX_TL, horizontal, Symbols::BOX_TR);
    let bottom = format!("  {}{}{}", Symbols::BOX_BL, horizontal, Symbols::BOX_BR);
    let content_len = "ReplayFix // ".chars().count() + title.chars().count();
    let pad = " ".repeat(inner_width.saturating_sub(content_len.saturating_add(2)));

    println!("{}", theme.muted(&top));
    println!(
        "  {} {} {}{}{}",
        theme.muted(Symbols::BOX_V),
        theme.brand("ReplayFix //"),
        theme.bold(title),
        pad,
        theme.muted(Symbols::BOX_V)
    );
    println!("{}", theme.muted(&bottom));
}

/// Print a clean labeled field line: `Label : Value (Note)`.
pub fn print_field(theme: &Theme, label: &str, value: &str, note: Option<&str>) {
    if theme.mode == OutputMode::Quiet || theme.mode == OutputMode::Machine {
        return;
    }
    let styled_label = format!("{: <14}", label);
    if let Some(n) = note {
        println!(
            "  {} : {} {}",
            theme.muted(&styled_label),
            theme.bold(value),
            theme.muted(&format!("({n})"))
        );
    } else {
        println!("  {} : {}", theme.muted(&styled_label), theme.bold(value));
    }
}

/// Phase execution status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepStatus {
    Running,
    Done,
    Fail,
    Warn,
    Skipped,
}

/// Print an operational phase step with background-filled 24-bit Truecolor pill badges.
pub fn print_step(
    theme: &Theme,
    phase: &str,
    status: StepStatus,
    headline: &str,
    detail: Option<&str>,
) {
    if theme.mode == OutputMode::Quiet || theme.mode == OutputMode::Machine {
        return;
    }
    let (icon, pill) = match status {
        StepStatus::Done => (theme.success(Symbols::CHECK), theme.pill_success("PASS")),
        StepStatus::Fail => (theme.error(Symbols::CROSS), theme.pill_error("FAIL")),
        StepStatus::Warn => (theme.warning(Symbols::WARN), theme.pill_warning("WARN")),
        StepStatus::Running => (theme.info(Symbols::BULLET), theme.pill_info("RUN ")),
        StepStatus::Skipped => (theme.muted(Symbols::ARROW_RIGHT), theme.pill_muted("SKIP")),
    };

    println!("  {icon} {: <14} {pill} {}", theme.bold(phase), headline);
    if let Some(d) = detail {
        println!("    {} {}", theme.muted(Symbols::STEP), theme.muted(d));
    }
}

/// Rich diagnostic check structure for `replayfix doctor`.
#[derive(Debug, Clone)]
pub struct DiagnosticCheck {
    pub name: String,
    pub status: DiagnosticStatus,
    pub finding: String,
    pub impact: String,
    pub remediation: Option<String>,
    pub evidence: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticStatus {
    Ok,
    Warn,
    Fail,
    NotChecked,
}

/// Render structured diagnostics with background-filled Truecolor pill badges.
pub fn print_diagnostics(theme: &Theme, checks: &[DiagnosticCheck]) {
    if theme.mode == OutputMode::Quiet || theme.mode == OutputMode::Machine {
        return;
    }
    for check in checks {
        let (icon, badge) = match check.status {
            DiagnosticStatus::Ok => (
                theme.success(Symbols::CHECK),
                theme.pill_success("OK         "),
            ),
            DiagnosticStatus::Warn => (
                theme.warning(Symbols::WARN),
                theme.pill_warning("WARN       "),
            ),
            DiagnosticStatus::Fail => {
                (theme.error(Symbols::CROSS), theme.pill_error("MISSING    "))
            }
            DiagnosticStatus::NotChecked => (
                theme.muted(Symbols::BULLET),
                theme.pill_muted("NOT CHECKED"),
            ),
        };
        println!(
            "  {icon} {} {: <14} {}",
            badge,
            theme.bold(&check.name),
            check.finding
        );
        println!(
            "     {} Evidence : {}",
            theme.muted(Symbols::STEP),
            theme.muted(&check.evidence)
        );
        if check.status != DiagnosticStatus::Ok {
            println!(
                "     {} Impact   : {}",
                theme.warning(Symbols::STEP),
                check.impact
            );
            if let Some(remedy) = &check.remediation {
                println!(
                    "     {} Action   : {}",
                    theme.info(Symbols::ARROW_RIGHT),
                    remedy
                );
            }
        }
    }
}

/// Print execution security & trust boundary card inside a rounded Unicode box (`╭───╮` / `╰───╯`).
pub fn print_security_boundary(theme: &Theme, repo: &str, is_read_only: bool, isolated: bool) {
    if theme.mode == OutputMode::Quiet || theme.mode == OutputMode::Machine {
        return;
    }
    let repo_mode = if is_read_only {
        "Strictly Read-Only (Δrepo = 0 invariant enforced)"
    } else {
        "Direct Mutation Permitted"
    };
    let isolation_status = if isolated {
        "Active (ephemeral scratch workspace in OS temp dir)"
    } else {
        "Disabled (running directly in workspace)"
    };

    let lines = [
        format!("Target Repository : {repo} ({repo_mode})"),
        format!("Execution Sandbox  : {isolation_status}"),
        "Test Freeze Hash   : SHA-256 pre-flight invariant active".to_string(),
    ];
    let max_line = lines
        .iter()
        .map(|s| s.chars().count().saturating_add(4))
        .max()
        .unwrap_or(60)
        .max(62);
    let horiz = Symbols::BOX_H.repeat(max_line);

    println!();
    println!(
        "  {}",
        theme.muted(&format!("{}{}{}", Symbols::BOX_TL, horiz, Symbols::BOX_TR))
    );
    let title = "Security & Isolation Guarantees";
    let title_pad = " ".repeat(max_line.saturating_sub(title.chars().count().saturating_add(4)));
    println!(
        "  {} {} {}{}{}",
        theme.muted(Symbols::BOX_V),
        theme.brand(Symbols::SHIELD),
        theme.bold(title),
        title_pad,
        theme.muted(Symbols::BOX_V)
    );
    for line in &lines {
        let pad = " ".repeat(max_line.saturating_sub(line.chars().count().saturating_add(4)));
        println!(
            "  {} {} {}{}{}",
            theme.muted(Symbols::BOX_V),
            theme.muted(Symbols::STEP),
            line,
            pad,
            theme.muted(Symbols::BOX_V)
        );
    }
    println!(
        "  {}",
        theme.muted(&format!("{}{}{}", Symbols::BOX_BL, horiz, Symbols::BOX_BR))
    );
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_precedence() {
        // 1. Explicit Always beats NO_COLOR
        temp_env::with_vars([("NO_COLOR", Some("1"))], || {
            assert!(ColorOption::Always.resolve_stream(false));
        });

        // 2. Explicit Never beats CLICOLOR_FORCE
        temp_env::with_vars([("CLICOLOR_FORCE", Some("1"))], || {
            assert!(!ColorOption::Never.resolve_stream(true));
        });

        // 3. Auto respects NO_COLOR
        temp_env::with_vars([("NO_COLOR", Some("1"))], || {
            assert!(!ColorOption::Auto.resolve_stream(true));
        });

        // 4. Auto respects CLICOLOR_FORCE over non-TTY
        temp_env::with_vars([("NO_COLOR", None), ("CLICOLOR_FORCE", Some("1"))], || {
            assert!(ColorOption::Auto.resolve_stream(false));
        });

        // 5. Auto respects TERM=dumb
        temp_env::with_vars(
            [
                ("NO_COLOR", None),
                ("CLICOLOR_FORCE", None),
                ("TERM", Some("dumb")),
            ],
            || {
                assert!(!ColorOption::Auto.resolve_stream(true));
            },
        );
    }

    #[test]
    fn test_theme_methods_produce_24bit_truecolor_and_pills() {
        let theme = Theme {
            color_enabled: true,
            mode: OutputMode::Human,
        };
        assert!(format!("{}", theme.brand("brand")).contains("\x1b[1;38;2;56;189;248m"));
        assert!(format!("{}", theme.success("ok")).contains("\x1b[1;38;2;16;185;129m"));
        assert!(format!("{}", theme.error("err")).contains("\x1b[1;38;2;244;63;94m"));
        assert!(format!("{}", theme.warning("warn")).contains("\x1b[1;38;2;245;158;11m"));
        assert!(format!("{}", theme.info("info")).contains("\x1b[1;38;2;96;165;250m"));
        assert!(format!("{}", theme.bold("bold")).contains("\x1b[1m"));
        assert!(format!("{}", theme.muted("dim")).contains("\x1b[38;2;148;163;184m"));

        let pass_pill = format!("{}", theme.pill_success("PASS"));
        assert!(pass_pill.contains("\x1b[1;38;2;9;13;22;48;2;16;185;129m PASS \x1b[0m"));

        let plain_theme = Theme {
            color_enabled: false,
            mode: OutputMode::Human,
        };
        assert_eq!(format!("{}", plain_theme.pill_success("PASS")), " PASS ");
    }
}
