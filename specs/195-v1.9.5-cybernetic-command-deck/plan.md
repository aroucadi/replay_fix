# Implementation Plan: ReplayFix v1.9.5 — Living Cybernetic Command Deck

**Spec**: `specs/195-v1.9.5-cybernetic-command-deck/spec.md`  
**Target Crate**: `crates/replayfix-cli` (`v1.9.5`) & `crates/replayfix-core` (`v1.9.5`)  

---

## 1. Architecture Overview

### 1.1 24-Bit Truecolor & Rounded Box Design System (`crates/replayfix-cli/src/ui.rs`)
- Upgrade `ui::style` to Truecolor RGB (`\x1b[38;2;R;G;Bm` foreground, `\x1b[48;2;R;G;Bm` background) and provide `Rgb(u8, u8, u8)` helper methods (`fg_ansi()`, `bg_ansi()`, `lerp()`).
- Add `Theme::pill(label, fg, bg)` producing background-filled pill badges (`\x1b[1;38;2;...;48;2;...m LABEL \x1b[0m`) when `color_enabled` is true, and clean `LABEL` / `[LABEL]` fallback when `color_enabled` is false.
- Upgrade `print_banner`, `print_step`, `print_diagnostics`, `print_security_boundary`, and `tui/dialog.rs::render_card` to use rounded Unicode box geometry (`╭`, `─`, `╮`, `│`, `╰`, `╯`) and Truecolor pill badges.

### 1.2 Living Motion Engine (`crates/replayfix-cli/src/tui/motion.rs`)
- **`MotionClock`**: Tracks `tick: u64` at 60 FPS (`FRAME_DURATION_MS = 16`).
- **`render_fractional_bar(progress: f64, width_cells: usize, color_enabled: bool) -> String`**: Uses `['▏', '▎', '▍', '▌', '▋', '▊', '▉', '█']` (1/8th cell resolution) with Truecolor cyber-cyan to emerald gradient.
- **`render_wave_shimmer(text: &str, tick: u64, base: Rgb, highlight: Rgb, color_enabled: bool) -> String`**: Computes per-character phase $\phi_i = 	ext{tick} \cdot \omega - i \cdot k$, mapping $\frac{1 + \sin(\phi_i)}{2}$ to RGB interpolation between `base` and `highlight`.
- **`render_breathing_prompt(tick: u64, color_enabled: bool) -> String`**: 1.5s period (90 ticks at 60 FPS) sinusoidal luminance modulation on `❯`.
- **`GateFlashAnimation`**: Tracks elapsed frames since gate passage (`0..=30` ticks), interpolating background highlight from bright emerald (`Rgb(16, 185, 129)`) down to muted surface slate before settling into the steady passed state.

### 1.3 Bento Terminal Layout & Double-Buffered Renderer (`crates/replayfix-cli/src/tui/layout.rs`)
- **`Cell` & `FrameBuffer`**: 2D grid of `(char, Rgb fg, Option<Rgb> bg, bool bold)` sized `cols x rows`.
- **`DifferentialRenderer`**: Holds `prev: Option<FrameBuffer>` and `curr: FrameBuffer`. `render_diff(&mut self, next: &FrameBuffer, color_enabled: bool) -> String` emits cursor positioning (`\x1b[{row};{col}H`) and cell runs only for coordinates that changed since `prev`, eliminating full-screen clear blink.
- **`BentoCommandDeck`**:
  - **Header Deck** (`3` rows): Rounded box `╭───╮` displaying active repository, branch badge (`󰊢 main`), ephemeral port (`:0`), and live token rate meter (`⚡ 305 tok/s`) with fractional bar.
  - **Dual-Pane Body** (`rows - 6` rows):
    - **Left Pane** (`42%` width): Live Agent Thought Stream & Tool Event Log with Nerd Font glyphs (`󰈚` spec, `⚡` spark, `󰒓` core) and wave shimmer on active deliberation line.
    - **Right Pane** (`58%` width): Virtualized Diff Reviewer with line-number gutters (`  12 │`), hunk headers (`@@`), and 24-bit syntax token coloring.
  - **Sticky Dock** (`3` rows): Rounded bottom dock with inverted background command pills (` Enter: Approve `, ` Tab: Cycle `, ` j/k: Scroll `, ` Space: Page `, ` q: Abort `).

### 1.4 Upgraded `DiffReviewer` & `StatusSpinner` (`tui/diff_view.rs`, `tui/spinner.rs`, `shell.rs`)
- `DiffReviewer`: Uses `DifferentialRenderer` and cursor home positioning instead of `Clear(ClearType::All)`, renders rounded box chrome, line-number gutters, Truecolor diff tokens, and Sticky Dock command pills while preserving all existing public methods (`new`, `focus`, `scroll_offset`, `handle_key`, `format_frame`, `run_interactive`) and security invariants (`sanitize_ansi`, `MIN_TERMINAL_COLS = 80`, `MIN_TERMINAL_ROWS = 24`).
- `StatusSpinner`: Uses `render_wave_shimmer`, `render_fractional_bar`, and `GateFlashAnimation` with automatic non-TTY fallback.
- `AgentShell`: Uses `render_breathing_prompt` on interactive TTY prompts while degrading cleanly when `!io::stdout().is_terminal()`.
