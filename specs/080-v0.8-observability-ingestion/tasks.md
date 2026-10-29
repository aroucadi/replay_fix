# 📋 Tasks: ReplayFix v0.8.0 — Observability, GitHub Ingestion & Human Approval Gate

**Feature Identity**: `080-v0.8-observability-ingestion`  
**Execution Posture**: Test-Driven, BeautifulRust Zero-Panic Discipline  

---

## Phase 1: Specifications & Contracts (SpecKit)
- [x] **Task 1.1**: Initialize `specs/080-v0.8-observability-ingestion/` directory structure.
- [x] **Task 1.2**: Author `spec.md` with user stories, functional requirements (FR-001..FR-010), and exit codes.
- [x] **Task 1.3**: Author `plan.md` with system architecture and security invariants.
- [x] **Task 1.4**: Author `data-model.md` defining Rust structures and contract mapping.
- [x] **Task 1.5**: Author JSON schemas in `contracts/` (`issue_ingestion.schema.json`, `approval_audit.schema.json`).
- [x] **Task 1.6**: Author checklists (`requirements.md`, `security.md`, `release.md`) and `quickstart.md`.

## Phase 2: Core Ingestion Module (`replayfix-core::ingestion`)
- [x] **Task 2.1**: Implement `GitHubIssueRef` parser and strict URL validation with unit tests.
- [x] **Task 2.2**: Implement `GitHubIssueFetcher` with timeout, size cap, rate-limit mapping, and untrusted framing.
- [x] **Task 2.3**: Implement markdown reproduction step parser (`extract_repro_steps`).
- [x] **Task 2.4**: Wire `ParsedIssue` into `TaskManifest` translation.
- [x] **Task 2.5**: Add comprehensive unit tests in `ingestion.rs` (valid URLs, PR rejection, non-GitHub hosts, rate-limit simulation, step extraction).

## Phase 3: Playwright Trace Controller & Viewer (`replayfix-core::trace`)
- [x] **Task 3.1**: Define `TraceConfig` and trace lifecycle helpers in `crates/replayfix-core/src/trace.rs`.
- [x] **Task 3.2**: Implement `launch_trace_viewer` with TTY check, CI environment detection, and detached process spawn.
- [x] **Task 3.3**: Connect trace configuration to `BrowserController` in `browser.rs`.
- [x] **Task 3.4**: Add unit tests for trace viewer launch logic (mocking CI, non-TTY, and valid execution paths).

## Phase 4: Human Approval Gate & Diff Syntax Highlighter (`replayfix-cli::approval`)
- [x] **Task 4.1**: Implement ANSI unified diff highlighter `highlight_diff` with line colors and headers.
- [x] **Task 4.2**: Implement `request_approval` with interactive TTY reader, `--yes` auto-bypass, and fail-closed non-interactive refusal.
- [x] **Task 4.3**: Implement `ApprovalAudit` record generation and serialization into `approval_audit.json`.
- [x] **Task 4.4**: Add unit and integration tests for human confirmation (approval, rejection, non-TTY fail-closed, `--yes` bypass).

## Phase 5: Disaggregated Artifacts & Engine Wiring
- [x] **Task 5.1**: Implement `crates/replayfix-core/src/artifacts.rs` for atomic fact writes to `.replayfix/artifacts/`.
- [x] **Task 5.2**: Update `crates/replayfix-core/src/patch.rs` with `UnifiedPatcher::apply_to_worktree`.
- [x] **Task 5.3**: Update `crates/replayfix-cli/src/main.rs` to wire `--trace`, `--issue <url>`, `--apply`, and `--yes` CLI arguments and handle exit codes 12, 13, 23, 24.

## Phase 6: Integration Testing & Verification
- [x] **Task 6.1**: Add integration tests in `crates/replayfix-cli/tests/` verifying `--trace`, `--issue`, `--yes`, and refusal exit codes.
- [x] **Task 6.2**: Run zero-panic check (`scripts/check-panics.ps1`) to ensure 0 panics outside tests.
- [x] **Task 6.3**: Run `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --check`.
- [x] **Task 6.4**: Run full test suite (`cargo test --workspace` — 185/185 tests passing).

## Phase 7: Documentation & Release Packaging
- [x] **Task 7.1**: Bump crate versions to `0.8.0` in `Cargo.toml`.
- [x] **Task 7.2**: Write release notes `docs/release-notes/v0.8.0.md` and update `CHANGELOG.md` & `docs/agent-state.md`.
- [x] **Task 7.3**: Create git commit and tag `v0.8.0`.
