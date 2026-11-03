//! Bento Terminal Layout & Double-Buffered Differential Renderer (`tui/layout.rs`).
//!
//! Implements an `80x24+` responsive Bento Command Deck with ratatui-style differential
//! cell buffering:
//! - **Header Deck**: Active repository, branch badge (`󰊢`), ephemeral port (`:0`), and token rate meter (`tok/s`).
//! - **Dual-Pane Body**:
//!   * **Left Pane**: Live Agent Thought Stream & Tool Event Log with Nerd Font glyphs (`󰈚` spec, `⚡` spark, `󰒓` core).
//!   * **Right Pane**: Virtualized Diff Reviewer with line-number gutters, hunk headers (`@@`), and 24-bit syntax token coloring.
//! - **Sticky Dock**: Inverted background command pills for hotkeys (` Enter: Approve `, ` Tab: Cycle `, ` j/k: Scroll `).

use crate::approval::sanitize_ansi;
use crate::tui::brand::render_compact_hud_emblem;
use crate::tui::motion::{render_fractional_bar, render_wave_shimmer};
use crate::ui::{style, CyberPalette, Rgb, Symbols, Theme};

/// Category of an entry in the Live Agent Thought Stream & Tool Event Log.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThoughtGlyph {
    /// `󰈚` Specification / Reproduction artifact
    Spec,
    /// `⚡` Muse Spark / Multimodal AI Deliberation
    Spark,
    /// `󰒓` Deterministic Core Substrate Gate / Sandbox
    Core,
}

impl ThoughtGlyph {
    #[must_use]
    pub const fn icon(self) -> &'static str {
        match self {
            Self::Spec => Symbols::SPEC,
            Self::Spark => Symbols::SPARK,
            Self::Core => Symbols::CORE,
        }
    }

    #[must_use]
    pub const fn color(self) -> Rgb {
        match self {
            Self::Spec => CyberPalette::CYAN,
            Self::Spark => CyberPalette::VIOLET,
            Self::Core => CyberPalette::EMERALD,
        }
    }
}

/// A single entry in the Left Pane Live Agent Thought Stream & Tool Event Log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThoughtEvent {
    pub glyph: ThoughtGlyph,
    pub label: String,
    pub detail: String,
    pub active: bool,
}

impl ThoughtEvent {
    #[must_use]
    pub fn new(
        glyph: ThoughtGlyph,
        label: impl Into<String>,
        detail: impl Into<String>,
        active: bool,
    ) -> Self {
        Self {
            glyph,
            label: label.into(),
            detail: detail.into(),
            active,
        }
    }
}

/// Styling attributes for a cell or text run in a `FrameBuffer`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellStyle {
    pub fg: Rgb,
    pub bg: Option<Rgb>,
    pub bold: bool,
}

impl CellStyle {
    #[must_use]
    pub const fn new(fg: Rgb, bg: Option<Rgb>, bold: bool) -> Self {
        Self { fg, bg, bold }
    }
}

/// Single terminal grid cell for ratatui-style double-buffered differential rendering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cell {
    pub ch: char,
    pub fg: Rgb,
    pub bg: Option<Rgb>,
    pub bold: bool,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            ch: ' ',
            fg: CyberPalette::TITANIUM,
            bg: None,
            bold: false,
        }
    }
}

/// 2D terminal cell grid buffer (`cols x rows`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameBuffer {
    cols: u16,
    rows: u16,
    cells: Vec<Cell>,
}

impl FrameBuffer {
    #[must_use]
    pub fn new(cols: u16, rows: u16) -> Self {
        let len = usize::from(cols).saturating_mul(usize::from(rows));
        Self {
            cols,
            rows,
            cells: vec![Cell::default(); len],
        }
    }

    #[must_use]
    pub const fn cols(&self) -> u16 {
        self.cols
    }

    #[must_use]
    pub const fn rows(&self) -> u16 {
        self.rows
    }

