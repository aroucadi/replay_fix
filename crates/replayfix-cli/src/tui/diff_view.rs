//! Interactive Terminal Diff Reviewer Component.
//!
//! Provides a keyboard-driven, virtualized diff viewer with syntax highlighting,
//! scrollable viewport bounding, and explicit human confirmation controls.

use crate::approval::{highlight_diff, parse_diff_stats, sanitize_ansi};
use crate::tui::screen::ScreenGuard;
use crate::ui::Theme;
use crossterm::cursor::MoveTo;
use crossterm::event::{KeyCode, KeyEvent};
use crossterm::terminal::{Clear, ClearType};
use crossterm::QueueableCommand;
use replayfix_core::digest::sha256_hex;
use std::io::{self, stdout, Write};

/// Minimum terminal dimensions required for full interactive diff layout.
pub const MIN_TERMINAL_COLS: u16 = 80;
pub const MIN_TERMINAL_ROWS: u16 = 24;

/// Active focus panel inside the interactive diff review workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewFocus {
    DiffViewport,
    ActionControls,
}

impl ReviewFocus {
    /// Cycle focus to the next panel on `Tab`.
    #[must_use]
    pub const fn cycle(self) -> Self {
        match self {
            Self::DiffViewport => Self::ActionControls,
            Self::ActionControls => Self::DiffViewport,
        }
    }
}

/// Interactive diff review outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffReviewOutcome {
    Approved,
    Rejected,
    Aborted,
}

/// Interactive Diff Reviewer widget.
#[derive(Debug)]
pub struct DiffReviewer<'a> {
    diff: &'a str,
    lines: Vec<String>,
    files: Vec<String>,
    insertions: usize,
    deletions: usize,
    patch_sha256: String,
    scroll_offset: usize,
    focus: ReviewFocus,
}

impl<'a> DiffReviewer<'a> {
    #[must_use]
    pub fn new(diff: &'a str) -> Self {
        let (files, insertions, deletions) = parse_diff_stats(diff);
        let patch_sha256 = sha256_hex(diff.as_bytes());
        let lines: Vec<String> = diff.lines().map(sanitize_ansi).collect();

        Self {
            diff,
            lines,
            files,
            insertions,
            deletions,
            patch_sha256,
            scroll_offset: 0,
            focus: ReviewFocus::DiffViewport,
        }
    }

    /// Return current focus state.
    #[must_use]
    pub const fn focus(&self) -> ReviewFocus {
        self.focus
    }

    /// Return current scroll offset.
    #[must_use]
    pub const fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    /// Process a key event deterministically without requiring a real terminal loop.
    pub fn handle_key(&mut self, code: KeyCode, rows: u16) -> Option<DiffReviewOutcome> {
        match code {
            KeyCode::Tab | KeyCode::BackTab => {
                self.focus = self.focus.cycle();
                None
            }
            KeyCode::Char('q') | KeyCode::Esc => Some(DiffReviewOutcome::Aborted),
            KeyCode::Char('r') => Some(DiffReviewOutcome::Rejected),
            KeyCode::Char('a') | KeyCode::Char('y') | KeyCode::Enter => {
                Some(DiffReviewOutcome::Approved)
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.scroll_offset + 1 < self.lines.len() {
                    self.scroll_offset += 1;
                }
                None
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.scroll_offset > 0 {
                    self.scroll_offset -= 1;
                }
                None
            }
            KeyCode::PageDown | KeyCode::Char('d') | KeyCode::Char(' ') => {
                let page_step = (rows as usize).saturating_sub(6).max(1);
                self.scroll_offset =
                    (self.scroll_offset + page_step).min(self.lines.len().saturating_sub(1));
                None
            }
            KeyCode::PageUp | KeyCode::Char('u') => {
                let page_step = (rows as usize).saturating_sub(6).max(1);
                self.scroll_offset = self.scroll_offset.saturating_sub(page_step);
                None
            }
            KeyCode::Home => {
                self.scroll_offset = 0;
                None
            }
            KeyCode::End => {
                let view_height = (rows as usize).saturating_sub(6).max(1);
                self.scroll_offset = self.lines.len().saturating_sub(view_height);
                None
            }
            _ => None,
        }
    }

    /// Launch interactive review loop.
    pub fn run_interactive(&mut self, theme: &Theme) -> DiffReviewOutcome {
        let _guard = match ScreenGuard::enter_alternate() {
            Ok(g) => g,
            Err(_) => return self.fallback_prompt(theme),
        };

        loop {
            let (cols, rows) = ScreenGuard::get_size();
            self.render_frame(cols, rows, theme);

            match ScreenGuard::read_key() {
                Ok(KeyEvent { code, .. }) => {
                    if let Some(outcome) = self.handle_key(code, rows) {
                        return outcome;
                    }
                }
                Err(_) => return DiffReviewOutcome::Aborted,
            }
        }
    }

