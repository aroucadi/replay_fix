//! Terminal UI design tokens, color policies, and rendering helpers for FixTream / ReplayFix CLI.
//!
//! Provides a zero-dependency, lightweight terminal styling and presentation system that
//! respects `NO_COLOR`, `CLICOLOR_FORCE`, `TERM=dumb`, and output redirection via `std::io::IsTerminal`.

use std::env;
use std::fmt;
use std::io::{self, IsTerminal};

/// Terminal color policy determining whether ANSI escape sequences should be emitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorChoice {
    Auto,
    Always,
    Never,
}

impl ColorChoice {
    /// Detect the active color policy from the environment and stream capabilities.
    #[must_use]
    pub fn detect_stdout() -> bool {
        Self::should_color(io::stdout().is_terminal())
    }

    #[must_use]
    pub fn detect_stderr() -> bool {
        Self::should_color(io::stderr().is_terminal())
    }

    pub fn should_color(is_terminal: bool) -> bool {
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

/// ANSI styling primitives.
pub mod style {
    pub const RESET: &str = "\x1b[0m";
    pub const BOLD: &str = "\x1b[1m";
    pub const DIM: &str = "\x1b[2m";

    pub const RED: &str = "\x1b[31m";
    pub const GREEN: &str = "\x1b[32m";
    pub const YELLOW: &str = "\x1b[33m";
    pub const BLUE: &str = "\x1b[34m";
    pub const MAGENTA: &str = "\x1b[35m";
    pub const CYAN: &str = "\x1b[36m";
    pub const WHITE: &str = "\x1b[37m";

    pub const BOLD_RED: &str = "\x1b[1;31m";
    pub const BOLD_GREEN: &str = "\x1b[1;32m";
    pub const BOLD_YELLOW: &str = "\x1b[1;33m";
    pub const BOLD_BLUE: &str = "\x1b[1;34m";
    pub const BOLD_CYAN: &str = "\x1b[1;36m";
    pub const BOLD_WHITE: &str = "\x1b[1;37m";
}

/// Semantic styling painter.
#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub color_enabled: bool,
}

impl Theme {
    #[must_use]
    pub fn for_stdout() -> Self {
        Self {
            color_enabled: ColorChoice::detect_stdout(),
        }
    }

    #[must_use]
    pub fn for_stderr() -> Self {
        Self {
            color_enabled: ColorChoice::detect_stderr(),
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

    fn apply<'a>(&self, text: &'a str, code: &'static str) -> Styled<'a> {
        Styled {
            text,
            code,
            enabled: self.color_enabled,
        }
    }
}

/// A string slice tagged with an ANSI style code that conditionally formats based on `enabled`.
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

/// Accessible terminal symbols with plain-text fallback.
#[derive(Debug, Clone, Copy)]
pub struct Symbols;

impl Symbols {
    pub const CHECK: &str = "✔";
    pub const CROSS: &str = "✖";
    pub const WARN: &str = "▲";
    pub const BULLET: &str = "◆";
    pub const ARROW_RIGHT: &str = "→";
    pub const PIPE: &str = "│";
}

/// Render a section header banner with consistent visual hierarchy.
pub fn print_banner(theme: &Theme, title: &str) {
    let bar = "─".repeat(60);
    println!("{}", theme.muted(&bar));
    println!("  {} {}", theme.brand("FixTream //"), theme.bold(title));
    println!("{}", theme.muted(&bar));
}

/// Print a clean labeled field line: `Label : Value (Note)`.
pub fn print_field(theme: &Theme, label: &str, value: &str, note: Option<&str>) {
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

/// Print a step indicator for long-running workflows.
pub fn print_step(theme: &Theme, step: &str, status: &str, detail: Option<&str>) {
    let icon = match status {
        "DONE" | "VERIFIED" | "GREEN" => theme.success(Symbols::CHECK),
        "FAIL" | "RED" | "REJECTED" => theme.error(Symbols::CROSS),
        "WARN" => theme.warning(Symbols::WARN),
        _ => theme.info(Symbols::BULLET),
    };
    if let Some(d) = detail {
        println!("  {icon} {: <10} : {}", step, d);
    } else {
        println!("  {icon} {step}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_styled_respects_enabled_flag() {
        let enabled = Theme {
            color_enabled: true,
        };
        let disabled = Theme {
            color_enabled: false,
        };

        let s1 = format!("{}", enabled.success("ok"));
        let s2 = format!("{}", disabled.success("ok"));

        assert!(s1.contains("\x1b[1;32m"));
        assert!(s1.contains("\x1b[0m"));
        assert_eq!(s2, "ok");
    }

    #[test]
    fn test_theme_methods_produce_expected_styles() {
        let theme = Theme {
            color_enabled: true,
        };
        assert!(format!("{}", theme.brand("brand")).contains("\x1b[1;36m"));
        assert!(format!("{}", theme.error("err")).contains("\x1b[1;31m"));
        assert!(format!("{}", theme.warning("warn")).contains("\x1b[1;33m"));
        assert!(format!("{}", theme.info("info")).contains("\x1b[1;34m"));
        assert!(format!("{}", theme.bold("bold")).contains("\x1b[1m"));
        assert!(format!("{}", theme.muted("dim")).contains("\x1b[2m"));
    }
}