    /// Set a single cell at `(col, row)` if in bounds.
    pub fn set(&mut self, col: u16, row: u16, cell: Cell) {
        if col < self.cols && row < self.rows {
            let idx = usize::from(row)
                .saturating_mul(usize::from(self.cols))
                .saturating_add(usize::from(col));
            if let Some(slot) = self.cells.get_mut(idx) {
                *slot = cell;
            }
        }
    }

    /// Write a styled plain string horizontally starting at `(col, row)`, clipping at `max_width`.
    pub fn put_str(
        &mut self,
        col: u16,
        row: u16,
        text: &str,
        cell_style: CellStyle,
        max_width: u16,
    ) {
        if row >= self.rows || col >= self.cols {
            return;
        }
        let limit = self.cols.min(col.saturating_add(max_width));
        for (offset, ch) in text.chars().enumerate() {
            #[allow(clippy::cast_possible_truncation)]
            let target_col = col.saturating_add(offset as u16);
            if target_col >= limit {
                break;
            }
            self.set(
                target_col,
                row,
                Cell {
                    ch,
                    fg: cell_style.fg,
                    bg: cell_style.bg,
                    bold: cell_style.bold,
                },
            );
        }
    }

    /// Get immutable reference to cell at `(col, row)`.
    #[must_use]
    pub fn get(&self, col: u16, row: u16) -> Option<&Cell> {
        if col < self.cols && row < self.rows {
            let idx = usize::from(row)
                .saturating_mul(usize::from(self.cols))
                .saturating_add(usize::from(col));
            self.cells.get(idx)
        } else {
            None
        }
    }
}

/// Double-buffered differential terminal renderer.
///
/// Compares consecutive `FrameBuffer` instances (or line buffers) and emits ANSI
/// cursor positioning (`\x1b[{row};{col}H`) and 24-bit Truecolor sequences only for
/// mutated cells/rows, completely eliminating `Clear(ClearType::All)` full-screen blink.
#[derive(Debug, Default, Clone)]
pub struct DifferentialRenderer {
    prev_grid: Option<FrameBuffer>,
    prev_lines: Vec<String>,
}

impl DifferentialRenderer {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            prev_grid: None,
            prev_lines: Vec::new(),
        }
    }

    /// Reset cached previous buffers (e.g. after terminal resize).
    pub fn invalidate(&mut self) {
        self.prev_grid = None;
        self.prev_lines.clear();
    }

    /// Diff `next` cell grid against the previous frame and return minimal ANSI escape sequence.
    ///
    /// Returns an empty string when `next` is identical to `prev_grid`.
    pub fn diff_grid(&mut self, next: &FrameBuffer, color_enabled: bool) -> String {
        let mut out = String::new();
        let same_dims = self
            .prev_grid
            .as_ref()
            .is_some_and(|p| p.cols == next.cols && p.rows == next.rows);

        for row in 0..next.rows {
            for col in 0..next.cols {
                let Some(cell) = next.get(col, row) else {
                    continue;
                };
                let unchanged = same_dims
                    && self
                        .prev_grid
                        .as_ref()
                        .and_then(|p| p.get(col, row))
                        .is_some_and(|prev_cell| prev_cell == cell);

                if unchanged {
                    continue;
                }

                // 1-based ANSI cursor coordinates
                out.push_str(&format!(
                    "\x1b[{};{}H",
                    row.saturating_add(1),
                    col.saturating_add(1)
                ));
                if color_enabled {
                    if cell.bold {
                        out.push_str(style::BOLD);
                    }
                    out.push_str(&cell.fg.fg_ansi());
                    if let Some(bg) = cell.bg {
                        out.push_str(&bg.bg_ansi());
                    }
                    out.push(cell.ch);
                    out.push_str(style::RESET);
                } else {
                    out.push(cell.ch);
                }
            }
        }

        self.prev_grid = Some(next.clone());
        out
    }

    /// Diff line-oriented frame output and emit cursor row jumps (`\x1b[{row};1H`) only for changed lines.
    pub fn diff_lines(&mut self, frame: &str) -> String {
        let next_lines: Vec<String> = frame.lines().map(ToString::to_string).collect();
        let mut out = String::new();

        for (idx, line) in next_lines.iter().enumerate() {
            let unchanged = self.prev_lines.get(idx).is_some_and(|prev| prev == line);
            if !unchanged {
                out.push_str(&format!("\x1b[{};1H{}\x1b[K", idx.saturating_add(1), line));
            }
        }

        if next_lines.len() < self.prev_lines.len() {
            for idx in next_lines.len()..self.prev_lines.len() {
                out.push_str(&format!("\x1b[{};1H\x1b[K", idx.saturating_add(1)));
            }
        }

        self.prev_lines = next_lines;
        out
    }
}

