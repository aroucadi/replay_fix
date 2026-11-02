# Tasks: Spec 160 — Sovereign Autonomous Healer: Multi-Turn Muse Spark Agent Loop & Sub-60s Macro Pipeline (Release 1.6.0)

**Input**: Design documents from `/specs/160-v1.6.0-sovereign-healer-macro/`  
**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [contracts/spark_deliberation.schema.json](./contracts/spark_deliberation.schema.json), [contracts/macro_heal_request.schema.json](./contracts/macro_heal_request.schema.json)

## Phase 1: Setup & Foundational Deliberation Types

- [x] T001 Define `DeliberationToolCall`, `DeliberationToolObservation`, `SparkDeliberationTurn`, `SparkDeliberationSession`, and `DeliberationStatus` in `crates/replayfix-core/src/deliberation.rs`, export module in `crates/replayfix-core/src/lib.rs`, and add domain errors (`Deliberation(String)`, `MacroExecution(String)`) to `crates/replayfix-core/src/error.rs`

## Phase 2: User Story 1 & 2 — Muse Spark Multi-Turn Deliberation Engine & Wonder Encrypted Backtracking

- [x] T002 [US1] Implement `DeliberationEngine` in `crates/replayfix-core/src/deliberation.rs` executing multi-turn tool loops with `muse-spark-1.3`, `prompt_cache_key: "replayfix-v1.6-spark-agent-loop"`, and structured tool dispatch (`replayfix_reproduce`, `replayfix_inspect_trace`, `replayfix_verify_patch`, `replayfix_inoculate`)
- [x] T003 [US2] Connect `DeliberationEngine` to `BoundedRepairClient` and Meta Muse Wonder 1.0 AST synthesis in `crates/replayfix-core/src/repair.rs` & `gateway.rs`, threading `previous_response_id` and cached `reasoning.encrypted_content` across iterative turns with compiler/assertion diagnostics

## Phase 3: User Story 3 & 4 — Sovereign End-to-End Macro Command (`replayfix heal`) & Autonomous Error Correction

- [x] T004 [US3/US4] Implement `crates/replayfix-cli/src/heal.rs` and wire `Commands::Heal` into `crates/replayfix-cli/src/main.rs`, orchestrating the full sub-60s macro chain (Video Ingestion ➔ SAM 3.1 ➔ Voice Lock ➔ Ephemeral Server ➔ Spark Deliberation ➔ Wonder AST ➔ Gate 2 Stability ➔ Gate 3 Stress Quorum ➔ Inoculation ➔ Merkle Receipt)
- [x] T005 [US4] Integrate Gate 3 stress quorum flakiness detection and compiler error backtracking into the `heal` pipeline, with fail-closed quarantine (`triage_report.json`, exit 21) if attempts fail

## Phase 4: User Story 5 — Hardened Offline Proof Portal & Receipt Updates

- [x] T006 [US5] Update `AuditReceipt` in `crates/replayfix-core/src/receipt.rs` to include `SparkDeliberationSession` and deliberation timeline metadata
- [x] T007 [US5] Update `proof_viewer.html` template in `crates/replayfix-core/src/portal.rs` to render the interactive Turn-by-Turn Muse Spark Deliberation Timeline, side-by-side accessibility tree DOM diff inspector, and Merkle leaf verification

## Phase 5: Verification, Benchmarks, Release Packaging & Git Governance

- [x] T008 Add comprehensive unit and integration tests for `DeliberationEngine`, `heal` CLI subcommand, encrypted backtracking, and timeline portal rendering
- [x] T009 Run full qualification battery: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace --all-targets -- --test-threads=2`, and `check-panics.ps1`
- [ ] T010 Bump workspace version to `1.6.0`, update `CHANGELOG.md`, `README.md`, `docs/release-notes/v1.6.0.md`, build release binaries, assemble `release/v1.6.0/` bundle (`replayfix.exe`, `proof_viewer.html`, `SHA256SUMS.txt`), merge `release/v1.6.0` to `main`, tag `v1.6.0`, and push
