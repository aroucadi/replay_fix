# Feature Specification: Spec 150 — Permanent Inoculation, Agentic Protocol (MCP) & Continuous CI/CD Automation (Release 1.5.0)

**Feature Branch**: `release/v1.5.0`  
**Created**: 2026-11-01  
**Status**: Ready for Planning & Implementation  
**Input**: Master Product Design, SpecKit Workflow, Git Lifecycle & Engineering Directive for ReplayFix Release 1.5.0.

---

## 1. Executive Summary & Product Vision

Release 1.5 transforms **ReplayFix** from an interactive human-in-the-loop terminal tool into an **autonomous verification and permanent inoculation platform** that bridges human developers, automated CI/CD pipelines, and external AI coding agents (Cursor, Windsurf, Claude Code, Antigravity).

While preserving 100% backward compatibility with all prior releases ($\Delta	ext{compat} = 0$) and strict repository isolation ($\Delta	ext{repo} = 0$), Release 1.5 delivers six core architectural pillars:
1. **Permanent Codebase Inoculation ("Dreaming")**: Translating ephemeral diagnostic probes and verified repairs into permanent, self-healing, idiomatic Playwright/TypeScript regression test suites (`tests/repro/rf_<id>.spec.ts`) tracked in a tamper-evident registry (`.replayfix/inoculations.json`).
2. **Model Context Protocol (MCP) Universal Server**: Exposing ReplayFix as the infallible verification oracle over JSON-RPC 2.0 (`stdio` and Streamable HTTP / `SSE`), providing first-class tools, resources, and prompts to any external IDE or cloud coding agent.
3. **Autonomous Git Packaging & PR Engine**: Packaging verified repairs and inoculation tests into isolated Git branches (`replayfix/fix-<bug_id>`, `replayfix/inoculate-<bug_id>`), signing commits with Ed25519 + RFC 6962 Merkle trailers, and generating Evidence Matrix PR descriptions.
4. **Concurrency Perturbation & Stress Quorum (Gate 3)**: Eliminating flaky patches and asynchronous race conditions via simulated CDP network throttling/jitter and a 3-pass concurrent execution quorum before certifying any repair.
5. **Multi-Modal Conversational Copilot**: Enhancing the interactive TUI with ASR acoustic transcript alignment alongside keyframes, an interactive ambiguity tie-breaker dialog ($|\Delta	ext{score}| < 0.05$), and real-time streaming CDP/console/network telemetry.
6. **CI/CD Action & Headless Reporters**: Providing drop-in GitHub Actions automation (`action.yml`) and industry-standard `results.sarif` (SARIF v2.1.0) and `junit.xml` test reports.

---

## 2. User Scenarios & Testing *(mandatory)*

### User Story 1 - Permanent Codebase Inoculation ("Dreaming") (Priority: P1) 🎯 MVP

As a software engineer who has verified and repaired a visual defect using ReplayFix, I want ReplayFix to synthesize a permanent, human-readable Playwright regression test (`tests/repro/rf_<id>.spec.ts`) using resilient accessibility locators (`getByRole`, `getByText`, `getByTestId`) and register it in `.replayfix/inoculations.json`, so that the bug can never regress in future commits and executes natively via `npx playwright test` without requiring ReplayFix binaries.

**Why this priority**: Turns ephemeral bug verification into permanent codebase immunity ("Dreaming"), fulfilling Pillar 1.

**Independent Test**: Run `replayfix inoculate --bug RF-01` (or `replayfix repro --bug RF-01 --mock-heal --inoculate`), verify that `tests/repro/rf_01.spec.ts` is synthesized with semantic accessibility selectors and postcondition assertions, verify `.replayfix/inoculations.json` records the SHA-256 digests, and verify that the synthesized spec fails against the unpatched baseline and passes against the patched code.