/// State model for the 80x24+ Asymmetric Bento Terminal Command Deck (40% Left / 60% Right).
#[derive(Debug, Clone)]
pub struct BentoCommandDeck {
    pub repo_name: String,
    pub branch: String,
    pub ephemeral_port: String,
    pub token_rate_tps: u32,
    pub progress_ratio: f64,
    pub reasoning_effort: String,
    pub thoughts: Vec<ThoughtEvent>,
    pub diff_lines: Vec<String>,
    pub diff_scroll: usize,
    pub fold_context_hunks: bool,
    pub tick: u64,
}

impl BentoCommandDeck {
    #[must_use]
    pub fn new(repo_name: impl Into<String>, branch: impl Into<String>, diff: &str) -> Self {
        let diff_lines = diff.lines().map(sanitize_ansi).collect();
        Self {
            repo_name: repo_name.into(),
            branch: branch.into(),
            ephemeral_port: ":0".to_string(),
            token_rate_tps: 312,
            progress_ratio: 0.75,
            reasoning_effort: "effort: xhigh".to_string(),
            thoughts: vec![
                ThoughtEvent::new(
                    ThoughtGlyph::Spec,
                    "SpecFreeze",
                    "[TOOL: ffmpeg] Keyframes & SHA-256 test freeze locked",
                    false,
                ),
                ThoughtEvent::new(
                    ThoughtGlyph::Core,
                    "Gate 1 (RED)",
                    "[GATE 1: RED CONFIRMED] Causal defect reproduced on :0",
                    false,
                ),
                ThoughtEvent::new(
                    ThoughtGlyph::Spark,
                    "UNDERSTANDING",
                    "Muse Spark 1.3 (effort: xhigh) synthesizing AST repair",
                    true,
                ),
                ThoughtEvent::new(
                    ThoughtGlyph::Core,
                    "Gate 2 (GREEN)",
                    "[GATE 2: PASS] 2-run deterministic verification ready",
                    false,
                ),
            ],
            diff_lines,
            diff_scroll: 0,
            fold_context_hunks: false,
            tick: 0,
        }
    }

    /// Toggle folding of unchanged context lines inside the Virtualized AST Diff Viewer.
    pub fn toggle_hunk_folding(&mut self) {
        self.fold_context_hunks = !self.fold_context_hunks;
    }

    /// Return the visible diff lines respecting hunk folding.
    #[must_use]
    pub fn visible_diff_lines(&self) -> Vec<(usize, String)> {
        let mut out = Vec::with_capacity(self.diff_lines.len());
        let mut folded_run = 0usize;
        for (idx, line) in self.diff_lines.iter().enumerate() {
            let is_structural = line.starts_with("+++")
                || line.starts_with("---")
                || line.starts_with("@@")
                || line.starts_with('+')
                || line.starts_with('-');
            if self.fold_context_hunks && !is_structural {
                folded_run = folded_run.saturating_add(1);
                if folded_run == 1 {
                    out.push((idx + 1, "⋯ [folded unchanged AST context] ⋯".to_string()));
                }
            } else {
                folded_run = 0;
                out.push((idx + 1, line.clone()));
            }
        }
        out
    }

