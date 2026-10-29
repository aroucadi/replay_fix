# 📋 Spec 080: Vertical Slice v0.8.0 — Observability, GitHub Ingestion, Human Approval Gate & Disaggregated Evidence Artifacts

**Feature Identity**: `080-v0.8-observability-ingestion`  
**Target Release**: `v0.8.0`  
**Git Branch**: `release/v0.8.0-observability-ingestion`  
**Constitutional Invariants**:
- Invariant 1 (Substrate Primacy): Deterministic Rust harness is the sole verification authority.
- Invariant 2 (Fail-Closed Verification): Unhandled errors, rate limits, non-interactive stdin without affirmative bypass, or unverified patches fail closed.
- Invariant 4 (Repository Isolation): Ephemeral scratch clones isolate exploration; base repo modification strictly requires verified 2-gate pass AND explicit human confirmation.
- Invariant 10 (Observability Separation & Human Confirmation): Observability tools (Trace Viewer) must not perturb headless CI determinism, and base repo writes require affirmative human approval.

---

## 1. Executive Summary & Value Proposition

ReplayFix v0.7.1 achieved cryptographically fortified differential diagnosis, anti-cheat test freezing, active regression parity, and checkpoint state machines. However, real-world developer workflows demand:
1. **Interactive Trace Scrubbing**: Being able to visually inspect Playwright execution traces step-by-step (`--trace`) without disrupting headless test execution.
2. **Real-World Bug Ingestion**: Ingesting live GitHub issues (`--issue <url>`) into verified task manifests, extracting reproduction steps while treating issue content as strictly untrusted user input.
3. **Human In The Loop Confirmation Gate**: Requiring explicit human review of the syntax-highlighted candidate diff before modifying the base working tree, with fail-closed behavior for non-interactive/CI environments unless explicitly opted in (`--yes`).
4. **Disaggregated Fact Artifacts**: Structuring all evidence, traces, differential diagnosis receipts, and regression reports in an inspectable, isolated `.replayfix/artifacts/` directory.

---

## 2. User Stories & Acceptance Criteria

### User Story 1 (US1) - Playwright Trace Viewer Auto-Launcher (`--trace`)
As a developer or hackathon judge, I want ReplayFix to automatically collect and optionally launch the Playwright Trace Viewer timeline scrubber when `--trace` is provided, so that I can inspect browser DOM snapshots, network requests, CDP commands, and timing.

- **Acceptance Criteria**:
  - When `--trace` is passed, the browser session records execution traces to `.replayfix/artifacts/<task_id>.trace.zip`.
  - Upon run completion, if `--trace` is enabled and standard output is a TTY (interactive session) and `CI` environment variable is not `true`, ReplayFix spawns `npx playwright show-trace <trace_path>` as a non-blocking detached process.
  - In non-interactive or CI environments (`!atty::is(Stream::Stdout)` or `CI=true`), trace viewer auto-launch is cleanly skipped with an informational log message, avoiding hung jobs or CI timeouts.
  - Trace collection failures do not cause harness panics; they fail closed or emit a non-fatal telemetry warning if verification succeeded.

### User Story 2 (US2) - Live GitHub Issue Ingestion (`--issue <url>`)
As an engineer fixing an open-source issue, I want to pass `--issue https://github.com/owner/repo/issues/123` to ReplayFix, so that it automatically ingests the title, issue body, and reproduction steps into a normalized task manifest.

- **Acceptance Criteria**:
  - Strict URL validation: must match `^https://github\.com/([a-zA-Z0-9_.-]+)/([a-zA-Z0-9_.-]+)/issues/(\d+)$`. Pull requests, commits, and arbitrary hosts are rejected immediately (exit code 23).
  - Robust retrieval: retrieves issue JSON via GitHub REST API (or fallback to `gh issue view` if available/configured) with timeouts (default 10s), size limits (max 512KB), and polite User-Agent headers.
  - Untrusted input isolation: the issue body is treated strictly as untrusted context (wrapped in `<UNTRUSTED_ISSUE_BODY>` guards) and never executed directly as shell commands.
  - Extraction: parses markdown headers/lists for steps to reproduce (e.g. `### Steps to Reproduce`, `1. Click X...`) into candidate user actions.
  - Emits normalized `.replayfix/artifacts/issue_ingestion.json`.

### User Story 3 (US3) - Interactive Human Confirmation Gate (`[Y/n]`)
As a repository owner, I want ReplayFix to preview the unified diff with colored syntax highlighting and prompt me for explicit confirmation before writing changes to my base repository, so that no autonomous AI agent modifies my source code without my consent.

