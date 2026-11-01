//! Interactive Modal Dialogs & Menus.

use crate::ui::Theme;
use std::io::{self, BufRead, Write};

/// Render a structured visual card in the terminal.
pub fn render_card(title: &str, body_lines: &[String], theme: &Theme) {
    println!();
    let bar = "─".repeat(60);
    println!("  {}", theme.muted(&bar));
    println!("  {} {}", theme.brand("◈ "), theme.bold(title));
    println!("  {}", theme.muted(&bar));
    for line in body_lines {
        println!("    {line}");
    }
    println!("  {}", theme.muted(&bar));
}

/// Prompt user to choose from a list of options.
pub fn prompt_selection(prompt: &str, options: &[String], theme: &Theme) -> Option<usize> {
    println!();
    println!("  {} {}", theme.brand("?"), theme.bold(prompt));
    for (i, opt) in options.iter().enumerate() {
        println!("    {} {}", theme.muted(&format!("{}. ", i + 1)), opt);
    }
    print!("  Enter choice (1-{}) or 'q' to cancel: ", options.len());
    let _ = io::stdout().flush();

    let mut input = String::new();
    if io::stdin().lock().read_line(&mut input).is_err() {
        return None;
    }

    let trimmed = input.trim();
    if trimmed.eq_ignore_ascii_case("q") || trimmed.is_empty() {
        return None;
    }

    if let Ok(idx) = trimmed.parse::<usize>() {
        if idx >= 1 && idx <= options.len() {
            return Some(idx - 1);
        }
    }

    None
}
