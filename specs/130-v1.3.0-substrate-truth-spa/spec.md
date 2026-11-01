# Feature Specification: Spec 130 — Substrate Truth & Universal SPA Grounding

**Feature Branch**: `release/v1.3.0-substrate-truth-spa`  
**Created**: 2026-11-01  
**Status**: Active / Ready for Implementation  
**Input**: User description: "Execute Release v1.3.0 (release/v1.3.0-substrate-truth-spa) to resolve all remaining provenance and billing integrity blockers from the v1.2.0 audit, eliminate fixture-based workarounds, and deliver the first production slice of the Universal Modern Web SPA Typology defined in ROADMAP.md §1."

---

## 1. Executive Summary & Architectural Thesis

Spec 130 (`v1.3.0`) delivers substrate truth across ReplayFix:
1. Enforces **strict execution-derived provenance** and **truth in token accounting**, ensuring no run claims live AI or charges non-zero tokens without authentic verified API telemetry.
2. Implements **Universal Modern Web SPA Ingestion** in both **Managed Mode** (`--repo <path>`) with dynamic port `:0` supervision and **Attached Mode** (`--url <endpoint> --repo <path>`) with sandbox-safe verification.
3. Eliminates **synthetic HTML fixture hacks** (such as `#btn-publish-quote` in `RF01_INDEX_HTML`), grounding perception and repairs against authentic workspace source trees on disk.

---

## Clarifications

### Session 2026-11-01
- Q: In Attached Mode (`--url <endpoint> --repo <path>`), how should ReplayFix apply candidate patches during the two-gate trial verification loop without violating Constitution Law III ($\Delta\text{repo} = 0$ on the base repository)? → A: Option A (Shadow Scratch First: Verify candidate patches inside an ephemeral shadow sandbox; only mutate `--repo` once Gate 2 GREEN is confirmed or via explicit `--apply` / `--yes`).
- Q: In Managed SPA Mode (`--repo <path>`), how should ReplayFix determine the dynamic port number for the spawned child dev server? → A: Option A (Hybrid Pre-Allocation: Pre-allocate port via OS socket `Platform::allocate_ephemeral_port()`, pass `PORT=<port>` & `--port <port>`, and confirm via HTTP readiness polling).
- Q: In Managed SPA Mode (`--repo <path>`), if a repository contains configuration or dependencies for multiple frameworks, what detection priority should `SpaToolchain::detect` enforce? → A: Option A (Deterministic Cascade + Filesystem Markers: Check `Vite` > `Next.js` > `ReactScripts` > custom `dev` > `start`, reinforced by config file presence).
- Q: When replaying multi-step video action trajectories against a live Modern Web SPA, how should Playwright CDP synchronize between consecutive actions? → A: Option A (DOM Mutation & Network-Idle Settle: Await DOM mutation quiescence with 200ms quiet window and network idle capped at 2s max before dispatching the next action).
- Q: In Managed SPA Mode, if a candidate patch causes the child dev server to crash or return a 500 compilation error during Gate 2, how should ReplayFix handle the failure? → A: Option A (Fast Fail-Closed & Diagnostic Retry: Catch child crash / 500, log stderr into `triage_report.json`, reset sandbox to baseline, and trigger the next repair attempt).

---

## 2. User Scenarios & Testing *(mandatory)*

### User Story 1 - Runtime-Verified Provenance & Anti-Spoofing (Priority: P1) 🎯 MVP

As an engineering auditor or hackathon judge, I want ReplayFix to derive audit receipt provenance strictly from verified runtime execution telemetry and reject conflicting mock flags, so that deterministic replays can never masquerade as live AI execution and token billing records are 100% honest.

**Why this priority**: Core integrity requirement. ReplayFix's credibility rests on infallible, fail-closed audit receipts.

**Independent Test**: Can be tested independently by running `replayfix run --bug RF-01` (unflagged), verifying `provenance == "deterministic-fixture"` and `token_accounting == null`, and confirming that `--mode live-ai` combined with any mock flag exits immediately with code `2`.

**Acceptance Scenarios**:
1. **Given** an invocation of `replayfix run --bug RF-01` without flags, **When** the two-gate loop completes, **Then** `.replayfix/audit_receipt.json` MUST contain `"provenance": "deterministic-fixture"`, `"model_provenance": null`, and `"token_accounting": null`.
2. **Given** an invocation with `--mode live-ai` and any mock flag (`--mock-syntax-retry`, `--mock-failing-repair`, `--mock-pkg-test-mutation`, or `--mock-patch != "builtin"`), **When** `build_manifest` executes, **Then** the process MUST exit immediately with Exit Code `2` (`ERR_USAGE`) and print a helpful mutual exclusion error.
3. **Given** an invocation with `--mode deterministic-fixture` and `--model muse-wonder-1.0`, **When** the repair succeeds, **Then** `receipt.token_accounting` MUST be cleared to `null` to maintain 100% parity with the disclaimer: *"Zero live external AI tokens billed"*.
4. **Given** an invocation with `--mode live-ai`, **When** genuine authenticated Meta AI responses are received and verified, **Then** `receipt.provenance` MUST be `"live-ai"` and `receipt.model_provenance` MUST contain the active serving models.