    /// Render the complete 80x24+ Asymmetric Bento Command Deck frame
    /// (Top Deck, 40% Left Thought Stream / 60% Right Virtualized AST Diff, Bottom Command Ribbon).
    #[must_use]
    pub fn render_frame(&self, cols: u16, rows: u16, theme: &Theme) -> String {
        let width = usize::from(cols.max(80));
        let height = usize::from(rows.max(24));
        let inner_width = width.saturating_sub(2);
        let left_width = (inner_width * 40 / 100).max(28);
        let right_width = inner_width.saturating_sub(left_width + 1);

        let mut out = String::with_capacity(width * height * 2);

        // 1. Top Deck (Branded HUD pill, active git branch `󰊢`, ephemeral port `:0`, and live token rate laser meter)
        let top_border = format!(
            "{}{}{}",
            Symbols::BOX_TL,
            Symbols::BOX_H.repeat(inner_width),
            Symbols::BOX_TR
        );
        out.push_str(&format!("{}\n", theme.muted(&top_border)));

        let hud_emblem = render_compact_hud_emblem(theme.color_enabled);
        let branch_label = format!("{} {}", Symbols::BRANCH, self.branch);
        let branch_badge = theme.pill(
            &branch_label,
            CyberPalette::TITANIUM,
            CyberPalette::VIOLET_BG,
        );
        let port_label = format!("PORT {}", self.ephemeral_port);
        let port_badge = theme.pill(
            &port_label,
            CyberPalette::TITANIUM,
            CyberPalette::CYAN_BG,
        );
        let meter_bar = render_fractional_bar(self.progress_ratio, 8, theme.color_enabled);
        let header_line = format!(
            "{} {} {}  {}  {}  {} {} tok/s {}",
            theme.muted(Symbols::BOX_V),
            hud_emblem,
            theme.bold(&self.repo_name),
            branch_badge,
            port_badge,
            theme.brand(Symbols::SPARK),
            self.token_rate_tps,
            meter_bar
        );
        out.push_str(&format!("{header_line}\n"));

        // Divider between Top Deck and Asymmetric 40%/60% Dual-Pane Body
        let split_top = format!(
            "├{}┬{}┤",
            Symbols::BOX_H.repeat(left_width),
            Symbols::BOX_H.repeat(right_width)
        );
        out.push_str(&format!("{}\n", theme.muted(&split_top)));

        // Pane titles
        let left_hdr = truncate_pad(
            &format!(" THOUGHT STREAM ({})", self.reasoning_effort),
            left_width,
        );
        let right_hdr = truncate_pad(" VIRTUALIZED AST DIFF VIEWER", right_width);
        out.push_str(&format!(
            "{}{}{}{}{}\n",
            theme.muted(Symbols::BOX_V),
            theme.brand(&left_hdr),
            theme.muted(Symbols::BOX_V),
            theme.brand(&right_hdr),
            theme.muted(Symbols::BOX_V)
        ));

        let pane_sep = format!(
            "├{}┼{}┤",
            Symbols::BOX_H.repeat(left_width),
            Symbols::BOX_H.repeat(right_width)
        );
        out.push_str(&format!("{}\n", theme.muted(&pane_sep)));

        // 2. Dual-Pane Body (`height - 8` rows)
        let visible_diff = self.visible_diff_lines();
        let body_rows = height.saturating_sub(8).max(6);
        for row_idx in 0..body_rows {
            let left_cell = self.format_left_thought_row(row_idx, left_width, theme);
            let right_cell = self.format_right_diff_row(&visible_diff, row_idx, right_width, theme);
            out.push_str(&format!(
                "{}{}{}{}{}\n",
                theme.muted(Symbols::BOX_V),
                left_cell,
                theme.muted(Symbols::BOX_V),
                right_cell,
                theme.muted(Symbols::BOX_V)
            ));
        }

        // Divider above Bottom Command Ribbon
        let split_bot = format!(
            "├{}┴{}┤",
            Symbols::BOX_H.repeat(left_width),
            Symbols::BOX_H.repeat(right_width)
        );
        out.push_str(&format!("{}\n", theme.muted(&split_bot)));

        // 3. Bottom Command Ribbon with inverted background hotkey pills
        let pill_enter = theme.command_pill("Enter: Run");
        let pill_tab = theme.command_pill("Tab: Switch");
        let pill_space = theme.command_pill("Space: Page");
        let pill_scroll = theme.command_pill("j/k: Scroll");
        let pill_quit = theme.command_pill("q: Quit");
        out.push_str(&format!(
            "{}  {}  {}  {}  {}  {} {}\n",
            theme.muted(Symbols::BOX_V),
            pill_enter,
            pill_tab,
            pill_space,
            pill_scroll,
            pill_quit,
            theme.muted(Symbols::BOX_V)
        ));

        let bot_border = format!(
            "{}{}{}",
            Symbols::BOX_BL,
            Symbols::BOX_H.repeat(inner_width),
            Symbols::BOX_BR
        );
        out.push_str(&format!("{}", theme.muted(&bot_border)));

        out
    }

