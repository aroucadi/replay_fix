//! Interactive Terminal Diff Reviewer Component (`tui/diff_view.rs`).
//!
//! Provides a keyboard-driven, virtualized diff viewer with 24-bit Truecolor syntax
//! highlighting, line-number gutters, rounded Unicode box geometry, Sticky Dock command pills,
//! and double-buffered differential rendering (zero `Clear(All)` full-screen blink).

use crate::approval::{highlight_diff, parse_diff_stats, sanitize_ansi};
use crate::tui::layout::{style_diff_token_line, DifferentialRenderer};
use crate::tui::screen::ScreenGuard;
use crate::ui::{CyberPalette, Symbols, Theme};
use crossterm::cursor::MoveTo;
use crossterm::event::{KeyCode, KeyEvent};
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

/// Interactive Diff Reviewer widget with differential frame rendering.
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
    renderer: DifferentialRenderer,
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
            renderer: DifferentialRenderer::new(),
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
                let page_step = usize::from(rows).saturating_sub(6).max(1);
                self.scroll_offset =
                    (self.scroll_offset + page_step).min(self.lines.len().saturating_sub(1));
                None
            }
            KeyCode::PageUp | KeyCode::Char('u') => {
                let page_step = usize::from(rows).saturating_sub(6).max(1);
                self.scroll_offset = self.scroll_offset.saturating_sub(page_step);
                None
            }
            KeyCode::Home => {
                self.scroll_offset = 0;
                None
            }
            KeyCode::End => {
                let view_height = usize::from(rows).saturating_sub(6).max(1);
                self.scroll_offset = self.lines.len().saturating_sub(view_height);
                None
            }
            _ => None,
        }
    }

    /// Launch interactive review loop using double-buffered differential updates.
    pub fn run_interactive(&mut self, theme: &Theme) -> DiffReviewOutcome {
        let _guard = match ScreenGuard::enter_alternate() {
            Ok(g) => g,
            Err(_) => return self.fallback_prompt(theme),
        };

        self.renderer.invalidate();
        loop {
            let (cols, rows) = ScreenGuard::get_size();
            self.render_frame(cols, rows, theme);

            match ScreenGuard::read_key_or_resize() {
                Ok(Some(KeyEvent { code, .. })) => {
                    if let Some(outcome) = self.handle_key(code, rows) {
                        return outcome;
                    }
                }
                Ok(None) => {
                    self.renderer.invalidate();
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
                "Terminal too small (minimum {}x{} required; current {}x{})
{} {} {}
",
                MIN_TERMINAL_COLS,
                MIN_TERMINAL_ROWS,
                cols,
                rows,
                theme.command_pill("a: Approve"),
                theme.command_pill("r: Reject"),
                theme.command_pill("q: Abort")
            );
        }

        let mut buf = String::with_capacity(4096);
        let width = usize::from(cols);
        let inner_width = width.saturating_sub(2);
        let short_sha = self.patch_sha256.get(..12).unwrap_or(&self.patch_sha256);
        let focus_pill = match self.focus {
            ReviewFocus::DiffViewport => {
                theme.pill("DIFF VIEWPORT", CyberPalette::TITANIUM, CyberPalette::CYAN_BG)
            }
            ReviewFocus::ActionControls => theme.pill(
                "ACTION CONTROLS",
                CyberPalette::TITANIUM,
                CyberPalette::VIOLET_BG,
            ),
        };

        let top_border = format!(
            "{}{}{}",
            Symbols::BOX_TL,
            Symbols::BOX_H.repeat(inner_width),
            Symbols::BOX_TR
        );
        buf.push_str(&format!("{}
", theme.muted(&top_border)));
        buf.push_str(&format!(
            "{} {} (+{} -{}) {} Files: {} | Scroll: {}/{}
",
            theme.muted(Symbols::BOX_V),
            theme.bold(&format!("ReplayFix Patch Review // {short_sha}")),
            self.insertions,
            self.deletions,
            focus_pill,
            self.files.join(", "),
            self.scroll_offset + 1,
            self.lines.len().max(1)
        ));
        buf.push_str(&format!(
            "{}
",
            theme.muted(&format!("├{}┤", Symbols::BOX_H.repeat(inner_width)))
        ));

        let available_height = usize::from(rows).saturating_sub(5);
        let end_idx = (self.scroll_offset + available_height).min(self.lines.len());

        for (offset, line) in self.lines[self.scroll_offset..end_idx].iter().enumerate() {
            let line_no = self.scroll_offset + offset + 1;
            let gutter = format!("{line_no:>4} {} ", Symbols::BOX_V);
            if theme.color_enabled {
                buf.push_str(&format!(
                    "{}{}
",
                    theme.muted(&gutter),
                    style_diff_token_line(line)
                ));
            } else {
                buf.push_str(&format!("{gutter}{line}
"));
            }
        }

        for _ in 0..available_height.saturating_sub(end_idx - self.scroll_offset) {
            buf.push('\n');
        }

        let bot_border = format!(
            "{}{}{}",
            Symbols::BOX_BL,
            Symbols::BOX_H.repeat(inner_width),
            Symbols::BOX_BR
        );
        buf.push_str(&format!("{}
", theme.muted(&bot_border)));
        buf.push_str(&format!(
            " {}  {}  {}  {}  {} ",
            theme.command_pill("Enter: Approve"),
            theme.command_pill("Tab: Cycle"),
            theme.command_pill("j/k: Scroll"),
            theme.command_pill("r: Reject"),
            theme.command_pill("q: Abort")
        ));
        buf
    }

    fn render_frame(&mut self, cols: u16, rows: u16, theme: &Theme) {
        let mut out = stdout();
        let _ = out.queue(MoveTo(0, 0));
        let frame = self.format_frame(cols, rows, theme);
        let diff_payload = self.renderer.diff_lines(&frame);
        if !diff_payload.is_empty() {
            print!("{diff_payload}");
            let _ = out.flush();
        }
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
        let mut large_diff = String::from("--- a/large.js
+++ b/large.js
@@ -1,10 +1,2000 @@
");
        for i in 0..2000 {
            large_diff.push_str(&format!("+ console.log('line {i}');
"));
        }
        let mut reviewer = DiffReviewer::new(&large_diff);
        assert_eq!(reviewer.lines.len(), 2003);
        assert_eq!(reviewer.insertions, 2000);
        assert_eq!(reviewer.deletions, 0);
        assert_eq!(reviewer.files, vec!["large.js"]);

        let theme = Theme::for_stdout(ColorOption::Never, OutputMode::Human);
        let frame = reviewer.format_frame(80, 24, &theme);
        assert!(frame.lines().count() <= 24);
        assert!(frame.contains('╭'));
        assert!(frame.contains('╰'));

        reviewer.handle_key(KeyCode::End, 24);
        assert!(reviewer.scroll_offset() > 1900);
    }

    #[test]
    fn test_diff_reviewer_neutralizes_ansi_escape_injection() {
        let malicious =
            "--- a/app.js
+++ b/app.js
@@ -1,1 +1,1 @@
+\x1b[2J\x1b]50;pwned\x07const x = 1;
";
        let reviewer = DiffReviewer::new(malicious);
        let theme = Theme::for_stdout(ColorOption::Never, OutputMode::Human);
        let frame = reviewer.format_frame(80, 24, &theme);
        assert!(!frame.contains('\x1b'));
        assert!(frame.contains("^["));
    }

    #[test]
    fn test_diff_reviewer_tab_focus_and_small_terminal_guard() {
        let diff = "--- a/app.js
+++ b/app.js
@@ -1,1 +1,1 @@
+const ok = true;
";
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