**Acceptance Scenarios**:
1. **Given** a verified defect repair (e.g. `RF-01`), **When** inoculation synthesis runs, **Then** it MUST generate `tests/repro/rf_01.spec.ts` containing idiomatic `@playwright/test` code using grounded accessibility locators (`getByRole`, `getByText`, or `getByTestId`) with zero positional anti-patterns (`.first()`, `.last()`, `.nth()`, `:nth-match`, `:nth-child`).
2. **Given** a synthesized inoculation test, **When** evaluated against the unpatched buggy code vs. the repaired code, **Then** the test MUST fail on the unpatched baseline and pass on the patched state.
3. **Given** an inoculation synthesis event, **When** persisted, **Then** `.replayfix/inoculations.json` MUST be atomically updated with the bug ID, spec file path, spec SHA-256 digest, patch SHA-256 digest, contract digest, and timestamp.

---

### User Story 2 - Model Context Protocol (MCP) Universal Server (Priority: P1)

As an external AI coding agent (e.g. Cursor, Claude Code, Windsurf) or IDE integration, I want to connect to ReplayFix via Model Context Protocol (JSON-RPC 2.0 over `stdio` or `SSE/HTTP`), discover verification tools, resources, and prompts, and invoke `replayfix_reproduce`, `replayfix_verify_patch`, `replayfix_inoculate`, `replayfix_inspect_trace`, and `replayfix_doctor`, so that I can test repair hypotheses in isolated sandboxes and obtain cryptographic proof receipts.

**Why this priority**: Establishes ReplayFix as the universal verification oracle for the AI agent ecosystem (Pillar 2).

**Independent Test**: Launch `replayfix mcp --transport stdio` (or connect via HTTP/SSE), send standard JSON-RPC 2.0 `initialize`, `tools/list`, `tools/call`, `resources/list`, `resources/read`, and `prompts/get` requests, and verify compliant JSON-RPC 2.0 responses and fail-closed error handling on malformed payloads.

**Acceptance Scenarios**:
1. **Given** an MCP client connects over `stdio` or `SSE/HTTP`, **When** it sends `initialize` and `tools/list`, **Then** the server MUST return protocol capabilities and schemas for all 5 tools: `replayfix_reproduce`, `replayfix_verify_patch`, `replayfix_inoculate`, `replayfix_inspect_trace`, and `replayfix_doctor`.
2. **Given** an MCP client invokes `tools/call` for `replayfix_reproduce`, `replayfix_verify_patch`, or `replayfix_inoculate`, **When** executed, **Then** ReplayFix MUST run the operation in an isolated sandbox ($\Delta	ext{repo} = 0$) and return structured JSON telemetry and cryptographic receipt digests.
3. **Given** an MCP client queries `resources/read` for `replayfix://receipts/{task_id}`, `replayfix://sessions/{session_id}`, or `replayfix://benchmarks`, **When** requested, **Then** the server MUST return the canonical JSON resource contents.
4. **Given** an MCP client queries `prompts/get` for `triage_bug`, **When** requested with a bug ID, **Then** the server MUST return a structured multi-modal triage prompt guiding causal attribution and surgical patch synthesis.

---

### User Story 3 - Stress & Flakiness Gating (Gate 3 Concurrency Quorum) (Priority: P1)

As a release engineer, I want ReplayFix to subject candidate repairs that pass Gate 2 to a Gate 3 Stress & Concurrency Quorum (simulated network latency/jitter + 3-pass randomized-interval concurrent stability sweep), so that patches containing race conditions or timing assumptions are rejected and quarantined with `FLAKY_NON_DETERMINISTIC` (Exit Code 21).

**Why this priority**: Prevents race conditions and flaky UI fixes from escaping the sandbox (Pillar 4).

**Independent Test**: Run `replayfix repro --bug RF-01 --mock-heal --stress-quorum` on a deterministic patch (verifying Gate 3 `PASS` and `stress_quorum` record in `audit_receipt.json`) and with `--mock-flaky` or a race-condition patch (verifying transition to `FLAKY_NON_DETERMINISTIC` and exit code `21`).