    fn format_left_thought_row(&self, row_idx: usize, width: usize, theme: &Theme) -> String {
        let Some(evt) = self.thoughts.get(row_idx) else {
            return " ".repeat(width);
        };
        let raw_text = format!(" {} {}: {}", evt.glyph.icon(), evt.label, evt.detail);
        let fitted = truncate_pad(&raw_text, width);
        if evt.active && theme.color_enabled {
            render_wave_shimmer(
                &fitted,
                self.tick,
                CyberPalette::MUTED_STEEL,
                evt.glyph.color(),
                true,
            )
        } else if theme.color_enabled {
            format!("{}{}{}", evt.glyph.color().fg_ansi(), fitted, style::RESET)
        } else {
            fitted
        }
    }

    fn format_right_diff_row(
        &self,
        visible_diff: &[(usize, String)],
        row_idx: usize,
        width: usize,
        theme: &Theme,
    ) -> String {
        let line_idx = self.diff_scroll.saturating_add(row_idx);
        let Some((orig_line_no, raw_line)) = visible_diff.get(line_idx) else {
            return " ".repeat(width);
        };
        let gutter = format!("{orig_line_no:>4} {} ", Symbols::BOX_V);
        let content_width = width.saturating_sub(gutter.chars().count());
        let body = truncate_pad(raw_line, content_width);

        if !theme.color_enabled {
            return format!("{gutter}{body}");
        }

        let gutter_styled = format!(
            "{}{}{}",
            CyberPalette::MUTED_STEEL.fg_ansi(),
            gutter,
            style::RESET
        );
        let body_styled = style_diff_token_line(&body);
        format!("{gutter_styled}{body_styled}")
    }
}

/// Apply 24-bit Truecolor syntax/diff token coloring to a sanitized diff line.
#[must_use]
pub fn style_diff_token_line(line: &str) -> String {
    if line.starts_with("+++") || line.starts_with("---") {
        format!(
            "\x1b[1m{}{}{}",
            CyberPalette::TITANIUM.fg_ansi(),
            line,
            style::RESET
        )
    } else if line.starts_with("@@") {
        format!(
            "\x1b[1;38;2;56;189;248;48;2;12;74;110m{}{}",
            line,
            style::RESET
        )
    } else if line.starts_with('+') {
        format!(
            "\x1b[38;2;16;185;129;48;2;6;48;36m{}{}",
            line,
            style::RESET
        )
    } else if line.starts_with('-') {
        format!(
            "\x1b[38;2;244;63;94;48;2;68;12;30m{}{}",
            line,
            style::RESET
        )
    } else {
        format!("{}{}{}", CyberPalette::MUTED_STEEL.fg_ansi(), line, style::RESET)
    }
}