---

### User Story 2 - Managed SPA Server Supervision & Port 0 Allocation (Priority: P2)

As a developer running ReplayFix on a local web repository, I want ReplayFix to automatically detect my SPA framework (Vite, Next.js, CRA, Node dev scripts), boot an ephemeral dev server on an operating-system-assigned dynamic port (`:0`), verify readiness via HTTP polling, and reliably tear down the entire process tree on completion, so that I can reproduce and repair bugs in real SPAs without port collisions or orphaned processes.

**Why this priority**: Delivers the foundation of the Universal Modern Web SPA typology with zero manual configuration.

**Independent Test**: Can be tested independently by running `replayfix repro --video bug.mp4 --repo ./benchmarks/metapulse` against a package containing a `package.json` dev script, verifying dynamic port allocation, Gate 1 RED, repair, Gate 2 GREEN, and zero lingering child processes after exit.

**Acceptance Scenarios**:
1. **Given** a target repository with a `package.json` declaring dev scripts, **When** `ManagedSpaServer::spawn` runs, **Then** it MUST bind to dynamic port `:0` (passing `PORT=0` and `--port 0`), parse the assigned listening port from stdout/stderr, and verify HTTP reachability within 30 seconds.
2. **Given** a managed dev server that crashes during compilation or exceeds the 30-second readiness deadline, **When** readiness polling fails, **Then** ReplayFix MUST terminate the child process tree via `Platform::terminate_process_tree`, emit a fail-closed triage report, and exit with code `25` (`ERR_ENVIRONMENT_SETUP`).
3. **Given** a running managed dev server, **When** the ReplayFix process exits, panics, or drops the `ManagedSpaServer` handle, **Then** its `Drop` implementation MUST guarantee 100% clean process tree termination with zero orphaned Node/Vite processes.

---

### User Story 3 - Universal SPA Attached Mode with Sandbox Safety (Priority: P2)

As a developer with an already-running dev server (e.g. `http://localhost:3000`), I want ReplayFix to attach Playwright CDP directly to that running URL and execute verification safely, so that I can debug complex SPAs instantly without waiting for dev-server cold boot.

**Why this priority**: Enables frictionless developer adoption for large-scale enterprise applications where dev servers are already running in external Docker containers or terminals.

**Independent Test**: Can be tested by starting a local HTTP server at `http://127.0.0.1:8080`, running `replayfix repro --video bug.mp4 --url http://127.0.0.1:8080 --repo ./app`, and confirming direct CDP attachment, Gate 1 baseline reproduction, patch application, and Gate 2 verification.

**Acceptance Scenarios**:
1. **Given** `--url <endpoint>` is passed, **When** Playwright launches, **Then** it MUST attach directly to `<endpoint>` without launching an internal static file server.
2. **Given** Attached Mode is active, **When** a candidate patch is generated, **Then** verification MUST execute within an ephemeral scratch worktree (preserving $\Delta\text{repo} = 0$ on the base repository during trials).
3. **Given** a candidate patch is applied, **When** Gate 2 verifies the application, **Then** ReplayFix MUST trigger a CDP page navigation/reload (`Page.reload`) and await DOM readiness to prevent Hot Module Replacement (HMR) race conditions.
4. **Given** verification succeeds in Attached Mode, **When** `audit_receipt.json` is generated, **Then** it MUST record `substrate: "attached-spa-remote"` to honestly reflect that an external ambient dev server was used.

---

### User Story 4 - Clean Substrate Grounding & Synthetic Fixture Removal (Priority: P3)

As a test engineer, I want all temporary synthetic HTML fixture hacks removed from the codebase, so that perception and repair operate purely on authentic application code and realistic benchmark repositories.

**Why this priority**: Upholds the "Zero Fixture Deception" engineering principle and prevents artificial benchmark overfitting.

**Independent Test**: Inspect `crates/replayfix-core/src/fixtures.rs` and verify that `RF01_INDEX_HTML` contains no `#btn-publish-quote`, while `benchmarks/metapulse/` operates as an independent, fully functional SPA.

**Acceptance Scenarios**:
1. **Given** `RF01_INDEX_HTML` in `fixtures.rs`, **When** inspected, **Then** it MUST contain only the standard shopping cart markup and zero synthetic `#btn-publish-quote` elements.
2. **Given** `benchmarks/metapulse/`, **When** inspected, **Then** it MUST contain a self-contained, runnable SPA structure (`package.json`, `server.js`, `public/index.html`, `public/app.js`) with authentic DOM elements matching video recordings.
3. **Given** `Fixture::for_task_in_repo(task_id, repo_path)`, **When** invoked against `benchmarks/metapulse`, **Then** it MUST resolve target files from disk rather than falling back to hardcoded fixture constants.

