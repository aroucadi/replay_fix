# Feature Specification: Spec 160 — Sovereign Autonomous Healer: Multi-Turn Muse Spark Agent Loop & Sub-60s Macro Pipeline (Release 1.6.0)

**Feature Branch**: `release/v1.6.0`  
**Created**: 2026-11-02  
**Status**: Ready for Planning & Implementation  
**Input**: Establish Meta Muse Spark 1.3 as the sovereign multi-turn agent loop brain driving autonomous reproduction, causal diagnosis, code repair delegation (to Meta Muse Wonder 1.0 with encrypted deliberation backtracking), sandbox verification, and permanent inoculation within a sub-60-second end-to-end CLI macro (`replayfix heal`).

---

## User Scenarios & Testing *(mandatory)*

### User Story 1 — Meta Muse Spark 1.3 Sovereign Multi-Turn Agent Loop & Autonomous Tool Calling (Priority: P1)

As a ReplayFix user running visual bug reproduction and repair, I want Meta Muse Spark 1.3 (`305 tok/s`, `reasoning: { effort: "xhigh" }`) to act as the primary cognitive agent loop orchestrator that maintains unbroken conversational and reasoning continuity across turns, autonomously issuing structured tool calls to the Rust harness (`replayfix_reproduce`, `replayfix_inspect_trace`, `replayfix_verify_patch`, `replayfix_inoculate`), receiving typed JSON observations, and deciding next steps without human intervention.

**Why this priority**: Directly fulfills Meta's core hackathon mandate ("Build Agent Loops with Muse Spark") by establishing Muse Spark as the autonomous multi-turn reasoning and tool-calling general contractor.

**Independent Test**: Execute the multi-turn agent loop with a synthetic or live defect session and verify that Muse Spark executes sequential turns (Turn 1: Grounding & Reproduction, Turn 2: Causal Diagnosis & Delegation, Turn 3: Sandbox Verification, Turn 5: Permanent Inoculation), records structured tool calls in telemetry, and preserves logical context across turns with `prompt_cache_key: "replayfix-v1.6-spark-agent-loop"`.

**Acceptance Scenarios**:
1. **Given** a multimodal bug brief (video keyframes, SAM 3.1 contours, Muse Voice transcript), **When** `DeliberationEngine` starts the loop, **Then** Muse Spark issues `replayfix_reproduce`, parses the Gate 1 RED failure, and extracts causal failure symptoms.
2. **Given** Gate 1 RED confirmation, **When** Muse Spark formulates a repair hypothesis, **Then** it delegates AST code synthesis to Meta Muse Wonder 1.0 while maintaining turn state.
3. **Given** a synthesized patch, **When** Muse Spark calls `replayfix_verify_patch`, **Then** the Rust harness executes Gate 2 and Gate 3 in an ephemeral isolated sandbox ($\Delta\text{repo}=0$) and returns the structured verdict to Spark.

---

### User Story 2 — Meta Muse Wonder 1.0 AST Code Generation with Multi-Turn Encrypted Deliberation Backtracking (Priority: P1)

As an autonomous repair harness attempting to fix an application defect, I want Meta Muse Wonder 1.0 to synthesize surgical, regression-free AST search/replace patches conforming strictly to `{ explanation, target_file, search_block, replace_block, touched_symbols }`, and when a candidate patch fails Gate 2 compilation/assertion or Gate 3 stress quorum, I want the harness to pass `previous_response_id` and cached `reasoning.encrypted_content` back to Wonder alongside compiler/assertion error diffs, preserving its deep AST deliberation graph across turns without re-analyzing the codebase from scratch.

**Why this priority**: Leverages Wonder's frontier coding power while using Meta's native encrypted deliberation caching to achieve high-speed, stateful self-correction without context thrashing.

**Independent Test**: Simulate an initial syntax or assertion error on Attempt 1 and verify that Attempt 2 provides `previous_response_id` and `encrypted_content`, allowing Wonder to self-correct and produce a valid, passing patch on Attempt 2.

