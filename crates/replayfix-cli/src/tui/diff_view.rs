//! Interactive Terminal Diff Reviewer Component.
//!
//! Provides a keyboard-driven, virtualized diff viewer with syntax highlighting,
//! scrollable viewport bounding, and explicit human confirmation controls.

use crate::approval::{highlight_diff, parse_diff_stats};
use crate::tui::screen::ScreenGuard;
use crate::ui::Theme;
use crossterm::cursor::MoveTo;
use crossterm::event::{KeyCode, KeyEvent};
use crossterm::terminal::{Clear, ClearType};
use crossterm::QueueableCommand;
use replayfix_core::digest::sha256_hex;
use std::io::{self, stdout, Write};

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
    lines: Vec<&'a str>,
    files: Vec<String>,
    insertions: usize,
    deletions: usize,
    patch_sha256: String,
    scroll_offset: usize,
}

impl<'a> DiffReviewer<'a> {
    #[must_use]
    pub fn new(diff: &'a str) -> Self {
        let (files, insertions, deletions) = parse_diff_stats(diff);
        let patch_sha256 = sha256_hex(diff.as_bytes());
        let lines: Vec<&'a str> = diff.lines().collect();

        Self {
            diff,
            lines,
            files,
            insertions,
            deletions,
            patch_sha256,
            scroll_offset: 0,
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
                Ok(KeyEvent { code, .. }) => match code {
                    KeyCode::Char('q') | KeyCode::Esc => return DiffReviewOutcome::Aborted,
                    KeyCode::Char('r') => return DiffReviewOutcome::Rejected,
                    KeyCode::Char('a') | KeyCode::Char('y') | KeyCode::Enter => {
                        return DiffReviewOutcome::Approved;
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if self.scroll_offset + 1 < self.lines.len() {
                            self.scroll_offset += 1;
                        }
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        if self.scroll_offset > 0 {
                            self.scroll_offset -= 1;
                        }
                    }
                    KeyCode::PageDown | KeyCode::Char('d') | KeyCode::Char(' ') => {
                        let page_step = (rows as usize).saturating_sub(6).max(1);
                        self.scroll_offset = (self.scroll_offset + page_step)
                            .min(self.lines.len().saturating_sub(1));
                    }
                    KeyCode::PageUp | KeyCode::Char('u') => {
                        let page_step = (rows as usize).saturating_sub(6).max(1);
                        self.scroll_offset = self.scroll_offset.saturating_sub(page_step);
                    }
                    KeyCode::Home => {
                        self.scroll_offset = 0;
                    }
                    KeyCode::End => {
                        let view_height = (rows as usize).saturating_sub(6).max(1);
                        self.scroll_offset = self.lines.len().saturating_sub(view_height);
                    }
                    _ => {}
                },
                Err(_) => return DiffReviewOutcome::Aborted,
            }
        }
    }

    fn render_frame(&self, cols: u16, rows: u16, theme: &Theme) {
        let mut out = stdout();
        let _ = out.queue(Clear(ClearType::All));
        let _ = out.queue(MoveTo(0, 0));

        let width = cols as usize;
        let short_sha = self.patch_sha256.get(..12).unwrap_or(&self.patch_sha256);

        // Header bar
        println!(
            "{}",
            theme.bold(&format!(
                " ReplayFix Candidate Patch Review // SHA: {} (+{} -{}) ",
                short_sha, self.insertions, self.deletions
            ))
        );
        println!(
            " Files: {} | Lines: {} | Scroll: {}/{}",
            self.files.join(", "),
            self.lines.len(),
            self.scroll_offset + 1,
            self.lines.len()
        );
        println!("{}", theme.muted(&"─".repeat(width)));

        // Viewport content
        let available_height = (rows as usize).saturating_sub(5);
        let end_idx = (self.scroll_offset + available_height).min(self.lines.len());

        for line in &self.lines[self.scroll_offset..end_idx] {
            if line.starts_with('+') && !line.starts_with("+++") {
                println!("{}", theme.success(line));
            } else if line.starts_with('-') && !line.starts_with("---") {
                println!("{}", theme.error(line));
            } else if line.starts_with("@@") {
                println!("{}", theme.brand(line));
            } else if line.starts_with("+++") || line.starts_with("---") {
                println!("{}", theme.bold(line));
            } else {
                println!("{line}");
            }
        }

        // Pad blank lines if content is shorter than viewport
        for _ in 0..available_height.saturating_sub(end_idx - self.scroll_offset) {
            println!();
        }

        // Footer toolbar
        println!("{}", theme.muted(&"─".repeat(width)));
        print!(" [a: Approve & Apply]  [r: Reject]  [j/k: Scroll]  [Space: Page]  [q: Abort] ");
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

    #[test]
    fn test_large_diff_viewport_bounding_2000_lines() {
        let mut large_diff = String::from("--- a/large.js\n+++ b/large.js\n@@ -1,10 +1,2000 @@\n");
        for i in 0..2000 {
            large_diff.push_str(&format!("+ console.log('line {i}');\n"));
        }
        let reviewer = DiffReviewer::new(&large_diff);
        assert_eq!(reviewer.lines.len(), 2003);
        assert_eq!(reviewer.insertions, 2000);
        assert_eq!(reviewer.deletions, 0);
        assert_eq!(reviewer.files, vec!["large.js"]);
    }
}
