# Feature Specification: 040-v0.4-playwright-cdp-grounding — Playwright CDP Grounding & Live Browser Verification

**Feature Branch**: `release/v0.4.0-playwright-cdp`
**Created**: 2026-10-27
**Status**: Ready for Planning
**Target Release**: `v0.4.0`
**Input**: Playwright Chromium CDP spatial grounding, `document.elementFromPoint(x, y)` reverse probing, `ariaSnapshot` accessibility role resolution, spatial-semantic locator scoring without blind `.first()` fallbacks, fail-closed ambiguity abstention (`AMBIGUOUS_GROUNDING` when $|\Delta S| < 0.25$), and live browser two-gate execution inside the ephemeral scratch sandbox (`--live-browser` and `--headed`).

---

## Scope Statement (read first)

v0.4 introduces **real browser verification and deterministic DOM grounding** alongside the existing simulated oracle. When `--live-browser` is passed to `replayfix repro`, the harness serves the target fixture application from inside the isolated `ScratchSandbox` on an OS-assigned `EphemeralPort` (`127.0.0.1:<port>`), launches Chromium via a line-delimited JSON-lines Playwright bridge (headless by default, or visible on the desktop with `--headed`), reverse-probes the click coordinate `(x, y)` via `document.elementFromPoint` and `ariaSnapshot`, verifies the **DOM Grounding Invariant** (no blind `.first()`, fail-closed when candidate score margin $|\Delta S| < 0.25$), and executes Gate 1 (RED) → disk patch in sandbox → Gate 2 (GREEN) against the live browser.

When `--mock-heal` is used without `--live-browser`, the fast deterministic simulated oracle from v0.3 continues to work unchanged for offline and unit workflows. Video keyframe extraction and SAM 3.1 segmentation arrive in Spec 050 (v0.5); model-generated patches arrive in Spec 060 (v0.6).

---

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Deterministic Spatial-Semantic Locator Grounding & Ambiguity Guard (Priority: P1)

As an engineer reproducing a visual UI defect from click coordinates `(x, y)`, I want ReplayFix to resolve those coordinates against the live DOM and accessibility snapshot using spatial distance and semantic role specificity—and to fail closed whenever the target is missing or ambiguous—so that generated browser actions never silently click the wrong element or rely on brittle `.first()` selectors.

**Why this priority**: Constitutional Law I (Substrate Primacy), Law II (Contract Invariance), and Operational Invariant 8/9 require every browser interaction to be grounded in physical DOM reality before Gate 1 or Gate 2 can be trusted.

**Independent Test**: Run `cargo test -p replayfix-core -- locator` to verify that `LocatorResolver` selects the interactive target enclosing `(x, y)`, synthesizes a resilient Playwright selector (`getByRole` / `getByTestId` / `getByLabel`) without `.first()`, rejects ungrounded candidates that lack a matching DOM/accessibility presence, and returns `AMBIGUOUS_GROUNDING` (exit code `22`) when the top two distinct candidates have score difference $|\Delta S| < 0.25$.

**Acceptance Scenarios**:
1. **Given** a target click coordinate `(x, y)` and a set of DOM/accessibility candidates from `elementFromPoint(x, y)` + `.closest(...)` and `ariaSnapshot`, **When** a single interactive element (such as `button "Checkout"` with `[data-testid="cart-checkout"]`) encloses `(x, y)` with a clear score margin $|\Delta S| \ge 0.25$, **Then** `LocatorResolver` returns a `GroundedTarget` with `element_from_point_match: true`, `accessible_name_match: true`, `candidate_score` in `[0.0, 1.0]`, and a resilient selector free of `.first()`.
2. **Given** two or more interactive candidates near `(x, y)` whose spatial-semantic scores differ by $|\Delta S| < 0.25$, **When** grounding is evaluated, **Then** the resolver fails closed with `TriageStatus::AmbiguousGrounding` (`AMBIGUOUS_GROUNDING`, exit code `22`) and writes `.replayfix/triage_report.json` listing the competing candidates rather than guessing.
3. **Given** a coordinate `(x, y)` that hits non-interactive background or a candidate whose role/name cannot be verified in the live DOM snapshot (**DOM Grounding Invariant**), **When** grounding is evaluated, **Then** the resolver rejects the ungrounded target and fails closed with `AMBIGUOUS_GROUNDING` (exit code `22`).
4. **Given** any synthesized selector string, **When** validated by the locator guard, **Then** any selector containing `.first()` or `:nth-match` blind indexing is rejected.

---