**Acceptance Scenarios**:
1. **Given** Attempt 1 of code repair, **When** the harness queries Meta Muse Wonder 1.0, **Then** the request specifies `include: ["reasoning.encrypted_content"]` and records the resulting `response_id` and encrypted deliberation token hash.
2. **Given** Gate 2 fails with a compiler or assertion failure on Attempt 1, **When** Attempt 2 is initiated, **Then** the request threads `previous_response_id` and `encrypted_content` with compiler diagnostics, allowing Wonder to backtrack and emit a corrected patch.

---

### User Story 3 — Sub-60s Sovereign End-to-End Macro Pipeline (`replayfix heal`) (Priority: P1)

As a developer or hackathon judge, I want to run a single command:
```powershell
replayfix heal --video <path_to_video.mp4> --repo <path_to_repo>
```
and have ReplayFix autonomously execute the full end-to-end pipeline:
`Video Ingestion` ➔ `FFmpeg Keyframe Extraction` ➔ `SAM 3.1 Centroid Snapping` ➔ `Voice Intent Lock` ➔ `Ephemeral Server (:0)` ➔ `Spark Gate 1 RED` ➔ `Wonder AST Repair` ➔ `Spark Gate 2 GREEN` ➔ `Gate 3 Stress Quorum` ➔ `Spark Permanent Inoculation` ➔ `Signed Merkle Receipt` + `proof_viewer.html` export in **under 60 seconds**.

**Why this priority**: Provides the definitive "magic moment" for the hackathon demo, taking raw video to verified, inoculated, cryptographically signed repair in a single fast command.

**Independent Test**: Run `replayfix heal --video tests/fixtures/rf01_drag_drop_telemetry.mp4 --repo tests/fixtures/metapulse_reference --bug RF-01 --mock-heal` (and live equivalent) and verify completion with exit code 0, generated `audit_receipt.json`, generated `tests/repro/rf_01.spec.ts`, and wall-clock time `< 60s`.

**Acceptance Scenarios**:
1. **Given** a valid video file and target repository path, **When** `replayfix heal` is executed, **Then** the macro pipeline runs without intermediate prompts, completes all phases, and exits with code 0 on success.
2. **Given** benchmark defect suites (RF-01 through RF-05), **When** evaluated under `heal`, **Then** the median execution duration is under 60 seconds.

---

### User Story 4 — Autonomous Compiler & Assertion Error Backtracking with Gate 3 Feedback (Priority: P1)

As an autonomous agent repairing complex bugs, I want compiler errors, TypeScript diagnostics, and Gate 3 stress quorum race condition warnings (e.g. non-deterministic timeouts under throttled network conditions) to be structured and fed back into Muse Spark and Muse Wonder, so that the agent loop can autonomously apply event listeners, guards, or debouncers to eliminate flakiness.

**Why this priority**: Eliminates brittle one-shot repair failures and prevents flaky patches from passing into production.

**Independent Test**: Inject a patch that introduces a race condition under 3G latency and verify that Gate 3 fails, the flakiness report is fed back to the deliberation engine, and the next turn synthesizes a patch that stabilizes under Gate 3 stress quorum.

**Acceptance Scenarios**:
1. **Given** a candidate patch that compiles but fails Gate 3 stress quorum (e.g. 1 of 5 runs fails under network throttle), **When** the harness detects flakiness, **Then** it feeds the exact timing/race diagnosis into Turn 4 of the deliberation engine.
2. **Given** repairs fail across all 3 attempts or exceed 180s, **When** the loop terminates, **Then** it fails closed, creates a quarantine triage report (`triage_report.json`), and exits with code 21.

---

### User Story 5 — Offline Proof Portal (`proof_viewer.html`) Multi-Turn Deliberation Timeline (Priority: P2)

As a hackathon judge or staff engineer reviewing an autonomous repair, I want to open `proof_viewer.html` offline in any browser and inspect an interactive, step-by-step deliberation timeline showing Muse Spark's turns, tool calls, and observations, side-by-side pre/post `ariaSnapshot` DOM diffs, and cryptographic RFC 6962 Merkle tree leaves with Ed25519 signature verification.

**Why this priority**: Delivers an airtight, transparent visual demonstration of the multi-turn agent loop for competition evaluation.

**Independent Test**: Load `proof_viewer.html` with an `audit_receipt.json` from a `heal` run and verify the Deliberation Timeline renders all turns, tool call arguments, DOM diffs, and cryptographic verification status.

