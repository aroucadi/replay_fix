# Spec 195: ReplayFix v1.9.5 — Living Cybernetic Command Deck (TUI/UX Architecture)

**Feature Branch**: `release/v1.9.5`  
**Version Target**: `1.9.5`  
**Status**: Active Specification (SpecKit SDD)  
**Inspiration**: Claude Code, Charm Lipgloss/Bubbles, Lazygit  

---

## 1. Executive Summary & Architectural Vision
Upgrade the ReplayFix Terminal UI (`crates/replayfix-cli/src/ui.rs` and `crates/replayfix-cli/src/tui/*`) from a static, 16-color terminal output layer into a 2026-era **Living Cybernetic Command Deck** featuring 24-bit Truecolor RGB aesthetics, rounded Unicode box geometry (`╭───╮`, `╰───╯`), background-filled pill badges (`\x1b[48;2;R;G;Bm`), a 60 FPS sub-character Living Motion Engine (`tui/motion.rs`), and a double-buffered differential Bento Terminal Layout (`tui/layout.rs`).

All components strictly obey **BeautifulRust Law X** (`0` `.unwrap()`, `0` `.expect()`, `0` panics in production code), preserve deterministic RAII terminal restoration (`ScreenGuard` + panic hook), and automatically degrade to clean unformatted text when `!stdout().is_terminal()` or `--color never` / `NO_COLOR` is active.

---

## 2. Functional Requirements (FR-195-001 – FR-195-012)

### Pillar 1: Banish 1970s Clichés (`ui.rs`, `tui/dialog.rs`, `tui/diff_view.rs`, `tui/spinner.rs`)
- **FR-195-001 (24-Bit Truecolor RGB Design Tokens)**: Replace raw 16-color ANSI escapes (`\x1b[36m`, `\x1b[1;36m`, etc.) with 24-bit Truecolor foreground (`\x1b[38;2;R;G;Bm`) and background (`\x1b[48;2;R;G;Bm`) semantic tokens (`CyberPalette`: Cyber Cyan, Emerald Matrix, Amber Pulse, Crimson Alert, Lavender Spark, Slate Surface, Muted Steel).
- **FR-195-002 (Background-Filled Pill Badges)**: Replace plain bracketed status tags (`[PASS]`, `[FAIL]`, `[OK]`, `[WARN]`) with background-filled Truecolor pill badges (` PASS `, ` FAIL `, ` RUN `, ` WARN `, ` SKIP `, ` OK `) with high-contrast foreground typography and graceful plain-text fallback when color is disabled.
- **FR-195-003 (Rounded Unicode Box Geometry)**: Replace flat ASCII/single-line horizontal rules (`+---+`, `-----`) in banners, security cards, and modal dialogs with rounded Unicode box frames (`╭───╮`, `│ ... │`, `╰───╯`).
- **FR-195-004 (Eliminate Full-Screen Clear Blink)**: Eliminate `Clear(ClearType::All)` full-screen redraw blink in interactive views in favor of cursor home/row positioning (`\x1b[H`, `MoveTo`) and line/frame differential updates.

### Pillar 2: Living Motion Engine (`crates/replayfix-cli/src/tui/motion.rs`)
- **FR-195-005 (60 FPS Sub-Character Tick Engine)**: Implement `MotionEngine` with a `16ms` (~60 FPS) deterministic frame clock and sub-character fractional progress bars using the 8-level Unicode block progression (`▏`, `▎`, `▍`, `▌`, `▋`, `▊`, `▉`, `█`).
- **FR-195-006 (Traveling RGB Sine-Wave Shimmer)**: Implement `render_wave_shimmer(text, tick, base_rgb, peak_rgb, color_enabled)` that interpolates 24-bit RGB luminance across characters along a traveling sine wave when the agent is deliberating or executing tools.
- **FR-195-007 (Breathing Idle Prompt)**: Implement `render_breathing_prompt(tick, color_enabled)` generating a smooth 1.5-second (90-frame at 60 FPS) sinusoidal luminance pulse on the `❯` prompt character.
- **FR-195-008 (Animated Status Row Flash Transitions)**: Implement `RowFlashState` / `render_gate_flash_row(...)` that flashes a row with a bright emerald background highlight upon gate passage before exponentially decaying into a muted passed state.

