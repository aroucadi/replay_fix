# Tasks: 000-v0.0-workspace-genesis — Clean-Room Workspace Genesis & Toolchain Lock-In

**Feature**: Clean-Room Workspace Genesis & Toolchain Lock-In  
**Spec**: [spec.md](spec.md) | **Plan**: [plan.md](plan.md)  
**Status**: Completed  

---

## Phase 1: Setup & Environment Initialization

- [x] T001 Initialize Git repository with default branch `main` in workspace root
- [x] T002 Configure root `.gitignore` ignoring build targets, caches, and ephemeral runs in `.gitignore`
- [x] T003 Establish project operational rules and clean-room directives in `AGENTS.md`

---

## Phase 2: Foundational Workspace Manifest

- [x] T004 Create root workspace manifest with resolver 2 and release profile in `Cargo.toml`
- [x] T005 [P] Scaffold core library crate manifest with serde, thiserror, tokio in `crates/replayfix-core/Cargo.toml`
- [x] T006 [P] Scaffold CLI crate manifest with clap, tracing, and core dependency in `crates/replayfix-cli/Cargo.toml`

---

## Phase 3: User Story 1 — Pristine 2-Crate Architecture (Priority: P1)

*Goal*: Build and compile the 2-crate workspace linking `replayfix-core` and `replayfix-cli`.  
*Independent Test*: `cargo check --workspace` exits with code 0.

- [x] T007 [P] [US1] Implement structured error taxonomy `CoreError` using thiserror in `crates/replayfix-core/src/error.rs`
- [x] T008 [P] [US1] Create library root exposing modules in `crates/replayfix-core/src/lib.rs`
- [x] T009 [US1] Implement CLI entrypoint with clap subcommand parsing in `crates/replayfix-cli/src/main.rs`
- [x] T010 [US1] Verify workspace compilation via `cargo check --workspace`

---

## Phase 4: User Story 2 — Host Toolchain & Platform Abstraction (Priority: P2)

*Goal*: Implement cross-platform OS family detection and path canonicalization.  
*Independent Test*: `cargo test --workspace` passes unit tests for `Platform::os_family`.

- [x] T011 [US2] Implement `OsFamily` enum and `Platform` abstraction in `crates/replayfix-core/src/platform.rs`
- [x] T012 [US2] Add unit test for host platform detection in `crates/replayfix-core/src/platform.rs`
- [x] T013 [US2] Run `cargo test --workspace` to verify host platform unit test execution

---

## Phase 5: User Story 3 — Living Agent State & Provenance Tagging (Priority: P3)

*Goal*: Establish empirical state journal and Day-0 version release commit.  
*Independent Test*: Verify `docs/agent-state.md` records empirical checks and release tag `v0.0.0` is created.

- [x] T014 [US3] Create living agent state journal recording verified evidence in `docs/agent-state.md`
- [x] T015 [US3] Verify optimized CLI binary version execution via `cargo run --release -p replayfix-cli -- --version`
- [x] T016 [US3] Stage workspace baseline and commit `feat(genesis): v0.0.0 clean-room workspace genesis and toolchain lock-in`
- [x] T017 [US3] Create and verify release tag `v0.0.0` via `git tag -a v0.0.0`

---

## Dependencies & Completion Order

```text
Phase 1 (Setup) ──> Phase 2 (Foundational) ──> Phase 3 (US1: Architecture) ──> Phase 4 (US2: Platform) ──> Phase 5 (US3: State & Commit)
```

- **User Story 1 (P1)**: Prerequisite for all downstream CLI and library functionality.
- **User Story 2 (P2)**: Builds upon `replayfix-core` from US1.
- **User Story 3 (P3)**: Validates and locks in US1 and US2 deliverables.

---

## Implementation Strategy (Elephant Carpaccio)

1. **Thinnest Slice First**: Establish the 2-crate layout and verify compilation.
2. **Deterministic Verification**: Verify compiler and unit tests at every transition before proceeding.
3. **Commit & Tag Lock-In**: Create the clean `v0.0.0` commit on `main`, push to remote, and stand down before Spec 010.
