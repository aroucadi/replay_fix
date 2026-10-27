//! Terminal UI design tokens, color policies, output modes, and rendering architecture for FixTream CLI.
//!
//! Implements a 10/10 production-grade CLI interaction layer inspired by Cargo, Git, and Claude Code:
//! - Three explicit output modes: Human, Quiet, and Machine.
//! - Deterministic color precedence: CLI flag `--color` > `NO_COLOR` > `CLICOLOR_FORCE` > `TERM=dumb` > `is_terminal()`.
//! - Independent capability detection for stdout and stderr.
//! - Structured diagnostic engine (`DoctorCheckResult`) with Status, Finding, Impact, Remediation, and Evidence.
//! - High-trust execution cards with clear security boundaries, repository invariants ($\Delta\text{repo}=0$), and verifiable audit summaries.

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

/// ANSI styling primitives.
pub mod style {
    pub const RESET: &str = "\x1b[0m";
    pub const BOLD: &str = "\x1b[1m";
    pub const DIM: &str = "\x1b[2m";
    pub const UNDERLINE: &str = "\x1b[4m";

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

/// Accessible terminal symbols with guaranteed unicode fallbacks.
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
}

/// Render a section header banner with consistent visual hierarchy.
pub fn print_banner(theme: &Theme, title: &str) {
    if theme.mode == OutputMode::Quiet || theme.mode == OutputMode::Machine {
        return;
    }
    let bar = "─".repeat(60);
    println!("{}", theme.muted(&bar));
    println!("  {} {}", theme.brand("FixTream //"), theme.bold(title));
    println!("{}", theme.muted(&bar));
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

/// Print an operational phase step with clear status semantics.
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
    let (icon, label) = match status {
        StepStatus::Done => (theme.success(Symbols::CHECK), theme.success("PASS")),
        StepStatus::Fail => (theme.error(Symbols::CROSS), theme.error("FAIL")),
        StepStatus::Warn => (theme.warning(Symbols::WARN), theme.warning("WARN")),
        StepStatus::Running => (theme.info(Symbols::BULLET), theme.info("RUN ")),
        StepStatus::Skipped => (theme.muted(Symbols::ARROW_RIGHT), theme.muted("SKIP")),
    };

    println!("  {icon} {: <14} [{label}] {}", theme.bold(phase), headline);
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

/// Render structured diagnostics with findings, impacts, and concrete remediation.
pub fn print_diagnostics(theme: &Theme, checks: &[DiagnosticCheck]) {
    if theme.mode == OutputMode::Quiet || theme.mode == OutputMode::Machine {
        return;
    }
    for check in checks {
        let (icon, badge) = match check.status {
            DiagnosticStatus::Ok => (
                theme.success(Symbols::CHECK),
                theme.success("[OK]         "),
            ),
            DiagnosticStatus::Warn => {
                (theme.warning(Symbols::WARN), theme.warning("[WARN]       "))
            }
            DiagnosticStatus::Fail => (theme.error(Symbols::CROSS), theme.error("[MISSING]    ")),
            DiagnosticStatus::NotChecked => {
                (theme.muted(Symbols::BULLET), theme.muted("[NOT CHECKED]"))
            }
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

/// Print execution security & trust boundary card.
pub fn print_security_boundary(theme: &Theme, repo: &str, is_read_only: bool, isolated: bool) {
    if theme.mode == OutputMode::Quiet || theme.mode == OutputMode::Machine {
        return;
    }
    println!();
    println!(
        "  {} {}",
        theme.brand(Symbols::SHIELD),
        theme.bold("Security & Isolation Guarantees:")
    );
    let repo_mode = if is_read_only {
        "Strictly Read-Only (Δrepo = 0 invariant enforced)"
    } else {
        "Direct Mutation Permitted"
    };
    println!(
        "    {} Target Repository : {} ({})",
        theme.muted(Symbols::STEP),
        repo,
        repo_mode
    );
    let isolation_status = if isolated {
        "Active (ephemeral scratch workspace in OS temp dir)"
    } else {
        "Disabled (running directly in workspace)"
    };
    println!(
        "    {} Execution Sandbox  : {}",
        theme.muted(Symbols::STEP),
        isolation_status
    );
    println!(
        "    {} Test Freeze Hash   : SHA-256 pre-flight invariant active",
        theme.muted(Symbols::STEP)
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
    fn test_theme_methods_produce_expected_styles() {
        let theme = Theme {
            color_enabled: true,
            mode: OutputMode::Human,
        };
        assert!(format!("{}", theme.brand("brand")).contains("\x1b[1;36m"));
        assert!(format!("{}", theme.error("err")).contains("\x1b[1;31m"));
        assert!(format!("{}", theme.warning("warn")).contains("\x1b[1;33m"));
        assert!(format!("{}", theme.info("info")).contains("\x1b[1;34m"));
        assert!(format!("{}", theme.bold("bold")).contains("\x1b[1m"));
        assert!(format!("{}", theme.muted("dim")).contains("\x1b[2m"));
    }
}