### User Story 2 - Live Playwright Browser Bridge & Two-Gate Verification (Priority: P2)

As a developer or hackathon judge verifying a bug fix, I want to run `replayfix repro --live-browser` (and optionally `--headed` to watch the Chromium window live on my desktop) so that Gate 1 reproduces the real runtime `TypeError` in Chromium on the unpatched scratch sandbox and Gate 2 proves the patched application renders the expected cart count in Chromium without errors.

**Why this priority**: Replaces static source inspection with live browser execution over an ephemeral localhost server (Constitutional Law IV & Law VI).

**Independent Test**: Run `replayfix repro --task-id RF-01 --bug-ref bug/RF-01 --video recordings/RF-01.mp4 --live-browser`. The harness serves the `RF-01` app from the scratch sandbox on `127.0.0.1:<ephemeral_port>`, runs Chromium via the Playwright bridge, observes the unhandled `TypeError` at Gate 1 (RED), applies the patch in the sandbox, verifies the updated DOM state at Gate 2 (GREEN) with zero page errors, verifies $\Delta \text{repo} = 0$, and writes `.replayfix/audit_receipt.json` with `substrate: "playwright-cdp"` and `grounding_telemetry`.

**Acceptance Scenarios**:
1. **Given** Playwright and Chromium are installed, **When** `replayfix repro --task-id RF-01 --bug-ref bug/RF-01 --video recordings/RF-01.mp4 --live-browser` runs, **Then** Gate 1 launches the sandbox server on the allocated ephemeral port, reverse-probes the checkout button at the target hotspot, clicks the grounded locator in Chromium, captures the runtime `TypeError: Cannot read properties of undefined (reading 'length')`, and records Gate 1 as `RED`.
2. **Given** Gate 1 is `RED` in live browser mode, **When** the candidate patch is applied inside the scratch sandbox and Gate 2 runs against the live browser, **Then** clicking the same grounded locator updates the cart summary text to `"Items: 0"` with zero page errors, Gate 2 is recorded as `GREEN`, and `.replayfix/audit_receipt.json` is written with exit code `0`.
3. **Given** `--headed` is passed alongside `--live-browser`, **When** the bridge launches Chromium, **Then** Chromium launches with `headless: false` so the user can visually observe the navigation, click, and repair verification on the desktop.
4. **Given** a candidate patch that leaves the runtime error unfixed in `--live-browser` mode, **When** Gate 2 executes in Chromium, **Then** the run fails closed with `REPAIR_GATE2_NOT_GREEN` (exit code `30`) and writes `.replayfix/triage_report.json`.

---

### User Story 3 - Line-Delimited JSON-Lines IPC & Ephemeral Server Lifecycle (Priority: P3)

As a developer running ReplayFix across Windows, macOS, and Linux, I want the Node.js Playwright bridge and the sandbox HTTP server to communicate via deterministic, flushed line-delimited JSON and to be cleanly terminated on completion or error, so that stdio pipes never hang on Windows and no orphan `node` or `chromium` processes remain after a run.

**Why this priority**: Cross-platform reliability and zero process leaks are mandatory for a trustworthy local developer tool.

**Independent Test**: `cargo test -p replayfix-core -- browser` verifies JSON-lines request/response serialization, explicit newline framing, timeout/error handling, and clean teardown of the sandbox HTTP server and child process tree.

**Acceptance Scenarios**:
1. **Given** the Rust harness starts a live browser session, **When** commands (`START_BROWSER`, `NAVIGATE`, `REVERSE_PROBE`, `CLICK_AND_ASSERT`, `CLOSE_BROWSER`) are sent over stdin/stdout, **Then** each message is a single UTF-8 JSON line terminated with `\n` and flushed immediately.
2. **Given** a live browser run completes or aborts at any gate, **When** `ScratchSandbox` tears down, **Then** both the local sandbox HTTP server and the Playwright bridge process tree are terminated cleanly and $\Delta \text{repo} = 0$ is verified on the base repository.

---

### Edge Cases

- **Ambiguous candidate tie ($|\Delta S| < 0.25$)**: Terminates with `AMBIGUOUS_GROUNDING` (exit code `22`) and writes `.replayfix/triage_report.json` with the candidate details.
- **Coordinate outside viewport or negative coordinates**: Fails closed with `AMBIGUOUS_GROUNDING` (exit code `22`) before dispatching a click.
- **Playwright bridge startup failure or missing browser binary**: Fails closed with a clear diagnostic message and writes `.replayfix/triage_report.json` (`BRIDGE_EXECUTION_FAILED`, exit code `20`) telling the user to run `npx playwright install chromium`.
- **`--mock-patch` or `--live-browser` flags without a valid task**: Preserves existing usage validation (exit code `2` on invalid CLI combinations, exit code `1` on unsupported fixture task IDs).
- **Backward compatibility**: All existing `--mock-heal` invocations, flags, and tests continue to pass without requiring a browser launch when `--live-browser` is not set.