fn truncate_pad(s: &str, width: usize) -> String {
    let mut out = String::with_capacity(width);
    let mut count = 0usize;
    for ch in s.chars() {
        if count >= width {
            break;
        }
        out.push(ch);
        count = count.saturating_add(1);
    }
    if count < width {
        out.push_str(&" ".repeat(width.saturating_sub(count)));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::{ColorOption, OutputMode};

    #[test]
    fn test_differential_renderer_emits_zero_bytes_on_identical_frame() {
        let mut fb = FrameBuffer::new(20, 4);
        fb.put_str(
            0,
            0,
            "ReplayFix v1.9.5",
            CellStyle::new(CyberPalette::CYAN, None, true),
            20,
        );

        let mut renderer = DifferentialRenderer::new();
        let first_pass = renderer.diff_grid(&fb, true);
        assert!(!first_pass.is_empty());
        assert!(first_pass.contains("\x1b[1;1H"));

        // Second pass with identical buffer must emit 0 bytes
        let second_pass = renderer.diff_grid(&fb, true);
        assert!(second_pass.is_empty());

        // Mutate a single cell and verify only that coordinate is emitted
        fb.set(
            5,
            2,
            Cell {
                ch: '⚡',
                fg: CyberPalette::EMERALD,
                bg: None,
                bold: true,
            },
        );
        let third_pass = renderer.diff_grid(&fb, true);
        assert!(third_pass.contains("\x1b[3;6H"));
        assert!(!third_pass.contains("\x1b[1;1H"));
    }

    #[test]
    fn test_bento_command_deck_renders_header_panes_and_sticky_dock() {
        let diff = "--- a/src/app.js
+++ b/src/app.js
@@ -1,4 +1,4 @@
 const ctx1 = 1;
 const ctx2 = 2;
-const broken = null;
+const broken = {};
";
        let mut deck = BentoCommandDeck::new("MetaPulse", "main", diff);

        let theme = Theme::for_stdout(ColorOption::Always, OutputMode::Human);
        let rendered = deck.render_frame(110, 24, &theme);

        // Top Deck invariants: Branded HUD emblem, repo, branch badge, port :0, token rate laser meter
        assert!(rendered.contains("▶⚡ REPLAYFIX"));
        assert!(rendered.contains("MetaPulse"));
        assert!(rendered.contains(Symbols::BRANCH));
        assert!(rendered.contains("PORT :0"));
        assert!(rendered.contains("312 tok/s"));

        // Left Pane (40%): effort: xhigh, [TOOL: ffmpeg], Nerd Font glyphs & gate cards
        assert!(rendered.contains("effort: xhigh"));
        assert!(rendered.contains("[TOOL: ffmpeg]"));
        assert!(rendered.contains(Symbols::SPEC));
        assert!(rendered.contains(Symbols::SPARK));
        assert!(rendered.contains(Symbols::CORE));

        // Right Pane (60%): line-number gutters & diff hunks
        assert!(rendered.contains("   1 │ "));
        assert!(rendered.contains("@@ -1,4 +1,4 @@"));

        // Hunk folding verification
        deck.toggle_hunk_folding();
        let folded_rendered = deck.render_frame(110, 24, &theme);
        assert!(folded_rendered.contains("[folded unchanged AST context]"));

        // Bottom Command Ribbon hotkey pills
        assert!(rendered.contains("Enter: Run"));
        assert!(rendered.contains("Tab: Switch"));
        assert!(rendered.contains("Space: Page"));
        assert!(rendered.contains("q: Quit"));

        // Rounded Unicode box geometry
        assert!(rendered.contains('╭'));
        assert!(rendered.contains('╮'));
        assert!(rendered.contains('╰'));
        assert!(rendered.contains('╯'));
    }
}