---

## 3. Edge Cases & Boundary Conditions

| Edge Case | Resolution Contract |
|---|---|
| **Mock Flag + `--mode live-ai` Collision** | `build_manifest` rejects any combination of `--mode live-ai` with mock flags with `ERR_USAGE` (Exit Code `2`). |
| **Default `replayfix run --bug RF-01` Invocation** | Passing `--bug <ID>` automatically activates two-gate execution and produces `.replayfix/audit_receipt.json` with `"provenance": "deterministic-fixture"`, `"model_provenance": null`, and `"token_accounting": null`. |
| **Dynamic Port `:0` Capture** | Clarified in Session 2026-11-01: `ManagedSpaServer` pre-allocates an ephemeral port via OS socket (`Platform::allocate_ephemeral_port()`), passes `PORT=<port>` and `--port <port>` (or `-p <port>` for Next.js), and confirms listening status via HTTP readiness polling. |
| **Dev-Server Crash or 30s Readiness Timeout** | Clarified in Session 2026-11-01: If a candidate patch causes the dev server to crash or return 500 during Gate 2, ReplayFix captures stderr into `triage_report.json`, fails Gate 2 (`SYNTAX_COMPILATION_ERROR`), resets the sandbox to baseline, and triggers the next bounded repair retry. If initial boot exceeds 30s, terminates process tree with Exit Code `25`. |
| **HMR Race Conditions in Attached Mode** | Before Gate 2 re-test, Playwright CDP issues `Page.reload` with `waitUntil: "domcontentloaded"` and awaits selector stabilization. |
| **Multi-Action Trajectory Settle** | Clarified in Session 2026-11-01: Between consecutive actions in an action trajectory, Playwright CDP awaits DOM mutation quiescence (200ms quiet window) and network-idle (capped at 2000ms max) before dispatching the next action. |
| **Base Repository Immutability ($\Delta\text{repo} = 0$)** | Clarified in Session 2026-11-01: Candidate trial patches are NEVER applied to the base repository during verification; all trial mutations occur in OS temporary scratch clones. Applying patches to `--repo` requires verified completion and the existing `--apply` / `--yes` approval gate. |
| **Node Modules Resolution in Sandboxes** | `ManagedSpaServer` injects `NODE_PATH=<base_repo>/node_modules` and prepends `<base_repo>/node_modules/.bin` to `PATH` to allow isolated scratch execution without duplicating large dependency trees. |

---

## 4. Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST reject `--mode live-ai` combined with any mock flags (`--mock-syntax-retry`, `--mock-failing-repair`, `--mock-pkg-test-mutation`, `--mock-patch != "builtin"`) upfront with Exit Code `2` (`ERR_USAGE`).
- **FR-002**: System MUST derive `receipt.provenance` strictly from verified runtime execution telemetry (`repair_telemetry.provider_mode == "live-meta-muse-api"` or `video_telemetry.sam_snap.model == "sam-3.1"`). If verified live telemetry is absent, `provenance` MUST default to `"deterministic-fixture"`.
- **FR-003**: System MUST set `receipt.model_provenance = None` and `receipt.token_accounting = None` whenever `provenance == "deterministic-fixture"`, guaranteeing zero live tokens billed.
- **FR-004**: System MUST support Managed SPA mode (`--repo <path>`): inspect `package.json`, detect SPA toolchain (Vite, Next.js, CRA, Node scripts), spawn child dev server on dynamic port `:0` (`PORT=0` / `--port 0`), poll HTTP readiness within 30s, and clean up process trees via RAII (`Drop`).
- **FR-005**: System MUST support Attached SPA mode (`--url <endpoint> --repo <path>`): connect Playwright CDP directly to `<endpoint>`, verify baseline Gate 1 RED, apply patches within an ephemeral scratch worktree (preserving $\Delta\text{repo} = 0$ on base repository during verification per Session 2026-11-01 clarification), reload page/DOM via CDP to eliminate HMR race conditions, verify Gate 2 GREEN, and record `substrate: "attached-spa-remote"`.
- **FR-006**: System MUST remove synthetic DOM elements (`#btn-publish-quote` in `RF01_INDEX_HTML`) and inline mock strings, grounding MetaPulse benchmarks directly against workspace files on disk.

### Success Criteria (Non-Functional)

- **SC-001**: Parity & Zero Panics: 100% test pass rate across unit and integration tests (`cargo test --workspace --all-targets`), 0 non-test panics (`scripts/check-panics.ps1`), and 0 Clippy warnings (`cargo clippy --workspace --all-targets -- -D warnings`).
- **SC-002**: Process Tree Hygiene: 0 orphaned child dev-server processes remaining after test runs or abnormal terminations on Windows and POSIX platforms.
