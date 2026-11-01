# Implementation Plan: Spec 140 — Agent Experience Revolution (Release 1.4)

**Branch**: `release/v1.4.0-agent-experience-revolution` | **Date**: 2026-11-01 | **Spec**: [spec.md](./spec.md)  
**Input**: Feature specification from `/specs/140-v1.4.0-agent-experience-revolution/spec.md` and research findings from `research.md`.

---

## 1. Summary

Spec 140 transforms ReplayFix from a flags-only CLI tool into a human-centered, conversational agentic terminal application.
Key deliverables:
1. **Interactive Conversational Shell (`replayfix` default / `replayfix chat`)**: Natural-language intent parser, interactive prompt with session context, and plan-then-execute workflow.
2. **Real-Time Substrate Event Engine**: An observer pipeline connecting synchronous `replayfix-core` execution to an interactive reactive terminal renderer in `replayfix-cli`.
3. **Context-Aware Tiered Permissions**: Low, Medium, High risk categorization with interactive approval dialogs and session persistence.
4. **Interactive Hunk Diff Reviewer**: Keyboard-navigable, virtualized diff viewer with syntax highlighting and selective patch confirmation.
5. **Session Lifecycle & Resumption**: Checkpoint tracking, activity logging, and safe interruption handling.
6. **100% Backward Compatibility**: Zero regressions across existing commands, flags, exit codes, and 241 unit/integration tests.

---

## 2. Technical Context

- **Language/Version**: Rust 2021 edition (Rust 1.80+)
- **Primary Dependencies**:
  - `crates/replayfix-core`: `serde`, `serde_json`, `thiserror`, `regex`, `chrono`, `sha2`, `ed25519-dalek`, `rand_core`, `reqwest`
  - `crates/replayfix-cli`: `clap`, `serde`, `serde_json`, `chrono`, `tracing-subscriber`, + `crossterm` (v0.28) for cross-platform raw mode, resize detection, and event polling
- **Architecture**: 2-crate workspace (`replayfix-core`, `replayfix-cli`)
- **Safety Standard**: `#![forbid(unsafe_code)]`, zero `.unwrap()` outside tests, `clippy -- -D warnings`
- **Testing**: `cargo test --workspace` (unit + integration), mock TTY harnesses, non-interactive CI tests

---

## 3. Constitution Check & Invariant Gates

- **Gate I (Substrate Primacy)**: Verified. All interactive UI elements derive state directly from genuine substrate execution events; no synthetic or fabricated progress.
- **Gate II (Contract Invariance)**: Gate 1 and Gate 2 continue to evaluate identical `BehavioralContract` canonical digests.
- **Gate III (Repository Isolation)**: $\Delta\text{repo} = 0$ is strictly maintained in ephemeral scratch sandboxes during trials. Patch application to base repo only occurs after explicit human gate approval.
- **Gate V (Regression Parity)**: 100% test parity enforcement remains intact in `RegressionEvaluator`.
- **Gate X (BeautifulRust Discipline)**: Zero panics, strongly typed domain errors via `thiserror`, modular components, clean clippy pass.

---

## 4. Module & File Layout

```text
crates/replayfix-core/
├── src/
│   ├── observer.rs       # [NEW] SubstrateObserver trait, SubstrateEvent, event dispatcher
│   ├── engine.rs         # [MOD] Integrate observer hook into run_live_browser & run_autonomous
│   └── lib.rs            # [MOD] Export observer module and types

crates/replayfix-cli/
├── Cargo.toml            # [MOD] Add crossterm = "0.28"
├── src/
│   ├── main.rs           # [MOD] Wire interactive default launcher when stdin is TTY
│   ├── ui.rs             # [MOD] Keep existing tokens, export styled primitives
│   ├── approval.rs       # [MOD] Integrate with interactive diff viewer
│   ├── shell.rs          # [NEW] Interactive Agent REPL loop, prompt, and command router
│   ├── intent.rs         # [NEW] Natural language intent parser & plan builder
│   ├── permissions.rs    # [NEW] Tiered permission policy, prompt dialogs, audit log
│   ├── tui/              # [NEW] Modular terminal components
│   │   ├── mod.rs        # Component exports & event loop
│   │   ├── screen.rs     # Alternate screen buffer & resize manager
│   │   ├── spinner.rs    # In-place animated spinner & phase timer
│   │   ├── diff_view.rs  # Keyboard-driven virtualized diff viewer
│   │   └── dialog.rs     # Contextual permission & plan review dialogs
│   └── session.rs        # [NEW] Interactive session state tracker & history log
└── tests/
    ├── cli.rs            # [EXISTING] All 47 CLI tests preserved
    └── agent_ux.rs       # [NEW] Comprehensive integration tests for interactive AX features
```

---

## 5. Architectural Implementation Sequence

### Phase 1: Core Event Observer Substrate
1. Create `crates/replayfix-core/src/observer.rs` defining `SubstrateEvent` and `SubstrateObserver`.
2. Connect `observer` into `MockHealer::run_live_browser_in_repo`, `run_autonomous_in_repo`, and `run_in_repo_with_regression`.
3. Provide a default `NullObserver` so that existing code and tests compile with 0 modifications.

### Phase 2: Terminal UI Components (`ui::terminal`)
1. Add `crossterm = "0.28"` to `crates/replayfix-cli/Cargo.toml`.
2. Implement `tui/screen.rs`: Raw mode management, window dimensions, graceful teardown on drop/panic.
3. Implement `tui/spinner.rs`: Lightweight 30 FPS active step spinner and duration clock.
4. Implement `tui/diff_view.rs`: Paged, keyboard-driven (`j`/`k`, `Tab`) diff review widget.
5. Implement `tui/dialog.rs`: Contextual approval dialogs for Low/Medium/High risk actions.

### Phase 3: Conversational Intent & Plan Engine
1. Implement `intent.rs`: Parse natural language strings into `AgentIntent` (`reproduce`, `fix`, `doctor`, `sweep`, `receipt`, `help`).
2. Map extracted intent to a structured `ExecutionPlan` detailing tasks, steps, affected resources, and risks.
3. Implement conversational clarification when tasks or parameters are ambiguous.

### Phase 4: Permissions & Session Management
1. Implement `permissions.rs`: Enforce Low, Medium, High risk categorization with `AllowOnce`, `AllowSession`, and `Deny` decisions.
2. Implement `session.rs`: Track conversational history, link to `SessionStateCheckpoint`, and record audit trails.

### Phase 5: Interactive Agent Shell & CLI Entry
1. Implement `shell.rs`: Interactive REPL coordinating prompt input, plan presentation, background worker execution, live TUI updates, and diff review.
2. Update `main.rs`: When run without arguments in an interactive TTY, launch `AgentShell::run()`; if non-interactive or subcommand provided, run existing CLI logic.

### Phase 6: Validation & Verification
1. Run `cargo test --workspace` ensuring all existing 241 tests pass.
2. Write integration tests in `crates/replayfix-cli/tests/agent_ux.rs` verifying natural language intent parsing, permission gating, diff review, and session persistence.
3. Run `cargo clippy --workspace --all-targets -- -D warnings` and fix any warnings.
4. Update `CHANGELOG.md`, `README.md`, and `ROADMAP.md`.