---

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: `replayfix_core::locator` MUST define `DomCandidate`, `BoundingBox`, `GroundingTelemetry`, and `GroundedTarget`, scoring candidates by spatial proximity to `(x, y)` (point containment + normalized center distance) and semantic accessibility specificity (`role`, `accessible_name`, `test_id`) while rejecting any selector containing `.first()` or `:nth-match`.
- **FR-002**: `replayfix_core::locator::LocatorResolver` MUST enforce the **DOM Grounding Invariant** and **Fail-Closed Ambiguity Guard**: if no interactive candidate matches `(x, y)` in the live snapshot, or if the top two candidates have score difference $|\Delta S| = |S_1 - S_2| < 0.25$, it MUST fail closed with `TriageStatus::AmbiguousGrounding` (`AMBIGUOUS_GROUNDING`, exit code `22`).
- **FR-003**: `replayfix_core::browser` MUST provide an embedded, deterministic local HTTP fixture server bound to the allocated `EphemeralPort` (`127.0.0.1:<port>`) serving the `ScratchSandbox` application files, plus a line-delimited JSON-lines stdio client for the Node.js Playwright bridge (`playwright_bridge.js`).
- **FR-004**: The Playwright bridge MUST implement `document.elementFromPoint(x, y)` + `.closest('button, a, input, select, textarea, [role="button"], [role="link"], [role="checkbox"], [role="menuitem"], [data-testid]')` reverse probing, capture `page.locator('body').ariaSnapshot()` (or role tree fallback), execute grounded clicks, and capture page runtime exceptions (`pageerror`) and post-click DOM state.
- **FR-005**: `replayfix repro` MUST support `--live-browser` (and honor `--headed` for visible desktop Chromium execution), running Gate 1 (RED), disk patch in `ScratchSandbox`, and Gate 2 (GREEN) against the live browser while enforcing $\Delta \text{repo} = 0$ on the base repository.
- **FR-006**: `AuditReceipt` (schema version `"0.4"`) MUST record `substrate` (`"playwright-cdp"` for live browser runs, `"mock-static-oracle"` for simulated runs) and optional `grounding_telemetry` (`viewport_normalized`, `element_from_point_match`, `accessible_name_match`, `candidate_score`, `matched_selector`, `score_margin`, `aria_snapshot_excerpt`).
- **FR-007**: `TriageStatus` MUST include `AmbiguousGrounding` (`AMBIGUOUS_GROUNDING`, exit code `22`) and `BridgeExecutionFailed` (`BRIDGE_EXECUTION_FAILED`, exit code `20`).

### Key Entities

- **BoundingBox / DomCandidate**: Geometry (`x`, `y`, `width`, `height`) and accessibility metadata (`tag`, `role`, `accessible_name`, `test_id`, `is_element_from_point`) extracted from the live page.
- **GroundedTarget / GroundingTelemetry**: Verified locator resolution result with `matched_selector`, `candidate_score`, `score_margin`, `element_from_point_match`, `accessible_name_match`, and `aria_snapshot_excerpt`.
- **PlaywrightBridge / SandboxServer**: Localhost HTTP server serving the `ScratchSandbox` on `127.0.0.1:<ephemeral_port>` and the Node.js Playwright child process communicating over JSON-lines stdio.

---

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: `replayfix repro --task-id RF-01 --bug-ref bug/RF-01 --video recordings/RF-01.mp4 --live-browser` executes Gate 1 (RED) and Gate 2 (GREEN) in a real Chromium browser against the scratch sandbox, exits `0`, leaves the base repository with $\Delta \text{repo} = 0$, and emits `.replayfix/audit_receipt.json` with `substrate: "playwright-cdp"` and `grounding_telemetry`.
- **SC-002**: Ambiguous candidate sets with $|\Delta S| < 0.25$ or ungrounded coordinates deterministically abort with `AMBIGUOUS_GROUNDING` (exit code `22`) and write `.replayfix/triage_report.json`.
- **SC-003**: 100% of unit and integration tests pass via native `cargo test --workspace`, with zero Clippy warnings (`-D warnings`) and clean `cargo fmt --all -- --check`.
