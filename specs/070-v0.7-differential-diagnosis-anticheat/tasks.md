# Tasks: 070-v0.7-differential-diagnosis-anticheat

## Phase 1: Setup & Ground Truth (Done)
- [x] **Task 1.1**: Verify baseline v0.6.2 tests, clippy, fmt, and zero-panic pass.
- [x] **Task 1.2**: Initialize branch `release/v0.7.0-fsm-anticheat` and configure `.specify/feature.json`.
- [x] **Task 1.3**: Author complete SpecKit documentation (`spec.md`, `plan.md`, `data-model.md`, `research.md`, `quickstart.md`, `contracts/*`, `checklists/*`).

## Phase 2: Checkpointing & Resumable Session State
- [x] **Task 2.1**: Implement `crates/replayfix-core/src/checkpoint.rs` with `SessionStateCheckpoint`, `CheckpointStage`, and `StageArtifacts`.
- [x] **Task 2.2**: Implement checkpoint serialization, JSON schema validation, and TTL / HEAD / manifest staleness checks.
- [x] **Task 2.3**: Add unit tests for checkpoint saving, loading, TTL expiry, and mismatch rejection.

## Phase 3: Differential Matrix & 6-State FSM
- [x] **Task 3.1**: Implement `crates/replayfix-core/src/fsm/matrix.rs` defining `DifferentialState`, `CausalConfidence`, `DifferentialDiagnosisReport`, and matrix resolution logic.
- [x] **Task 3.2**: Export matrix types through `crates/replayfix-core/src/fsm.rs` and update `crates/replayfix-core/src/lib.rs`.
- [x] **Task 3.3**: Add unit tests verifying state transitions for all 6 states (`CAUSAL_DEFECT_CONFIRMED`, `INVALID_TEST_OVERFIT`, `QUARANTINE_INVERTED_BASELINE`, `UNATTRIBUTED_SYMPTOM`, `CLEAN_PASS_UNREPRODUCED`, `FLAKY_NON_DETERMINISTIC`).

## Phase 4: Cryptographic Anti-Cheat & Surgical `package.json` DiffGuard
- [x] **Task 4.1**: Enhance `TestFreezeGuard` in `crates/replayfix-core/src/repair.rs` to parse `package.json` AST/JSON and detect test script tampering while permitting non-test edits.
- [x] **Task 4.2**: Harden path normalization and reparse point detection for scratch clone target paths.
- [x] **Task 4.3**: Add unit tests in `repair.rs` testing `package.json` test-script mutation rejection, non-test script modification acceptance, and path traversal rejection.

## Phase 5: Structured Regression Parity Engine
- [x] **Task 5.1**: Implement `crates/replayfix-core/src/regression.rs` with `TestIdentity`, `RegressionSuiteSummary`, and JSON reporter parsing for Playwright / Jest / Vitest test outputs.
- [x] **Task 5.2**: Implement set comparison math: $F_{\text{patched}} \setminus F_{\text{baseline}} = \emptyset$, test count invariants, and skip count tracking.
- [x] **Task 5.3**: Add unit tests in `regression.rs` verifying regression detection, drop detection, skip count inflation, and valid parity.

## Phase 6: Engine Orchestration & Clean Reference Verification
- [x] **Task 6.1**: Update `crates/replayfix-core/src/engine.rs` to support `--clean-ref <REF>` evaluation in ephemeral scratch clones.
- [x] **Task 6.2**: Chain execution: Checkpoint -> Gate 1 RED -> Clean Ref check -> Repair -> Gate 2 (2x) -> Regression Parity -> Receipt.
- [x] **Task 6.3**: Enforce correct exit codes (`0`, `10`, `11`, `21`, `31`, `32`) based on differential matrix evaluation.

## Phase 7: Receipt Schema 0.7 & Artifact Disaggregation
- [x] **Task 7.1**: Update `crates/replayfix-core/src/receipt.rs` schema version to `"0.7"`, embedding `differential_diagnosis` and `regression_parity`.
- [x] **Task 7.2**: Implement artifact disaggregation: write `repro_spec.json`, `candidate.patch`, and `regression_report.json` to `.replayfix/artifacts/`.
- [x] **Task 7.3**: Update receipt unit tests and verify RFC 8785 canonical hash computation.

## Phase 8: CLI Subcommands & Flags
- [x] **Task 8.1**: Update `crates/replayfix-cli/src/main.rs` to support `--clean-ref <REF>` and `--verify-matrix` on `run` and `repro`.
- [x] **Task 8.2**: Add `freeze-spec` subcommand to `replayfix` CLI.
- [x] **Task 8.3**: Add integration tests in `crates/replayfix-cli/tests/cli.rs` covering `--clean-ref`, `freeze-spec`, and regression parity failure handling.

## Phase 9: Quality Gate Audits & Release Qualification
- [x] **Task 9.1**: Run `cargo fmt --all -- --check`.
- [x] **Task 9.2**: Run `cargo check --workspace --all-targets --all-features`.
- [x] **Task 9.3**: Run `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- [x] **Task 9.4**: Run `powershell -ExecutionPolicy Bypass -File scripts/check-panics.ps1`.
- [x] **Task 9.5**: Run `cargo test --workspace --all-targets`.
- [ ] **Task 9.6**: Prepare release notes in `docs/release-notes/v0.7.0.md` and update `CHANGELOG.md` and `docs/agent-state.md`.
- [ ] **Task 9.7**: Merge to `main`, tag `v0.7.0`, and push to `origin`.