    /// Render a single frame into a string buffer for deterministic inspection and terminal output.
    #[must_use]
    pub fn format_frame(&self, cols: u16, rows: u16, theme: &Theme) -> String {
        if cols < MIN_TERMINAL_COLS || rows < MIN_TERMINAL_ROWS {
            return format!(
                "Terminal too small (minimum {}x{} required; current {}x{})\n[a: Approve] [r: Reject] [q: Abort]\n",
                MIN_TERMINAL_COLS, MIN_TERMINAL_ROWS, cols, rows
            );
        }

        let mut buf = String::with_capacity(4096);
        let width = cols as usize;
        let short_sha = self.patch_sha256.get(..12).unwrap_or(&self.patch_sha256);
        let focus_badge = match self.focus {
            ReviewFocus::DiffViewport => "[FOCUS: DIFF VIEWPORT]",
            ReviewFocus::ActionControls => "[FOCUS: ACTION CONTROLS]",
        };

        buf.push_str(&format!(
            "{}\n",
            theme.bold(&format!(
                " ReplayFix Candidate Patch Review // SHA: {} (+{} -{}) {} ",
                short_sha, self.insertions, self.deletions, focus_badge
            ))
        ));
        buf.push_str(&format!(
            " Files: {} | Lines: {} | Scroll: {}/{}\n",
            self.files.join(", "),
            self.lines.len(),
            self.scroll_offset + 1,
            self.lines.len().max(1)
        ));
        buf.push_str(&format!("{}\n", theme.muted(&"─".repeat(width))));

        let available_height = (rows as usize).saturating_sub(5);
        let end_idx = (self.scroll_offset + available_height).min(self.lines.len());

        for line in &self.lines[self.scroll_offset..end_idx] {
            if line.starts_with('+') && !line.starts_with("+++") {
                buf.push_str(&format!("{}\n", theme.success(line)));
            } else if line.starts_with('-') && !line.starts_with("---") {
                buf.push_str(&format!("{}\n", theme.error(line)));
            } else if line.starts_with("@@") {
                buf.push_str(&format!("{}\n", theme.brand(line)));
            } else if line.starts_with("+++") || line.starts_with("---") {
                buf.push_str(&format!("{}\n", theme.bold(line)));
            } else {
                buf.push_str(line);
                buf.push('\n');
            }
        }

        for _ in 0..available_height.saturating_sub(end_idx - self.scroll_offset) {
            buf.push('\n');
        }

        buf.push_str(&format!("{}\n", theme.muted(&"─".repeat(width))));
        buf.push_str(
            " [Tab: Focus]  [a: Approve & Apply]  [r: Reject]  [j/k: Scroll]  [Space: Page]  [q: Abort] ",
        );
        buf
    }

    fn render_frame(&self, cols: u16, rows: u16, theme: &Theme) {
        let mut out = stdout();
        let _ = out.queue(Clear(ClearType::All));
        let _ = out.queue(MoveTo(0, 0));
        let frame = self.format_frame(cols, rows, theme);
        print!("{frame}");
        let _ = out.flush();
    }

    /// Non-alternate fallback for simple pipes or terminal errors.
    fn fallback_prompt(&self, _theme: &Theme) -> DiffReviewOutcome {
        println!();
        println!("{}", highlight_diff(self.diff));
        println!(
            "Files: {} | Insertions (+): {} | Deletions (-): {}",
            self.files.join(", "),
            self.insertions,
            self.deletions
        );
        print!("Apply candidate patch to base repository? [y/N]: ");
        let _ = io::stdout().flush();

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            return DiffReviewOutcome::Rejected;
        }

        let choice = input.trim().to_lowercase();
        if choice == "y" || choice == "yes" {
            DiffReviewOutcome::Approved
        } else {
            DiffReviewOutcome::Rejected
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::{ColorOption, OutputMode};

    #[test]
    fn test_large_diff_viewport_bounding_2000_lines() {
        let mut large_diff = String::from("--- a/large.js\n+++ b/large.js\n@@ -1,10 +1,2000 @@\n");
        for i in 0..2000 {
            large_diff.push_str(&format!("+ console.log('line {i}');\n"));
        }
        let mut reviewer = DiffReviewer::new(&large_diff);
        assert_eq!(reviewer.lines.len(), 2003);
        assert_eq!(reviewer.insertions, 2000);
        assert_eq!(reviewer.deletions, 0);
        assert_eq!(reviewer.files, vec!["large.js"]);

        let theme = Theme::for_stdout(ColorOption::Never, OutputMode::Human);
        let frame = reviewer.format_frame(80, 24, &theme);
        assert!(frame.lines().count() <= 24);

        reviewer.handle_key(KeyCode::End, 24);
        assert!(reviewer.scroll_offset() > 1900);
    }

    #[test]
    fn test_diff_reviewer_neutralizes_ansi_escape_injection() {
        let malicious = "--- a/app.js\n+++ b/app.js\n@@ -1,1 +1,1 @@\n+\x1b[2J\x1b]50;pwned\x07const x = 1;\n";
        let reviewer = DiffReviewer::new(malicious);
        let theme = Theme::for_stdout(ColorOption::Never, OutputMode::Human);
        let frame = reviewer.format_frame(80, 24, &theme);
        assert!(!frame.contains('\x1b'));
        assert!(frame.contains("^["));
    }

    #[test]
    fn test_diff_reviewer_tab_focus_and_small_terminal_guard() {
        let diff = "--- a/app.js\n+++ b/app.js\n@@ -1,1 +1,1 @@\n+const ok = true;\n";
        let mut reviewer = DiffReviewer::new(diff);
        assert_eq!(reviewer.focus(), ReviewFocus::DiffViewport);
        reviewer.handle_key(KeyCode::Tab, 24);
        assert_eq!(reviewer.focus(), ReviewFocus::ActionControls);
        reviewer.handle_key(KeyCode::Tab, 24);
        assert_eq!(reviewer.focus(), ReviewFocus::DiffViewport);

        let theme = Theme::for_stdout(ColorOption::Never, OutputMode::Human);
        let small_frame = reviewer.format_frame(60, 15, &theme);
        assert!(small_frame.contains("Terminal too small (minimum 80x24 required"));
    }
}