**Acceptance Scenarios**:
1. **Given** Gate 3 Stress Quorum is enabled (`--stress-quorum`), **When** Gate 2 passes, **Then** the engine MUST execute a perturbation pass (simulating network latency/jitter up to 500ms) and a 3-pass concurrency quorum with jittered intervals.
2. **Given** any pass in Gate 3 fails or exhibits non-deterministic divergence, **When** evaluated, **Then** the state machine MUST transition to `REJECTED` with `TriageStatus::FlakyNonDeterministic` and exit with code `21`.
3. **Given** all Gate 3 passes succeed, **When** `audit_receipt.json` is emitted, **Then** it MUST include the `stress_quorum` telemetry record committed into the RFC 6962 Merkle tree.

---

### User Story 4 - Autonomous Git Packaging & PR Engine (Priority: P2)

As a developer or CI bot, I want `replayfix repro --bug RF-01 --package-pr` to package a verified fix and its inoculation test onto a clean, isolated Git branch (`replayfix/fix-RF-01`), commit with cryptographically signed `ReplayFix-*` trailers (`ReplayFix-Merkle-Root`, `ReplayFix-Signature`, `ReplayFix-Gate1-Status`, `ReplayFix-Gate2-Status`), and generate an Evidence Matrix PR description markdown (`pr_description.md`), so that repairs are immediately ready for code review and merge.

**Why this priority**: Automates the last mile from verified sandbox proof to production pull request (Pillar 3).

**Independent Test**: Run `replayfix repro --bug RF-01 --mock-heal --package-pr` in a target repository, verify that branch `replayfix/fix-rf-01` (or packaged commit artifact) is created with exact `ReplayFix-*` trailers verified against the Ed25519 signature, and verify that `pr_description.md` contains the Evidence Matrix, DOM `ariaSnapshot` diff, and verification summary.

**Acceptance Scenarios**:
1. **Given** `--package-pr` is passed on a verified repair, **When** packaging executes, **Then** it MUST format or commit the changes onto an isolated branch (`replayfix/fix-<bug_id>`) with canonical `ReplayFix-Verified: true`, `ReplayFix-Substrate`, `ReplayFix-Merkle-Root`, `ReplayFix-Signature`, `ReplayFix-Gate1-Status`, and `ReplayFix-Gate2-Status` trailers.
2. **Given** a packaged PR bundle, **When** `pr_description.md` is generated, **Then** it MUST include keyframe metadata with click bounding boxes, side-by-side DOM `ariaSnapshot` diffs (Defect vs. Repaired), gate execution logs, token accounting, and the offline verification command.

---

### User Story 5 - Multi-Modal Conversational Copilot & Ambiguity Picker (Priority: P2)

As a developer using the interactive terminal workspace, I want to inspect ASR spoken narration aligned with video keyframes, resolve close SAM/DOM locator ties ($|\Delta	ext{score}| < 0.05$) via an interactive terminal picker instead of blind guessing, and view streaming CDP/console/network events in real time.

**Why this priority**: Elevates human-agent collaboration during complex multi-modal debugging (Pillar 5).

**Independent Test**: Trigger an ambiguous candidate tie ($|\Delta	ext{score}| < 0.05$ / `500` bps) in interactive mode to verify the interactive locator picker resolves the chosen candidate, and inspect ASR transcript + keyframe alignment and streaming CDP events in the copilot log.

