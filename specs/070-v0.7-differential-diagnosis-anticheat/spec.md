# Feature Specification: 070-v0.7-differential-diagnosis-anticheat — Differential Diagnosis FSM, Cryptographic Anti-Cheat, Regression Parity & Release Qualification

**Feature Branch**: `release/v0.7.0-fsm-anticheat`  
**Created**: 2026-10-28  
**Status**: Ready for Planning  
**Target Release**: `v0.7.0`  
**Methodology**: Elephant Carpaccio Vertical Micro-Slicing  
**Constitutional Laws**: Law V (Test Collection Parity), Law VI (Monotonic Causal Verification), Law VII (Model Honesty), Law X (Substrate Primacy), Operational Invariants 10 & 14–16  

---

## 1. Executive Summary & Problem Statement

ReplayFix `v0.6.x` established the bounded autonomous model repair loop (`replayfix_core::repair`). However, a passing test after repair does not in itself prove a **causal, non-regressive, and un-gamed repair**. An agent can:
1. Pass Gate 2 on an already-passing test (invalid test overfit / false positive).
2. "Pass" by modifying or deleting verification test assertions (`test.skip()`, assertion deletion).
3. Introduce silent regressions in unrelated parts of the application.
4. Pass once on a flaky timing condition that fails on subsequent runs.
5. Tamper with test runners (`package.json` `"scripts.test"` rewritten to `exit 0`).

`v0.7.0` introduces **independent differential verification, anti-cheating cryptographic enforcement, machine-readable regression parity math ($F_{\text{patched}} \setminus F_{\text{baseline}} = \emptyset$), and a 6-state formal diagnosis FSM**. The deterministic Rust substrate remains the sole qualification authority.

---

## 2. Requirements Clarification & Ground Truth Answers

During initial analysis, several key requirements were clarified against the project constitution and reference architecture:

1. **Exact Meaning of `clean_ref`**:
   * `clean_ref` is a trusted Git reference (branch name, tag, or 40-character commit SHA) representing the known-clean or upstream baseline before the defect was introduced.
   * When provided (Mode A), the harness clones `clean_ref` into an isolated, read-only scratch sandbox and executes the reproduction test to establish baseline behavior.
   * If `clean_ref` is omitted (Mode B), the harness operates in **Invariant Defect Mode**, verifying that unhandled runtime exceptions/crashes in the target context are eliminated while 100% of the baseline test suite passes.
2. **Defect Behavior on the Clean Baseline**:
   * On `clean_ref`, the reproduction test MUST pass (or not exhibit the target defect failure signature).
   * If the reproduction test fails on `clean_ref`, the baseline is broken, and the harness immediately quarantines with `QUARANTINE_INVERTED_BASELINE`.
3. **The Sixth Diagnostic FSM State**:
   * Confirmed from the authoritative specification:
     1. `CAUSAL_DEFECT_CONFIRMED`: Defect reproduces on baseline, passes on patch, regression suite untouched.
     2. `INVALID_TEST_OVERFIT`: Test passes on baseline before any fix is applied (invalid reproduction).
     3. `QUARANTINE_INVERTED_BASELINE`: Target test fails on baseline when clean branch is supposed to be working.
     4. `UNATTRIBUTED_SYMPTOM`: Crash occurs in harness infrastructure rather than target application.
     5. `CLEAN_PASS_UNREPRODUCED`: Repro test passes on buggy unpatched baseline (bug does not reproduce).
     6. `FLAKY_NON_DETERMINISTIC`: Non-reproducible run across the 2-run stability pass.
4. **Treatment of Pre-Existing Failures**:
   * Pre-existing failures in the baseline test suite are captured in the set $F_{\text{baseline}}$.
   * A patched candidate is NOT penalized for failures that already existed in $F_{\text{baseline}}$.
   * However, the patched candidate cannot introduce ANY new failures: $F_{\text{patched}} \setminus F_{\text{baseline}} = \emptyset$.
   * Total test inventory must remain intact: $T_{\text{baseline}} \setminus T_{\text{patched}} = \emptyset$.
   * Zero newly skipped tests are permitted: $\text{Skipped}_{\text{patched}} \le \text{Skipped}_{\text{baseline}}$.
