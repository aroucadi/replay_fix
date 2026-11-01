# Tasks: Spec 130 — Substrate Truth & Universal SPA Grounding (v1.3.0)

**Input**: Design documents from `specs/130-v1.3.0-substrate-truth-spa/` (`spec.md`, `plan.md`)  
**Prerequisites**: `spec.md` (required for user stories), `plan.md` (required for architecture)  
**Organization**: Tasks are grouped by User Story and Phase to enable independent implementation and testable MVP increments.

## Format: `[ID] [P?] [Story] Description`
- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (`US1`, `US2`, `US3`, `US4`)
- Exact file paths included in all descriptions

---

## Phase 1: Setup & Data Contracts

**Purpose**: Project initialization, module registration, and error definitions

- [x] T001 Register `environment` module in `crates/replayfix-core/src/lib.rs` and define `CoreError::Environment(String)` variant in `crates/replayfix-core/src/error.rs`.
- [x] T002 [P] Create `crates/replayfix-core/src/environment.rs` defining `SpaToolchain`, `ManagedSpaServer`, and `AttachedSpaEndpoint` types and serde derivations.

---

## Phase 2: Foundational Infrastructure

**Purpose**: Core toolchain detection and dynamic port extraction infrastructure

- [x] T003 Implement `SpaToolchain::detect(repo_dir: &Path)` inspecting `package.json` for Vite, Next.js, Create-React-App, and Node dev scripts in `crates/replayfix-core/src/environment.rs`.
- [x] T004 Implement dynamic port regex parsing (`http://(?:127\.0\.0\.1|localhost):(\d+)`) and HTTP readiness polling with 30s timeout in `crates/replayfix-core/src/environment.rs`.

---

## Phase 3: User Story 1 - Runtime-Verified Provenance & Anti-Spoofing (Priority: P1) 🎯 MVP

**Goal**: Guarantee audit receipt provenance is derived strictly from verified execution telemetry, reject conflicting mock flags, and zero token accounting on deterministic runs.

- [x] T005 [US1] Enforce `--mode live-ai` vs. mock flags mutual exclusion in `crates/replayfix-cli/src/main.rs:build_manifest`, rejecting collisions with Exit Code `2` (`ERR_USAGE`).
- [x] T006 [US1] Wire execution-derived provenance in `crates/replayfix-cli/src/main.rs:run_two_gate_loop`, stamping `"live-ai"` only on authentic telemetry and defaulting to `"deterministic-fixture"`.
- [x] T007 [US1] Initialize `provenance: Some("deterministic-fixture".to_string())` in `crates/replayfix-core/src/engine.rs` constructors and clear `token_accounting = None` in deterministic runs.
- [x] T008 [P] [US1] Add automated regression tests in `crates/replayfix-cli/tests/cli.rs` verifying default provenance, mock flag rejection, and zero billing on deterministic replays.

**Checkpoint**: User Story 1 (P1 MVP) is complete, testable, and independently verifiable.

---

## Phase 4: User Story 2 - Managed SPA Server Supervision & Port 0 Allocation (Priority: P2)

**Goal**: Automatically detect SPA toolchains, boot an ephemeral dev server on dynamic port `:0`, verify readiness within 30s, and cleanly tear down child processes via RAII `Drop`.

- [x] T009 [US2] Implement `ManagedSpaServer::spawn` with dynamic port `:0` allocation, `NODE_PATH` wiring, and child stdout/stderr reader threads in `crates/replayfix-core/src/environment.rs`.
- [x] T010 [US2] Implement `Drop` for `ManagedSpaServer` invoking `Platform::terminate_process_tree` to guarantee zero orphaned child dev-server processes on Windows and POSIX.
- [x] T011 [US2] Integrate `ManagedSpaServer` into `MockHealer::run_live_browser_in_repo` in `crates/replayfix-core/src/engine.rs` when `package.json` is detected in the scratch sandbox.
- [x] T012 [P] [US2] Add unit tests for `ManagedSpaServer` boot, port extraction, readiness timeout handling, and RAII cleanup in `crates/replayfix-core/src/environment.rs`.

---

## Phase 5: User Story 3 - Universal SPA Attached Mode with Sandbox Safety (Priority: P2)

**Goal**: Allow ReplayFix to connect Playwright CDP directly to an already-running dev server (`--url <endpoint>`), verify Gate 1 RED, apply patches in a scratch sandbox, and verify Gate 2 GREEN after page reload.

- [x] T013 [US3] Add `--url <endpoint>` CLI argument to `ReproArgs` and `ReproExtras` in `crates/replayfix-cli/src/main.rs`.
- [x] T014 [US3] Implement `AttachedSpaEndpoint::validate` in `crates/replayfix-core/src/environment.rs` verifying URL scheme and TCP/HTTP reachability.
- [x] T015 [US3] Wire Attached Mode into `MockHealer::run_live_browser_in_repo` in `crates/replayfix-core/src/engine.rs`, bypassing static file server, executing CDP `Page.reload`, and stamping `substrate = "attached-spa-remote"`.
- [x] T016 [P] [US3] Add integration test for Attached Mode in `crates/replayfix-cli/tests/cli.rs` testing against an active HTTP test server.

---

## Phase 6: User Story 4 - Clean Substrate Grounding & Synthetic Fixture Removal (Priority: P3)

**Goal**: Remove synthetic `#btn-publish-quote` from `RF01_INDEX_HTML` and ground MetaPulse benchmarks directly against authentic workspace files on disk.

- [x] T017 [US4] Remove synthetic `<button id="btn-publish-quote">` from `RF01_INDEX_HTML` in `crates/replayfix-core/src/fixtures.rs`.
- [x] T018 [US4] Populate authentic runnable MetaPulse SPA in `benchmarks/metapulse/` (`package.json`, `server.js`, `public/index.html`, `public/app.js`).
- [x] T019 [US4] Update `Fixture::for_task_in_repo` in `crates/replayfix-core/src/fixtures.rs` to ground against authentic disk-based workspace files.

---

## Phase 7: Quality Gates & Milestone Release Qualification

**Purpose**: Verify all compiler, linter, test, and packaging gates before shipping v1.3.0

- [x] T020 Run `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets -- -D warnings`.
- [x] T021 Run `cargo test --workspace --all-targets` and verify 100% pass parity.
- [x] T022 Run `powershell -ExecutionPolicy Bypass -File .specify/scripts/check-panics.ps1` to verify zero non-test panics.
- [x] T023 Update `CHANGELOG.md`, `README.md`, `docs/agent-state.md`, and package `release/v1.3.0/` binaries.