**Acceptance Scenarios**:
1. **Given** video telemetry contains ASR narration cues and keyframes, **When** inspected in the TUI or via `/trace`, **Then** spoken narration transcripts MUST be rendered alongside keyframe timestamps with exact millisecond alignment.
2. **Given** DOM/SAM candidate scoring yields a tie ($|\Delta	ext{score}| < 0.05$ / `500` bps), **When** running in an interactive terminal session, **Then** the copilot MUST prompt the operator with an interactive locator picker displaying candidate roles, accessible names, and bounding boxes; in non-interactive mode, it MUST fail closed with `AMBIGUOUS_GROUNDING` (Exit Code 22).
3. **Given** live execution is running, **When** Playwright CDP events, console errors, or network roundtrips occur, **Then** they MUST stream as structured `SubstrateEvent` entries to the active observer.

---

### User Story 6 - CI/CD Action & Headless SARIF / JUnit Reporters (Priority: P3)

As a DevOps engineer integrating ReplayFix into GitHub Actions and enterprise CI dashboards, I want ReplayFix to export `results.sarif` (SARIF v2.1.0) and `junit.xml` reports and provide a reusable `action.yml` GitHub Action definition, so that visual bug triage and regression sweeps plug directly into GitHub Code Scanning and CI test summaries.

**Why this priority**: Enables headless enterprise CI/CD adoption (Pillar 6).

**Independent Test**: Run `replayfix repro --bug RF-01 --mock-heal --sarif results.sarif --junit junit.xml` (and `sweep`), validate the generated SARIF v2.1.0 JSON schema and JUnit XML structure, and verify `action.yml` syntax.

**Acceptance Scenarios**:
1. **Given** `--sarif <path>` or `--junit <path>` is passed (or default CI export is triggered), **When** verification or sweep completes, **Then** valid `results.sarif` (SARIF v2.1.0) and `junit.xml` files MUST be written atomically.
2. **Given** `action.yml` in the repository root, **When** inspected, **Then** it MUST define inputs (`video`, `bug`, `repo`, `mode`, `sarif-output`, `junit-output`) and steps to execute headless verification and upload signed triage/audit reports.

---

## 3. Functional Requirements *(mandatory)*

- **FR-001 (Idiomatic Inoculation Synthesis)**: ReplayFix MUST synthesize standalone, executable Playwright TypeScript test files (`tests/repro/rf_<id>.spec.ts`) from verified bug contracts, action trajectories, and grounded accessibility locators (`getByRole`, `getByText`, `getByTestId`), forbidding positional selectors (`.first()`, `.last()`, `.nth()`, `:nth-match`, `:nth-child`).
- **FR-002 (Tamper-Evident Inoculation Registry)**: ReplayFix MUST maintain `.replayfix/inoculations.json` tracking every synthesized inoculation test, its bug ID, target path, test file SHA-256, patch SHA-256, contract SHA-256, and timestamp.
- **FR-003 (MCP JSON-RPC 2.0 Server)**: `replayfix-cli` MUST provide an `mcp` subcommand supporting both `stdio` and `SSE/HTTP` transports adhering to JSON-RPC 2.0 and the Model Context Protocol specification.
- **FR-004 (MCP Tools, Resources & Prompts)**: The MCP server MUST expose the 5 verification tools (`replayfix_reproduce`, `replayfix_verify_patch`, `replayfix_inoculate`, `replayfix_inspect_trace`, `replayfix_doctor`), 3 resources (`replayfix://receipts/{task_id}`, `replayfix://sessions/{session_id}`, `replayfix://benchmarks`), and the `triage_bug` prompt.
- **FR-005 (Autonomous Git Branch & Trailer Packager)**: ReplayFix MUST support packaging a verified patch and inoculation spec into an isolated Git branch (`replayfix/fix-<bug_id>` or `replayfix/inoculate-<bug_id>`), embedding Ed25519-signed `ReplayFix-*` commit trailers and generating an Evidence Matrix PR markdown (`pr_description.md`).
- **FR-006 (Gate 3 Stress & Concurrency Quorum)**: ReplayFix MUST implement Gate 3 perturbation testing (simulated network latency/3G/500ms jitter and a 3-pass randomized-interval concurrency quorum), transitioning any non-deterministic or failing run to `FLAKY_NON_DETERMINISTIC` (Exit Code 21).
- **FR-007 (Multi-Modal Copilot & Ambiguity Picker)**: ReplayFix MUST render ASR acoustic narration aligned with video keyframes, provide an interactive terminal locator picker when candidate grounding scores tie ($|\Delta	ext{score}| < 0.05$ in interactive TTY, failing closed in non-TTY), and stream CDP/console/network events via `SubstrateEvent`.
- **FR-008 (SARIF, JUnit & GitHub Action Automation)**: ReplayFix MUST export valid SARIF v2.1.0 (`results.sarif`) and JUnit XML (`junit.xml`) reports and provide a drop-in GitHub Actions `action.yml`.