5. **Protected Verification Files**:
   * `tests/**`, `__tests__/**`, `*.spec.*`, `*.test.*`.
   * `playwright.config.*`, `jest.config.*`, `vitest.config.*`, `cypress.config.*`, `tsconfig*.json`.
   * Surgical `package.json` test commands: `"scripts.test"` and `"scripts.test:*"` entries must remain byte-for-byte immutable, while non-test fields may be updated if required.
6. **Two-Run Stability Bounds**:
   * Must execute within the bounded 180s wall-clock limit (`MAX_REPAIR_WALL_CLOCK`). Both runs must pass consecutively.
7. **Receipt Schema Version**:
   * Upgraded to `"0.7"`, with explicit disaggregation of evidence artifacts into `.replayfix/artifacts/`.

---

## 3. User Scenarios & Prioritized User Stories

### User Story 1 (P1) — 6-State Differential Diagnosis FSM Matrix
As a release qualification authority, I want ReplayFix to evaluate baseline and patched outcomes across a formal 6-state FSM so that only genuinely repaired, causally verified defects receive a passing receipt, while overfit, broken-baseline, or infrastructure crashes fail closed into explicit quarantine categories.

**Independent Test**:
Unit tests in `crates/replayfix-core/src/fsm/matrix.rs` verify each of the 6 state transitions, terminal states, and rejection of invalid state transitions.

**Acceptance Scenarios**:
1. **Given** a baseline run where Gate 1 fails with the expected defect signature, and a patched run where Gate 2 passes twice consecutively with 100% regression parity, **When** the differential diagnosis matrix evaluates the run, **Then** it transitions to `CAUSAL_DEFECT_CONFIRMED`.
2. **Given** an unpatched candidate where Gate 1 unexpectedly passes, **When** evaluated, **Then** it transitions to `CLEAN_PASS_UNREPRODUCED` (exit code `10`).
3. **Given** a `clean_ref` where the reproduction test fails, **When** evaluated, **Then** it transitions to `QUARANTINE_INVERTED_BASELINE` (exit code `32`).
4. **Given** a patch that passes Gate 2 Run 1 but fails Gate 2 Run 2, **When** evaluated, **Then** it transitions to `FLAKY_NON_DETERMINISTIC` (exit code `21`).

---

### User Story 2 (P1) — Dual-Mode Evaluation (Differential Mode & Invariant Defect Mode)
As a developer running `replayfix run`, I want to optionally specify `--clean-ref <git-ref>` (Mode A) to compare against a known-clean baseline branch, or run without `--clean-ref` (Mode B) to verify that unhandled crashes in the defective context are eliminated without regressions.

**Independent Test**:
CLI integration tests verify that `--clean-ref main` launches the differential baseline comparison, while running without `--clean-ref` executes Invariant Defect Mode.

**Acceptance Scenarios**:
1. **Given** `--clean-ref main`, **When** the harness initializes, **Then** it clones `main` into a separate immutable scratch directory, executes the reproduction test to establish clean baseline behavior, and records its commit SHA in `audit_receipt.json`.
2. **Given** no `--clean-ref` argument, **When** the harness initializes, **Then** it establishes Gate 1 RED on the current target repo and requires Gate 2 GREEN plus full regression parity.

---

### User Story 3 (P1) — DiffGuard & Surgical `package.json` Verification Script Immutability
As a security auditor, I want ReplayFix to block any patch that modifies test files, runner configurations, or verification scripts in `package.json`, across case variants, path traversal, or symlink/junction escapes, with `CoreError::SecurityViolation("TEST_MUTATION_PROHIBITED")` or `"CONFIG_MUTATION_PROHIBITED"`.

