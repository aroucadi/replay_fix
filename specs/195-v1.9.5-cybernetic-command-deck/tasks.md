# Tasks: ReplayFix v1.9.5 — Living Cybernetic Command Deck

**Spec**: `specs/195-v1.9.5-cybernetic-command-deck/spec.md`  
**Plan**: `specs/195-v1.9.5-cybernetic-command-deck/plan.md`  

---

## Phase 1: 24-Bit Truecolor Tokens, Rounded Boxes & Pill Badges
- [x] **T001**: Upgrade `crates/replayfix-cli/src/ui.rs` with 24-bit Truecolor RGB palette (`Rgb`, `\x1b[38;2;...m`, `\x1b[48;2;...m`), background-filled pill badges (`Theme::pill`), rounded Unicode box geometry (`╭───╮`, `╰───╯`), and Nerd Font symbol constants (`󰊢`, `󰈚`, `⚡`, `󰒓`, `❯`).
- [x] **T002**: Upgrade `crates/replayfix-cli/src/tui/dialog.rs` and `crates/replayfix-cli/src/approval.rs` to use rounded Unicode box geometry and Truecolor diff highlighting while preserving non-TTY fallback and ANSI injection sanitization.

## Phase 2: Living Motion Engine (`tui/motion.rs`)
- [x] **T003**: Create `crates/replayfix-cli/src/tui/motion.rs` implementing 60 FPS `MotionClock`, 8-level fractional progress bars (`▏▎▍▌▋▊▉█`), traveling RGB sine-wave shimmer (`render_wave_shimmer`), 1.5s breathing prompt (`render_breathing_prompt` on `❯`), and animated emerald gate flash transitions (`GateFlashAnimation`).
- [x] **T004**: Wire `StatusSpinner` (`crates/replayfix-cli/src/tui/spinner.rs`) and `AgentShell` (`crates/replayfix-cli/src/shell.rs`) to the Living Motion Engine with automatic `!stdout().is_terminal()` fallback.

## Phase 3: Bento Terminal Layout & Differential Renderer (`tui/layout.rs` & `tui/diff_view.rs`)
- [x] **T005**: Create `crates/replayfix-cli/src/tui/layout.rs` implementing double-buffered `FrameBuffer` and `DifferentialRenderer`, plus `BentoCommandDeck` (Header Deck with repo, `󰊢` branch badge, `:0` port, token rate meter; Left Pane Thought Stream with `󰈚` spec, `⚡` spark, `󰒓` core; Right Pane Virtualized Diff Reviewer with line-number gutters and token coloring; Sticky Dock with inverted hotkey pills).
- [x] **T006**: Upgrade `crates/replayfix-cli/src/tui/diff_view.rs` and `crates/replayfix-cli/src/tui/mod.rs` to remove `Clear(ClearType::All)`, integrate differential frame rendering, rounded Bento chrome, line-number gutters, and Sticky Dock command pills.

## Phase 4: Verification, Versioning & Quality Gates
- [x] **T007**: Add comprehensive unit and integration tests for `ui.rs`, `tui/motion.rs`, `tui/layout.rs`, and `tui/diff_view.rs` verifying Truecolor sequences, 8-level fractional bars, wave shimmer, breathing prompt, gate flash decay, differential buffer zero-op on identical frames, and non-TTY degradation.
- [x] **T008**: Bump workspace crate versions to `1.9.5`, update `CHANGELOG.md` and `docs/release-notes/v1.9.5.md`, and verify all quality gates (`cargo check`, `cargo clippy -- -D warnings`, `cargo test --workspace`, `scripts/check-panics.ps1`).