- **Acceptance Criteria**:
  - After Gate 2 (Green) and Regression Parity pass, before `apply_patch_to_base()` is invoked, ReplayFix displays the candidate unified diff with ANSI colored formatting (+green, -red, @cyan).
  - Displays summary: files modified, insertions, deletions, and candidate patch SHA-256.
  - Prompts `Apply patch to base repository? [y/N]: `.
  - In interactive TTY: requires affirmative `y` or `yes` (case-insensitive). Refusal (`n` or blank Enter for default No) aborts the write cleanly with exit code 12 (`UserRejectedPatch`), leaving base repo unchanged ($\Delta\text{repo}=0$).
  - In non-interactive environments: if `--yes` flag is passed, automatically confirms; if `--yes` is absent in non-interactive environment, fails closed with exit code 13 (`NonInteractiveRefusal`), preserving $\Delta\text{repo}=0$.
  - Generates `.replayfix/artifacts/approval_audit.json` recording user decision, timestamp, TTY mode, and patch SHA-256.

### User Story 4 (US4) - Disaggregated Fact Artifacts
As an auditor or CI pipeline, I want all verification facts, traces, diffs, and receipts disaggregated into individual, well-structured files in `.replayfix/artifacts/`, so that each aspect of the run can be consumed or verified independently.

- **Acceptance Criteria**:
  - Emits:
    - `candidate.patch` (exact unified diff)
    - `differential_diagnosis.json` (root cause, failure pattern, stability)
    - `regression_report.json` (baseline suite comparison)
    - `approval_audit.json` (human approval or rejection record)
    - `issue_ingestion.json` (when `--issue` used)
    - `playwright.trace.zip` (when `--trace` used)
    - `audit_receipt.json` (final verification receipt)

---

## 3. Functional Requirements (FR-001 through FR-010)

- **FR-001 (Trace Option)**: Add `--trace` boolean flag to `replayfix repro` / `replayfix run`.
- **FR-002 (Trace Path & Packaging)**: Playwright context records trace into `.replayfix/artifacts/<bug_id>_trace.zip`.
- **FR-003 (Trace Launcher)**: If `--trace` is active and interactive stdout is detected, invoke trace viewer via `npx playwright show-trace` detached from harness process.
- **FR-004 (Issue Option)**: Add `--issue <url>` argument to `replayfix repro` / `replayfix run`.
- **FR-005 (Issue URL Validation)**: Enforce strict GitHub issue URL regex; reject non-GitHub hosts, PR URLs, and path traversals.
- **FR-006 (Issue Fetch & Timeout)**: Fetch issue via HTTPS with 10s timeout, 512KB max size, and rate-limit detection (HTTP 403 / 429).
- **FR-007 (Reproduction Step Parser)**: Extract numbered or bulleted steps following headers like "Steps to reproduce", "Repro", or similar common variants.
- **FR-008 (Diff Highlighting)**: Format unified diff with ANSI escape codes for terminal display, showing line numbers and file headers.
- **FR-009 (Interactive Confirmation Prompt)**: Prompt user for affirmative confirmation before base repo mutation; support `--yes` flag for automated/scripted workflows.
- **FR-010 (Artifact Disaggregation)**: Write individual artifacts into `.replayfix/artifacts/` with atomic writes.

---

## 4. Exit Codes Taxonomy

| Exit Code | Identifier | Description |
|-----------|------------|-------------|
| 0 | `Success` | Harness verified bug reproduction and repair successfully |
| 1 | `DoctorFailed` | Environment doctor dependency check failed |
| 2 | `UsageOrInputError` | Invalid CLI arguments or missing files |
| 10 | `InternalOrSystemError` | Unhandled internal or system error |
| 11 | `SecurityViolation` | Path traversal or sandbox boundary escape |
| 12 | `UserRejectedPatch` | Interactive human user declined to apply patch (`[y/N]`) |
| 13 | `NonInteractiveRefusal` | Base repo modification requested in non-interactive environment without `--yes` |
| 20 | `BridgeExecutionFailed` | Playwright or CDP bridge process failure |
| 21 | `QuarantineRepairFailed` | Autonomous model repair failed after maximum retry budget |
| 22 | `AmbiguousGrounding` | Multimodal visual grounder tie (|ΔS| < 0.25) |
| 23 | `InvalidIssueUrl` | Provided `--issue` URL is invalid, malformed, or not a GitHub issue |
| 24 | `IssueFetchFailed` | Failed to retrieve issue (HTTP 404, rate limited, network timeout) |
| 30 | `RepairRejected` | Candidate patch failed Gate 2 validation |
| 31 | `RegressionDetected` | Candidate patch broke pre-existing regression test suite |
| 32 | `QuarantineInvertedBaseline` | Gate 1 baseline was already green (inverted defect) or DiffGuard violated |