**Independent Test**:
`cargo test -p replayfix-core -- security::tests` verifies rejection of patches modifying `tests/**`, `playwright.config.ts`, and `"scripts.test"` in `package.json`, while permitting harmless changes to non-test `package.json` fields.

**Acceptance Scenarios**:
1. **Given** a patch modifying `package.json` that changes `"scripts": { "test": "exit 0" }`, **When** `DiffGuard::validate_patch` inspects the patch, **Then** it rejects with `CONFIG_MUTATION_PROHIBITED` (exit code `11`).
2. **Given** a patch modifying `package.json` that only adds a dependency or updates a build script, **When** `DiffGuard::validate_patch` inspects the patch, **Then** it allows the modification.

---

### User Story 4 (P1) — Machine-Readable Regression Parity Engine
As an automated verification pipeline, I want ReplayFix to execute the pre-existing test suite before and after patch application, parse the structured Playwright JSON reporter output, and enforce $F_{\text{patched}} \setminus F_{\text{baseline}} = \emptyset$, $T_{\text{baseline}} \setminus T_{\text{patched}} = \emptyset$, and $\text{Skipped}_{\text{patched}} \le \text{Skipped}_{\text{baseline}}$.

**Independent Test**:
`cargo test -p replayfix-core -- regression::tests` tests 10 regression scenarios: passing test fails, test disappears, new test masks missing test, test is newly skipped, test identity renamed, duplicate test identities, malformed JSON reporter output.

**Acceptance Scenarios**:
1. **Given** a patch that fixes `RF-01` but causes an existing test `tests/user.spec.ts` to fail, **When** regression parity is evaluated, **Then** it fails with `REGRESSION_DETECTED` (exit code `31`) and lists the regression in `triage_report.json`.
2. **Given** a patch that preserves all pre-existing passing tests and skips zero tests, **When** regression parity is evaluated, **Then** it records `regression_parity_passed: true` in `audit_receipt.json`.

---

### User Story 5 (P1) — 2-Run Gate 2 Stability Sweep & Resumable Checkpoint TTL
As an engineer running long verification jobs, I want Gate 2 to require 2 consecutive passing runs to eliminate test flakiness, and I want execution state to be checkpointed to `session_state.json` with a cryptographic TTL envelope (`base_repo_head`, `manifest_sha256`, `contract_digest`, `ttl_seconds`) so that state can be resumed safely without using stale data.

**Independent Test**:
`cargo test -p replayfix-core -- checkpoint::tests` verifies checkpoint serialization, expiration after TTL, and rejection with `STALE_CHECKPOINT_INVALIDATED` when git HEAD or files mutate.

**Acceptance Scenarios**:
1. **Given** a valid run progress, **When** state is written, **Then** `session_state.json` conforms to `contracts/session_state.schema.json`.
2. **Given** a saved `session_state.json` whose `base_repo_head` differs from current repository HEAD, **When** resumed, **Then** it rejects with `STALE_CHECKPOINT_INVALIDATED` and starts fresh.

---

### User Story 6 (P2) — Codebase Inoculation ("Dreaming" / `freeze-spec`)
As a lead developer, I want to run `replayfix freeze-spec --bug RF-01` to synthesize the ephemeral verification test into a permanent regression test (`tests/repro/rf_01_regression.spec.ts`) in the target repository with an immutable cryptographic header, permanently immunizing the codebase against regressions.

**Independent Test**:
`cargo test -p replayfix-cli -- freeze_spec` verifies generation of `tests/repro/rf_01_regression.spec.ts` with SHA-256 header and valid Playwright test syntax.

---

## 4. Functional Requirements