---

## 4. Success Criteria *(mandatory)*

- **SC-001 (MCP End-to-End Interoperability)**: An external MCP client can connect over `stdio` or `SSE/HTTP`, discover all 5 tools, 3 resources, and 1 prompt, and execute a full reproduction, patch verification, and inoculation cycle with 100% JSON-RPC 2.0 schema compliance.
- **SC-002 (Inoculation Native Execution Parity)**: 100% of synthesized `rf_<id>.spec.ts` inoculation specs contain valid Playwright TypeScript syntax, use resilient accessibility locators, fail against the unpatched defect state, and pass against the patched state.
- **SC-003 (Gate 3 Flakiness Quarantine & PR Traceability)**: 100% of simulated flaky or latency-intolerant patches are caught by Gate 3 and rejected with Exit Code 21 (`FLAKY_NON_DETERMINISTIC`), while certified patches packaged with `--package-pr` contain cryptographically verifiable `ReplayFix-Merkle-Root` and `ReplayFix-Signature` commit trailers and a complete Evidence Matrix PR description.
- **SC-004 (Zero Regression & Zero-Panic Baseline)**: 100% pass rate across all 251+ existing v1.3/v1.4 tests plus all new v1.5 unit, integration, and adversarial tests; 0 warnings under `cargo clippy --workspace --all-targets -- -D warnings`; and 0 `.unwrap()` / `.expect()` / `panic!()` calls in non-test production code verified by `.specify/scripts/check-panics.ps1`.

---

## 5. Key Entities

- **`InoculationSpec` & `InoculationRegistry`**: Represents a synthesized Playwright test (`tests/repro/rf_<id>.spec.ts`) and its tamper-evident registry entry in `.replayfix/inoculations.json`.
- **`McpRequest`, `McpResponse`, `McpToolCall`, `McpResource`**: Canonical JSON-RPC 2.0 protocol envelopes and MCP domain payloads.
- **`StressQuorumConfig` & `StressQuorumReport`**: Gate 3 network perturbation parameters (latency, jitter, 3G simulation) and 3-pass concurrent execution results.
- **`GitPrPackage` & `CommitTrailers`**: Isolated branch metadata, RFC 6962 + Ed25519 commit trailers, and Evidence Matrix markdown payload.
- **`SarifReport` & `JunitTestSuites`**: Headless CI/CD report artifacts conforming to SARIF v2.1.0 and JUnit XML standards.

---

## 6. Assumptions & Boundaries

- **Zero External Crate Bloat**: JSON-RPC 2.0 (`stdio` and `SSE/HTTP`), SARIF v2.1.0, JUnit XML, Git branch/commit packaging, and Gate 3 concurrency sweeps are implemented using the existing workspace dependencies (`serde`, `serde_json`, `std::net::TcpListener`, `std::thread`, `ed25519-dalek`, `sha2`, `crossterm`).
- **Repository Isolation ($\Delta	ext{repo} = 0$)**: Verification and Gate 3 stress runs occur exclusively inside ephemeral scratch sandboxes. Host repository branch creation (`replayfix/fix-<bug_id>`) or inoculation file writing only occurs when explicitly requested via `--package-pr`, `--inoculate`, or `replayfix inoculate`.
