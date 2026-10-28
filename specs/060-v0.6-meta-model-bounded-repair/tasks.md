# Tasks: Meta Model Bounded Repair & AST Patching (`v0.6.0`)

**Input**: Design documents from `/specs/060-v0.6-meta-model-bounded-repair/`
**Prerequisites**: `plan.md` (required), `spec.md` (required for user stories), `research.md`, `data-model.md`, `contracts/repair_response.schema.json`

## Phase 1: Setup & Error Taxonomy

- [x] T001 Bump workspace crate versions to `0.6.0` in `crates/replayfix-core/Cargo.toml` and `crates/replayfix-cli/Cargo.toml`
- [x] T002 Update `CoreError` in `crates/replayfix-core/src/error.rs` to add `ModelApiError(String)`, `PatchApplicationError(String)`, `RepairBudgetExceeded(String)`, and `SyntaxValidationError(String)`

---

## Phase 2: Foundational — Anti-Cheating `TestFreezeGuard` & Security Boundaries (Blocking Prerequisite)

- [x] T003 [US3] Implement `TestFreezeGuard` (`capture`, `validate_target_path`, `verify_integrity`) in `crates/replayfix-core/src/repair.rs` blocking `tests/**`, `*.spec.*`, `*.test.*`, `playwright.config.*`, `jest.config.*`, `vitest.config.*`, `tsconfig*.json`, and `package.json` across case variants, alternate separators, and `..` traversal with `CoreError::SecurityViolation("TEST_MUTATION_PROHIBITED")`
- [x] T004 [P] [US3] Add unit tests in `crates/replayfix-core/src/repair.rs` covering `tests/cart.spec.ts`, `TESTS\Foo.SPEC.ts`, `src/../../tests/foo.spec.ts`, `playwright.config.ts`, `package.json`, and post-patch disk tamper detection

---

## Phase 3: User Story 1 & 2 — Model Routing, Prompt Isolation, Deliberation Continuity & Ambiguity-Safe Block Patching

- [x] T005 [US1] Implement `MuseModelSlug`, `ReasoningEffort`, and `MuseModelConfig` in `crates/replayfix-core/src/repair.rs` permitting `muse-spark-1.3` (`xhigh`), `muse-wonder-1.0` (`high`), `muse-spark-1.3-contributor` (`xhigh`), and fallback `meta-models/muse-spark-open-1026`, while rejecting `muse-wonder-full-1.0` and `effort: "max"`
- [x] T006 [US1] Implement `PromptBuilder` in `crates/replayfix-core/src/repair.rs` with static system prefix (`prompt_cache_key: "replayfix-v0.6-harness-prefix-v1"`, `prompt_cache_retention: "24h"`) and `<UNTRUSTED_CONTEXT>` tag escaping
- [x] T007 [US2] Implement `RepairRequest` and `RepairModelResponse` in `crates/replayfix-core/src/repair.rs` supporting `include: ["reasoning.encrypted_content"]`, `previous_response_id`, and `encrypted_content` continuity across retry attempts
- [x] T008 [US1] Implement `StructuredRepairPatch::decode_strict` and `BlockPatcher::apply` in `crates/replayfix-core/src/repair.rs` enforcing the strict 5-field JSON Schema (`additionalProperties: false`, no markdown wrappers), exact single-match uniqueness (`0` -> `PATCH_TARGET_NOT_FOUND`, `>= 2` -> `AMBIGUOUS_PATCH`), and symbol verification
- [x] T009 [US2] Implement `SyntaxPreflight::check_file` (`node --input-type=module --check` via `run_bounded`) in `crates/replayfix-core/src/repair.rs`
- [x] T010 [US2] Implement `TokenAccounting`, `AttemptLineage`, `RepairTelemetry`, and `BoundedRepairClient` (with 3-attempt cap, 180s wall-clock ceiling, rate-card micro-dollar calculation, and offline deterministic modes) in `crates/replayfix-core/src/repair.rs` and export in `crates/replayfix-core/src/lib.rs`

---

## Phase 4: User Story 4 — Engine Integration, 2-Run Gate 2 Stability, Receipt `"0.6"` & CLI Wiring

- [x] T011 [US4] Update `crates/replayfix-core/src/receipt.rs` to set `SCHEMA_VERSION = "0.6"`, add `TriageStatus::QuarantineRepairFailed` (`"QUARANTINE_REPAIR_FAILED"`, exit code `21`), and add `repair_telemetry: Option<RepairTelemetry>` and `token_accounting: Option<TokenAccounting>` to `AuditReceipt` and `TriageReport`
- [x] T012 [US4] Update `crates/replayfix-core/src/engine.rs` to integrate `TestFreezeGuard`, `BoundedRepairClient`, `SyntaxPreflight`, and 2-run consecutive Gate 2 verification (`gate2_runs_completed: 2`, `gate2_stability_passed: true`) into `MockHealer`
- [x] T013 [US4] Update `crates/replayfix-cli/src/main.rs` to add `QUARANTINE_REPAIR_FAILED: i32 = 21` to `exit_codes`, add `--model <MODEL>`, `--mock-failing-repair`, and `--mock-syntax-retry` flags to `ReproArgs`, and render `Test Freeze`, `Muse Repair`, `Syntax Check`, and `Gate 2 (GREEN)` steps
- [x] T014 [P] [US4] Update `crates/replayfix-cli/tests/cli.rs` for schema `"0.6"` and add end-to-end CLI tests for `--model muse-spark-1.3`, `--mock-failing-repair` (exit `21`), `--mock-syntax-retry`, and protected test path rejection (`TEST_MUTATION_PROHIBITED`, exit `11`)

---

## Phase 5: Polish, Live Qualification & Release Gates

- [x] T015 Execute live browser qualification (`cargo run -p replayfix-cli -- run --bug RF-01 --live-browser --headed --model muse-spark-1.3 --output-dir target/qa-v06`) and quarantine qualification (`cargo run -p replayfix-cli -- run --bug RF-01 --mock-failing-repair --output-dir target/qa-v06-quarantine`)
- [x] T016 Run all mechanical verification gates (`check-panics.ps1`, `cargo fmt --all -- --check`, `cargo check --workspace --all-targets --all-features`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace`, process/temp hygiene check)
- [x] T017 Update `README.md`, `CHANGELOG.md`, `docs/release-notes/v0.6.0.md`, and `docs/agent-state.md`
- [x] T018 Commit, merge `release/v0.6.0-model-repair` to `main`, create annotated tag `v0.6.0`, and push to `origin`
