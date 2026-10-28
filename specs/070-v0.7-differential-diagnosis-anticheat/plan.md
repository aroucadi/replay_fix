# Implementation Plan: 070-v0.7-differential-diagnosis-anticheat — Differential Diagnosis, Anti-Cheat, Regression Parity & Resumable Checkpoints

**Branch**: `release/v0.7.0-fsm-anticheat` | **Date**: 2026-10-28 | **Spec**: [spec.md](spec.md)

---

## Summary

Deliver ReplayFix v0.7.0 by formalizing causal differential verification, cryptographic anti-cheat test & config freeze, automated regression parity testing, and durable checkpoint state management:

1. **6-State Differential Diagnosis FSM (`crates/replayfix-core/src/fsm/matrix.rs`)**:
   Implement a formal state matrix taking inputs from $B_{\text{unpatched}}$, $B_{\text{clean}}$, and $B_{\text{patched}}$ (Run 1 & Run 2) to attribute bug repairs causally to the patch. Distinguish valid fixes from clean baseline failures (`QUARANTINE_INVERTED_BASELINE`, exit 32), non-reproductions (`CLEAN_PASS_UNREPRODUCED`, exit 10), flaky patches (`FLAKY_NON_DETERMINISTIC`, exit 21), and test overfit (`INVALID_TEST_OVERFIT`, exit 10).
2. **Cryptographic Anti-Cheat & Surgical `package.json` Guard (`crates/replayfix-core/src/repair.rs`)**:
   Harden `TestFreezeGuard` to validate pre- and post-repair SHA-256 digests across all test files and configs. Extend protection with surgical `package.json` validation: allow safe dependency or script updates while strictly forbidding any modification to `"scripts.test*"` keys with `CoreError::SecurityViolation("TEST_MUTATION_PROHIBITED")` (exit 11).
3. **Structured Regression Parity Engine (`crates/replayfix-core/src/regression.rs`)**:
   Parse Playwright / test runner JSON reports. Compute $F_{\text{patched}} \setminus F_{\text{baseline}} = \emptyset$, test count invariants, and skip counts. Trigger `REGRESSION_DETECTED` (exit 31) if any regression is found.
4. **Resumable Session Checkpointing & Stale-State Defense (`crates/replayfix-core/src/checkpoint.rs`)**:
   Implement `SessionStateCheckpoint` adhering to `session_state.schema.json`. Reject stale checkpoints whose TTL is expired or whose `base_repo_head` or `manifest_sha256` differ from the active workspace.
5. **Artifact Disaggregation & Schema 0.7 Receipt Updates (`crates/replayfix-core/src/receipt.rs`)**:
   Write stage artifacts into `.replayfix/artifacts/` (`repro_spec.json`, `candidate.patch`, `regression_report.json`). Bump schema version to `"0.7"`, embedding `differential_diagnosis` and `regression_parity` in `AuditReceipt` and `TriageReport`.
6. **CLI Commands & Verification Flags (`crates/replayfix-cli/src/main.rs`)**:
   Add `--clean-ref <REF>` to `replayfix run` and `replayfix repro`. Add subcommand `replayfix freeze-spec --bug <ID>`. Add flag `--verify-matrix` for inspecting and evaluating FSM state decisions.

---

## Constitution & Invariant Matrix

| Invariant / Law | Target Module | Enforcement Mechanism | Verification |
| :--- | :--- | :--- | :---: |
| **Law I: Substrate Primacy** | `fsm/matrix.rs`, `regression.rs` | Rust harness is sole arbiter; typed error taxonomy | `check-panics.ps1`, `cargo test` |
| **Law V: Anti-Cheating Freeze** | `repair.rs`, `security.rs` | SHA-256 pre/post freeze, surgical `package.json` script check | Integration test in `tests/cli.rs` |
| **Law VI: Regression Parity** | `regression.rs` | Mathematical set difference $F_{\text{patched}} \setminus F_{\text{baseline}} = \emptyset$ | Unit tests in `regression.rs` |
| **Law VII: Truthful Receipts** | `receipt.rs`, `checkpoint.rs` | Schema `"0.7"`, RFC 8785 canonical hashes, disaggregated artifacts | Schema validation tests |
| **Law X: Production-Grade Rust** | Entire workspace | Zero `.unwrap()` in core/cli non-test code, clippy warnings denied | `cargo clippy -- -D warnings` |

---

## Implementation Steps (Micro-Phases)

1. **Step 1: Checkpoint & Session State Model** (`crates/replayfix-core/src/checkpoint.rs`)
   - Define `SessionStateCheckpoint`, `CheckpointStage`, and `StageArtifacts`.
   - Implement serialization, JSON schema validation, and TTL staleness checks.
2. **Step 2: Differential Matrix & 6-State FSM** (`crates/replayfix-core/src/fsm/matrix.rs`)
   - Define `DifferentialState`, `DifferentialDiagnosisReport`, and causal evaluation logic.
   - Wire into `crates/replayfix-core/src/fsm.rs`.
3. **Step 3: Cryptographic Anti-Cheat & Surgical `package.json` Guard** (`crates/replayfix-core/src/repair.rs`)
   - Update `TestFreezeGuard` to inspect `package.json` `"scripts"` AST/JSON.
   - Enforce path normalization and reparse point checks.
4. **Step 4: Regression Parity Engine** (`crates/replayfix-core/src/regression.rs`)
   - Implement JSON report parsing and set difference evaluation.
   - Integrate regression check into repair verification lifecycle.
5. **Step 5: Engine & Orchestrator Integration** (`crates/replayfix-core/src/engine.rs`)
   - Thread `--clean-ref` worktree evaluation.
   - Chain Gate 1 RED -> Repair -> Gate 2 GREEN (2x) -> Regression Parity -> Checkpointing -> Receipt.
6. **Step 6: Receipt & Triage Schema 0.7 Bump** (`crates/replayfix-core/src/receipt.rs`)
   - Add differential diagnosis and regression parity to `AuditReceipt` and `TriageReport`.
   - Write disaggregated artifacts into `.replayfix/artifacts/`.
7. **Step 7: CLI Subcommands & Flags** (`crates/replayfix-cli/src/main.rs`)
   - Add `freeze-spec` command and `--clean-ref` argument.
8. **Step 8: Test Coverage, Clippy, Panics & Release Qualification**
   - Unit tests across all new modules.
   - CLI integration tests.
   - Run full verification suite.