- **FR-001 (Differential Diagnosis FSM)**: `replayfix_core::fsm::DifferentialMatrix` MUST implement the 6 formal states (`CAUSAL_DEFECT_CONFIRMED`, `INVALID_TEST_OVERFIT`, `QUARANTINE_INVERTED_BASELINE`, `UNATTRIBUTED_SYMPTOM`, `CLEAN_PASS_UNREPRODUCED`, `FLAKY_NON_DETERMINISTIC`) with explicit transition predicates.
- **FR-002 (Clean-Reference Baseline)**: When `--clean-ref <ref>` is specified, the harness MUST clone the clean reference into an isolated read-only scratch sandbox, execute the reproduction test, and verify that the target defect does NOT fail on the clean reference.
- **FR-003 (DiffGuard Runner Config Immutability)**: `DiffGuard` MUST reject any patch targeting `playwright.config.*`, `jest.config.*`, `vitest.config.*`, `tsconfig*.json`, or verification scripts in `package.json` (`scripts.test*`) with `CONFIG_MUTATION_PROHIBITED` (exit code `11`).
- **FR-004 (Surgical `package.json` Handling)**: `DiffGuard` MUST parse `package.json` modifications structurally, rejecting changes to verification test scripts while permitting benign updates to dependencies or application metadata.
- **FR-005 (Structured Regression Parity Math)**: `RegressionParityEngine` MUST execute the baseline test suite, capture Playwright JSON reporter output, and assert $F_{\text{patched}} \setminus F_{\text{baseline}} = \emptyset$, $T_{\text{baseline}} \setminus T_{\text{patched}} = \emptyset$, and $\text{Skipped}_{\text{patched}} \le \text{Skipped}_{\text{baseline}}$.
- **FR-006 (2-Run Consecutive Stability)**: Gate 2 MUST execute 2 consecutive qualifying runs of the unchanged reproduction test. Any mismatch between run 1 and run 2 MUST classify as `FLAKY_NON_DETERMINISTIC` (exit code `21`).
- **FR-007 (Disaggregated Fact Artifacts)**: The harness MUST write disaggregated physical evidence artifacts to `.replayfix/artifacts/` (`01_candidate.diff`, `02_preflight.json`, `03_gate2_evidence.json`, `04_audit_receipt.json`).
- **FR-008 (Resumable State Checkpoint TTL)**: `CheckpointManager` MUST write `session_state.json` conforming to `contracts/session_state.schema.json` and invalidate any checkpoint if git HEAD, manifest SHA-256, or TTL expires (`STALE_CHECKPOINT_INVALIDATED`).
- **FR-009 (Codebase Inoculation CLI)**: `replayfix freeze-spec --bug <id>` MUST generate a permanent regression test at `tests/repro/rf_{id}_regression.spec.ts` in the base repo upon human confirmation.
- **FR-010 (Audit Receipt Schema `"0.7"`)**: `AuditReceipt` and `TriageReport` MUST serialize `schema_version: "0.7"`, including `differential_diagnosis`, `regression_parity`, and `checkpoint_provenance`.

---

## 5. Success Criteria & Verification Gates

- **SC-001**: `cargo run -p replayfix-cli -- run --bug RF-01 --clean-ref main` reproduces the defect causal baseline, verifies clean-ref pass, applies the patch, passes 2 consecutive Gate 2 runs, verifies regression parity, and writes `audit_receipt.json` (schema `"0.7"`, exit code `0`).
- **SC-002**: An intentional regression introduced in a patch triggers `REGRESSION_DETECTED` (exit code `31`), writing `triage_report.json` with the exact failing regression tests.
- **SC-003**: An intentional tampering with `package.json` `"scripts.test"` or `playwright.config.ts` triggers `CONFIG_MUTATION_PROHIBITED` (exit code `11`).
- **SC-004**: `replayfix freeze-spec --bug RF-01` generates `tests/repro/rf_01_regression.spec.ts` with valid syntax and immutable cryptographic header.
- **SC-005**: 100% of workspace tests pass (`cargo test --workspace`), zero Clippy warnings (`-D warnings`), zero formatting discrepancies (`cargo fmt --all -- --check`), and zero non-test panics (`scripts/check-panics.ps1`).