### Pillar 3: Bento Terminal Layout & Differential Renderer (`crates/replayfix-cli/src/tui/layout.rs`)
- **FR-195-009 (Double-Buffered Differential Screen Buffer)**: Implement `Cell`, `FrameBuffer`, and `DifferentialRenderer` that compare `current` and `previous` 2D cell grids and emit minimal ANSI cursor moves and Truecolor sequences only for mutated rows/cells, preventing terminal tearing or flicker.
- **FR-195-010 (Responsive 80x24+ Bento Command Deck Layout)**:
  - **Header Deck**: Displays active repository name, Git branch badge (`󰊢`), ephemeral port (`:0`), and live token rate meter (`tok/s`).
  - **Dual-Pane Body**:
    - **Left Pane**: Live Agent Thought Stream & Tool Event Log with Nerd Font glyphs (`󰈚` spec, `⚡` spark, `󰒓` core).
    - **Right Pane**: Virtualized Diff Reviewer with line-number gutters, hunk headers (`@@`), and 24-bit syntax token coloring.
  - **Sticky Dock**: Inverted background command pills for hotkeys (` Enter: Approve `, ` Tab: Cycle `, ` j/k: Scroll `, ` q: Abort `).

### Pillar 4: Zero-Panic & Substrate Compliance
- **FR-195-011 (BeautifulRust Law X Compliance)**: Zero `.unwrap()`, zero `.expect()`, zero `panic!`, and zero `unsafe` across all new and modified TUI/UI modules.
- **FR-195-012 (RAII Terminal Safety & Non-TTY Fallback)**: `ScreenGuard` and `install_terminal_panic_hook()` guarantee clean exit from raw mode and alternate screen on drop or panic; when `!stdout().is_terminal()` or `color_enabled == false`, all components degrade to clean, unformatted output suitable for CI/CD and pipes.

### Pillar 5: Sovereign Web Portals (`proof_viewer.html` & `dashboard.html`)
- **FR-195-013 (`proof_viewer.html` — The Sovereign Cryptographic Notary Vault)**:
  - **Aesthetic & Typography**: High-end judicial/cryptographic instrument meets Swiss typographic editorial (`Instrument Serif` / `Fraunces` display headings, `Plus Jakarta Sans` structural copy, `JetBrains Mono` hexadecimal digests and Merkle leaves).
  - **Material Atmosphere**: Frosted obsidian glass (`backdrop-filter: blur(24px)`), `1px` structural hairline borders (`rgba(255,255,255,0.07)`), and an organic tactile SVG `feTurbulence` noise overlay evoking a physical tamper-evident bond.
  - **Layout & Invariants**: Interactive hierarchical Merkle tree explorer (`leaf-list`, `leaf-tree`, `merkle-root`, `signature-status`, `diff-container`, `diff-viewer`), glowing inclusion audit paths, side-by-side surgical AST diff reviewer, and a pulsing tactile cryptographic seal badge, while strictly preserving 100% of the RFC 8785, RFC 6962, and RFC 8032 Ed25519 client-side JS engine and air-gapped CSP (`default-src 'none'; connect-src 'none'`).
- **FR-195-014 (`dashboard.html` — The Autonomous Agent Swarm Observatory)**:
  - **Aesthetic & Typography**: Cyber-industrial mission control HUD / deep-space observatory (`Syne` / `Cabinet Grotesk` display headings paired with `Bricolage Grotesque` and `Space Mono`).
  - **Material Atmosphere**: Deep void canvas (`#08090c`) with surgical contrast accents (phosphor amber `#f59e0b`, acid emerald `#10b981`, and international Klein blue `#2563eb`), zero purple gradients, subtle scanline overlay, and staggered `cubic-bezier(0.16, 1, 0.3, 1)` reveals.
  - **Layout & Invariants**: Asymmetric Bento matrix grouping the live browser screencast viewport (`/ws/screencast/:id`), multi-agent Muse Spark reasoning thoughts (`effort: xhigh`, `/events/reasoning/:id`), rate-pool token laser meters, and parallel worker health tiles while preserving all existing WebSocket, SSE, and REST telemetry hooks.

---

## 3. Success Criteria (SC-195-001 – SC-195-005)
- **SC-195-001**: Zero occurrences of `Clear(ClearType::All)`, ASCII square borders (`+---+`), or legacy 16-color escapes in `ui.rs` and `tui/*`.
- **SC-195-002**: `tui/motion.rs` unit tests verify 8-level fractional progress blocks (`▏▎▍▌▋▊▉█`), traveling RGB sine-wave shimmer, 1.5s breathing `❯` prompt, and emerald gate transition flash decay.
- **SC-195-003**: `tui/layout.rs` unit tests verify 80x24+ Bento layout rendering (Header Deck with `󰊢` and `:0`, Left Thought Stream with `󰈚`/`⚡`/`󰒓`, Right Virtualized Diff with line gutters, Sticky Dock command pills) and differential frame diffing (0 bytes emitted on identical consecutive frames).
- **SC-195-004**: `powershell -ExecutionPolicy Bypass -File scripts/check-panics.ps1` reports `Total non-test violations: 0`.
- **SC-195-005**: `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace` pass 100%.
