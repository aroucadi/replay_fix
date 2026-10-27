# Tasks: 030-v0.3-ephemeral-scratch-isolation — Ephemeral Scratch Isolation & Dual-Layer Integrity

**Spec**: [spec.md](spec.md) | **Plan**: [plan.md](plan.md)
**Rule**: The workspace must compile and all tests must pass after every task.

---

## Phase 1: Setup & Error Taxonomy

- [x] T001 Bump workspace crates to `0.3.0` in `crates/replayfix-core/Cargo.toml` and `crates/replayfix-cli/Cargo.toml`
- [x] T002 Add `CoreError::Worktree(String)` in `crates/replayfix-core/src/error.rs` and declare `pub mod worktree` in `crates/replayfix-core/src/lib.rs`

---

## Phase 2: Foundational — Ephemeral Port & Process Tree Supervision (User Story 3)

- [x] T003 [P] [US3] Implement `EphemeralPort::allocate()` (`127.0.0.1:0`) and `Platform::terminate_process_tree(pid)` in `crates/replayfix-core/src/platform.rs`
- [x] T004 [US3] Add unit tests for `EphemeralPort::allocate` (non-zero port, distinct allocations) and `Platform::terminate_process_tree` (spawned child process termination and idempotent exit on dead PID) in `crates/replayfix-core/src/platform.rs`

---

## Phase 3: User Story 1 — Dual-Layer Base Repository Integrity ($\Delta \text{repo} = 0$) (Priority: P1) 🎯 MVP

- [x] T005 [US1] Implement `FileEntry` and `WorktreeManifest` (`capture`, `capture_with_excludes`, `verify_unchanged`, Layer 1 recursive SHA-256 tree walk + Layer 2 `git status --porcelain=v1 -uall`) in `crates/replayfix-core/src/worktree.rs`
- [x] T006 [US1] Add unit tests for `WorktreeManifest` (deterministic tree hash, exclusion of `.git`/`.replayfix`/custom output dirs, detection of modified/added/deleted/untracked files, and Git porcelain drift detection) in `crates/replayfix-core/src/worktree.rs`

---

## Phase 4: User Story 2 — Ephemeral Scratch Sandbox & Windows Lock Defense (Priority: P2)

- [x] T007 [US2] Implement `ScratchSandbox` (`create`, `root`, `register_child_pid`, `read_file`, `write_file`, `teardown`, and `Drop` with child process termination, read-only attribute clearing, and retry backoff) in `crates/replayfix-core/src/worktree.rs`
- [x] T008 [US2] Enforce path traversal and sandbox escape guards on all `ScratchSandbox` file operations in `crates/replayfix-core/src/worktree.rs`
- [x] T009 [US2] Add unit tests for `ScratchSandbox` (copying source tree without mutating base repo, applying patches on disk inside sandbox, rejecting `../` and absolute escape paths, and tearing down sandboxes containing read-only files) in `crates/replayfix-core/src/worktree.rs`

---

## Phase 5: Engine, Receipt & CLI Integration (User Stories 1, 2, 3)

- [x] T010 [P] [US1] Update `SCHEMA_VERSION` to `"0.3"`, add `TriageStatus::BaseRepoDirtyMutationDetected` (exit code `11`), and add `base_tree_sha256`, `sandbox_tree_sha256`, `base_repo_unmodified_verified`, and `ephemeral_port` to `AuditReceipt` in `crates/replayfix-core/src/receipt.rs`
- [x] T011 [US1] Wire `MockHealer` in `crates/replayfix-core/src/engine.rs` to execute Gate 1, patch application, and Gate 2 inside `ScratchSandbox`, verify `pre_manifest == post_manifest` on the base repository, and emit `BASE_REPO_DIRTY_MUTATION_DETECTED` if any mutation occurred
- [x] T012 [US1] Add engine unit tests in `crates/replayfix-core/src/engine.rs` covering sandboxed execution, custom repo source files, and simulated base-repo dirty mutation rejection
- [x] T013 [US1] Update `crates/replayfix-cli/src/main.rs` to validate `--repo`, pass it and `--output-dir` into `MockHealer::run_in_repo`, and print the isolation and ephemeral port summary
- [x] T014 [US1] Add CLI integration tests in `crates/replayfix-cli/tests/cli.rs` verifying receipt v0.3 fields (`base_tree_sha256`, `sandbox_tree_sha256`, `base_repo_unmodified_verified`, `ephemeral_port`), custom repo file preservation ($\Delta \text{repo} = 0$), and invalid `--repo` handling

---

## Phase 6: Polish, Quality Gates & Shipping

- [x] T015 Run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, and `cargo test --workspace`
- [x] T016 Execute mandatory review (`gstack-review`), QA (`gstack-qa`), and security (`gstack-cso`) audits; address any findings
- [x] T017 Update `README.md`, `CHANGELOG.md`, `docs/release-notes/v0.3.0.md`, `docs/agent-state.md`, and mark completed tasks in `specs/030-v0.3-ephemeral-scratch-isolation/tasks.md`
- [x] T018 Execute `gstack-ship`: commit, merge `release/v0.3.0-scratch-isolation` to `main`, tag `v0.3.0`, push to `origin`, and verify remote state