**Acceptance Scenarios**:
1. **Given** a generated `audit_receipt.json` containing `deliberation_turns`, **When** loaded into `proof_viewer.html`, **Then** the portal visualizes each turn chronologically with model badge, tool name, inputs, outputs, and elapsed time.
2. **Given** Gate 1 and Gate 2 DOM snapshots, **When** viewing the DOM Diff section, **Then** the viewer renders a syntax-highlighted side-by-side comparison of the accessibility trees.

---

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001 (Multi-Turn Muse Spark Agent Loop)**: `DeliberationEngine` MUST orchestrate multi-turn interactions using model slug `muse-spark-1.3`, `reasoning: { effort: "xhigh" }`, and `prompt_cache_key: "replayfix-v1.6-spark-agent-loop"`, maintaining turn memory across reproduction, diagnosis, repair, and verification.
- **FR-002 (Autonomous Tool Schema & Dispatch)**: The deliberation harness MUST provide 4 canonical tools to Muse Spark:
  1. `replayfix_reproduce`: Runs perception grounding and Gate 1 execution to establish the RED failure baseline.
  2. `replayfix_inspect_trace`: Inspects Playwright CDP trace, console logs, and network events for root cause analysis.
  3. `replayfix_verify_patch`: Applies candidate patch to ephemeral scratch sandbox ($\Delta\text{repo}=0$) and executes Gate 2 stability pass and Gate 3 stress quorum.
  4. `replayfix_inoculate`: Synthesizes permanent Playwright regression test (`tests/repro/rf_<id>.spec.ts`) using grounded ARIA locators.
- **FR-003 (Meta Muse Wonder 1.0 Code Synthesis with Deliberation Caching)**: When Muse Spark triggers code repair, it MUST delegate patch synthesis to `muse-wonder-1.0`. The request MUST request `include: ["reasoning.encrypted_content"]`. On retries, `previous_response_id` and the encrypted reasoning token hash MUST be supplied to maintain stateful deliberation.
- **FR-004 (Sub-60s Sovereign Macro Command `heal`)**: `replayfix-cli` MUST provide a top-level subcommand `heal` taking `--video <path>`, `--repo <path>`, optional `--bug <id>`, `--mode <mode>` (or `--mock-heal` / `--live-browser`), `--output <dir>`, and `--json`. The command MUST execute the integrated chain and enforce an internal 60-second optimization budget.
- **FR-005 (Autonomous Error Correction & Backtracking)**: The harness MUST capture TypeScript syntax errors, Playwright assertion diffs, and Gate 3 stress quorum failures, injecting them into the next turn context for up to 3 repair attempts. If exhausted, it MUST emit `triage_report.json` and exit with code 21.
- **FR-006 (Offline Proof Portal Deliberation Timeline)**: `proof_viewer.html` and `portal.rs` MUST format multi-turn deliberation events (turns, tool calls, observations, execution times) into `audit_receipt.json` and render an interactive timeline, side-by-side DOM diff, and cryptographic Merkle proof tree.

---

## Success Criteria *(mandatory)*

- **SC-001 (Autonomous Macro End-to-End Execution)**: Running `replayfix heal --video <path> --repo <path>` succeeds end-to-end with exit code 0, achieving Gate 1 RED ➔ Gate 2 GREEN ➔ Gate 3 Stress Quorum ➔ Inoculation ➔ RFC 6962 signed Merkle receipt export within `< 60s` on standard benchmark tests.
- **SC-002 (Stateful Deliberation Continuity)**: Retry repair attempts verifiably transmit `previous_response_id` and cached `reasoning.encrypted_content`, as recorded in `AttemptLineage` and `audit_receipt.json`.
- **SC-003 (Substrate Invariance & Zero Regression)**: Base repository remains strictly unmodified ($\Delta\text{repo}=0$). 100% pass parity is maintained across all 283+ existing unit and integration tests plus new v1.6 tests.
- **SC-004 (Zero Panics & Clean Code)**: `cargo clippy --workspace --all-targets -- -D warnings` compiles with 0 warnings, and `check-panics.ps1` confirms 0 non-test panics.
