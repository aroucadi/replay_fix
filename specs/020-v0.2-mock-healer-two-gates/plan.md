# Implementation Plan: 020-v0.2-mock-healer-two-gates — Dual-Gate State Machine & Mock Healer

**Branch**: `release/v0.2.0-mock-healer` | **Date**: 2026-10-26 | **Spec**: [spec.md](spec.md)

**Input**: Dual-gate FSM, in-memory unified-diff patcher, anti-no-op guard, terminal receipts, wired into `replayfix repro --mock-heal`.

---

## Summary

Add five focused modules to `replayfix-core` (`fsm`, `patch`, `digest`, `engine`, `receipt`) plus the built-in `RF-01` fixture, and wire them into the CLI behind `--mock-heal`. Remove technical debt carried from v0.1: the orphaned `scripts/cargo-runner.ps1`, unused dependencies (`tokio`, `tracing`, `walkdir`, `async-trait` in core; `serde`, `tracing` in the CLI), and the hardcoded version string.

---

## Technical Context

**Language/Version**: Rust 1.99.1 (Edition 2021)
**New Dependency**: `sha2` (core) for SHA-256 digests. Everything else is existing stack.
**Storage**: JSON artifacts in a configurable output directory (default `.replayfix/`, git-ignored)
**Testing**: `cargo test --workspace` — unit tests per module plus CLI integration tests (`crates/replayfix-cli/tests/`)
**Target Platform**: Windows (MSVC), macOS, Linux
**Constraints**: Fail closed; no filesystem access beyond the output directory and an optional patch file; no network; no subprocesses

---

## Constitution Check

| Constitutional Invariant | Status | Alignment & Evidence |
| :--- | :---: | :--- |
| **Law I: Substrate Primacy** | **PASS** | Only the Rust state machine can reach `Gate2Green`; transitions are guarded and illegal moves error. |
| **Law II: Contract Invariance** | **PASS** | Canonical contract digest compared before Gate 2 dispatch (`CONTRACT_MISMATCH` otherwise). |
| **Law III: Workspace Isolation** | **PASS (scoped)** | v0.2 touches no repository. Scratch clones arrive in Spec 030. Output paths use the shared path validator. |
| **Law IV: Ephemeral Provenance** | **N/A** | No server is spawned in v0.2; documented, not claimed. |
| **Law VI: Monotonic Ordering** | **PASS** | RED requires SETUP/TARGET/ACTION to pass before ASSERT fails. |
| **Law VII: Model Honesty** | **PASS** | Receipt names the substrate (`mock-static-oracle`) and carries a disclaimer; no model is involved. |
| **Law X: Vertical Micro-Slicing** | **PASS** | One reviewable slice, compiling and green after each task. |

---

## Project Structure

```text
specs/020-v0.2-mock-healer-two-gates/
├── spec.md  plan.md  research.md  data-model.md  quickstart.md  tasks.md
└── checklists/requirements.md

crates/replayfix-core/src/
├── lib.rs        # export new modules
├── error.rs      # + Patch variant
├── digest.rs     # canonical JSON + SHA-256 hex
├── patch.rs      # UnifiedPatcher (task 2.1)
├── fixtures.rs   # RF-01 buggy source + built-in diff (task 2.2)
├── fsm.rs        # Phase, PhaseMachine
├── engine.rs     # contract, oracle, MockHealer, Gate 2 (tasks 2.3, 2.4)
└── receipt.rs    # AuditReceipt, TriageReport, writers (task 2.5)

crates/replayfix-cli/
├── src/main.rs   # --mock-heal wiring; helpers extracted from the monolithic match
└── tests/cli.rs  # integration tests via the compiled binary
```

---

## Complexity Tracking

One new dependency (`sha2`) is justified by Spec 070/090 needs (test freeze, Merkle receipts) and avoids a hand-rolled hash. Gate evaluation is deliberately a static fixture inspection; implementing more would pretend to be a browser.
