# Tasks: 020-v0.2-mock-healer-two-gates — Dual-Gate State Machine & Mock Healer

**Spec**: [spec.md](spec.md) | **Plan**: [plan.md](plan.md)
**Rule**: the workspace compiles and tests pass after every task.

## Phase 1: Setup & Debt Removal
- [x] T001 Bump workspace crates to `0.2.0`; add `sha2` to core; remove unused deps (`tokio`, `tracing`, `walkdir`, `async-trait` from core; `serde`, `tracing` from CLI)
- [x] T002 Delete orphaned `scripts/cargo-runner.ps1`; fix mojibake in `docs/windows-code-signing.md`
- [x] T003 Add `CoreError::Patch`; export new modules from `lib.rs`

## Phase 2: Foundations
- [x] T004 [P] `digest.rs`: canonical JSON + SHA-256 hex with tests (key order, nesting, float rejection, known digest)
- [x] T005 [P] `fsm.rs`: `Phase`, `PhaseMachine` guarded transitions with tests (legal path, illegal jumps, terminal states)

## Phase 3: US2 — Unified Diff Patcher (Task 2.1)
- [x] T006 `patch.rs`: `UnifiedPatcher::apply_diff` (single file, exact context, count/order validation, safe header paths)
- [x] T007 Patch tests: valid, multi-hunk, context mismatch, bad header, count mismatch, overlap, multi-file, creation header, unsafe path, CRLF, no-op

## Phase 4: US1 — Two-Gate Loop (Tasks 2.2, 2.3)
- [x] T008 `fixtures.rs`: `RF-01` buggy source, contract, and built-in fix (`cart.items?.length || 0`) (Task 2.2)
- [x] T009 `engine.rs`: `BehavioralContract`, simulated oracle, event classification (Law VI), `MockHealer::run` with Gate 1, Repair, Gate 2 (Task 2.3)
- [x] T010 Engine tests: happy path, contract mismatch, Gate 1 not RED, target-deleting patch

## Phase 5: US3 — Anti-No-Op Guard (Task 2.4)
- [x] T011 Reject empty, whitespace-only, net-zero patches (`REPAIR_NOOP_REJECTED`); non-fixing patch ends `GATE2_NOT_GREEN`
- [x] T012 Tests for each adversarial case

## Phase 6: Receipts & CLI (Task 2.5)
- [x] T013 `receipt.rs`: `AuditReceipt`, `TriageReport`, writers with stale-artifact removal and tests
- [x] T014 CLI: `--mock-heal`, `--mock-patch`, `--output-dir`; extract helpers; version from crate metadata; corrected ingestion status line
- [x] T015 CLI integration tests (`tests/cli.rs`): success, noop, unsafe output dir, unknown task, custom patch file, flag dependency, stale receipt removal

## Phase 7: Quality Gates & Release
- [x] T016 `cargo fmt --check`, `clippy -D warnings`, `cargo test --workspace`, release build, CLI smoke tests
- [x] T017 Review (`gstack-review`), QA (`gstack-qa`), security audit (`gstack-cso`); fix findings
- [x] T018 Update README, CHANGELOG, `RELEASE_NOTES.md`, `docs/agent-state.md`, spec tasks; humanize user-facing text
- [x] T019 Ship (`gstack-ship`): commit, merge to `main`, tag `v0.2.0`, push, verify remote
