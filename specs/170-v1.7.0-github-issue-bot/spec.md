# Feature Specification: Spec 170 — Autonomous GitHub Issue Ingestion Bot, Automated PR Dispatch & Continuous Triage Engine (Release 1.7.0)

**Feature Branch**: `release/v1.7.0`  
**Created**: 2026-11-02  
**Status**: Implemented & Verified (Release 1.7.0)  
**Input**: Evolve ReplayFix from a local CLI sovereign healer into an autonomous GitHub continuous triage agent (`replayfix bot`) that ingests GitHub issues and attached bug videos (`.mp4`, `.webm`, `.mov`), runs the v1.6 sovereign healing pipeline inside an ephemeral scratch sandbox ($\Delta	ext{repo}=0$), dispatches signed Pull Requests (`replayfix/issue-<id>-<slug>`) with permanent Playwright inoculation specs (`tests/repro/rf_issue_<id>.spec.ts`), posts verifiable Evidence Matrix comments and triage labels (`replayfix:verified-healed`, `replayfix:unreproduced`, `replayfix:ambiguous`), and exposes an HMAC-SHA256 authenticated webhook server (`replayfix bot --listen <addr>`).

---

## User Scenarios & Testing *(mandatory)*

### User Story 1 — Multi-Modal GitHub Issue Video Ingestion & Narrative Extraction (Priority: P1)

As a maintainer or automated CI workflow running `replayfix bot --issue <url>`, I want ReplayFix to connect to GitHub's REST API v3 / GraphQL or local `gh` credentials (`GITHUB_TOKEN`, `GH_TOKEN`), parse the issue markdown and comments to extract attached bug recordings (`https://github.com/user-attachments/assets/...`, `.mp4`, `.webm`, `.mov`), download and validate the media into `.replayfix/scratch/issue_<number>.mp4`, and extract the sanitized narrative context to seed the Sovereign Healer.

**Why this priority**: Bridges user-reported GitHub issues directly into ReplayFix's multimodal perception and healing pipeline without manual video downloading or manifest authoring.

**Independent Test**: Invoke `GitHubClient` and `extract_issue_video_urls` against realistic GitHub issue markdown containing `user-attachments/assets` and `.mp4`/`.webm`/`.mov` URLs, verify media caching into `.replayfix/scratch/issue_<number>.mp4` with container header validation, and verify untrusted prompt-injection sanitization.

**Acceptance Scenarios**:
1. **Given** a GitHub issue URL (`https://github.com/org/repo/issues/123`) containing markdown video links or `<video src="...">` tags, **When** `GitHubClient::ingest_issue_bundle` executes, **Then** all valid video URLs are extracted, downloaded into `.replayfix/scratch/issue_123.mp4`, and validated for container integrity.
2. **Given** an issue description with reproduction steps and comments, **When** narrative extraction runs, **Then** untrusted text is sanitized against delimiter breakout and packaged into a structured `IssueTriageBundle`.

---

### User Story 2 — Ephemeral Scratch Triage & Sovereign v1.6 Healing Orchestration (Priority: P1)

As a repository owner, I want `replayfix bot` to execute the entire reproduction, Meta Muse Spark 1.3 multi-turn deliberation, Meta Muse Wonder 1.0 AST repair, Gate 2 GREEN stability pass, Gate 3 concurrency stress quorum, and permanent inoculation inside an isolated `ScratchSandbox` ($\Delta	ext{repo}=0$) within a 120-second SLA budget.

**Why this priority**: Guarantees that autonomous issue triage never contaminates the host working tree while leveraging the full v1.6 Sovereign Healer capability.

**Independent Test**: Run `IssueTriageRunner` on a target repository for issue `#123` (`RF-01`), verify `WorktreeManifest` pre/post invariance ($\Delta	ext{repo}=0$) on the base repository, and confirm generation of `tests/repro/rf_issue_123.spec.ts` and `audit_receipt.json` in `< 120s`.

**Acceptance Scenarios**:
1. **Given** an ingested `IssueTriageBundle`, **When** `IssueTriageRunner::run_triage` executes, **Then** it clones/copies the repository into an isolated `ScratchSandbox`, runs the Sovereign v1.6 healing loop, and verifies $\Delta	ext{repo}=0$ on the host repo.
2. **Given** a healed defect for issue `#123`, **When** inoculation runs, **Then** a permanent Playwright spec is synthesized at `tests/repro/rf_issue_123.spec.ts` with grounded ARIA role locators.

---

### User Story 3 — Autonomous Git Pull Request Dispatch Engine (Priority: P1)

As a maintainer, when ReplayFix verifies a causal repair (Gate 1 RED $	o$ Gate 2 GREEN $	o$ Gate 3 PASSED), I want it to automatically create an isolated branch `replayfix/issue-<number>-<slug>`, commit the code repair and `tests/repro/rf_issue_<number>.spec.ts` with Ed25519-signed RFC 6962 Merkle commit trailers (`Fixes #<number>`, `ReplayFix-Issue`, `ReplayFix-Merkle-Root`, `ReplayFix-Signature`, `ReplayFix-Gate1-Status`, `ReplayFix-Gate2-Status`, `ReplayFix-Gate3-Status`), and open a pull request with a comprehensive Evidence Matrix description.

**Why this priority**: Transforms verified repairs into review-ready, cryptographically auditable Pull Requests without human intervention.

**Independent Test**: Execute `BotPrDispatcher` for issue `#123` with title `"Cart total does not refresh on click"`, verify branch name `replayfix/issue-123-cart-total-does-not-refresh-on-click`, verify commit message trailers and Ed25519 signature verification, and verify PR creation payload.

**Acceptance Scenarios**:
1. **Given** a verified `AuditReceipt` and `InoculationSpec` for issue `#123`, **When** `BotPrDispatcher` packages the fix, **Then** the commit includes `Fixes #123` and all signed `ReplayFix-*` trailers.
2. **Given** a configured GitHub API endpoint or `gh` CLI, **When** PR dispatch runs, **Then** it submits the branch and Evidence Matrix markdown to open a Pull Request and records the PR URL/number.

---

### User Story 4 — Issue Thread Evidence Commenter & Automated Triage Labeling (Priority: P1)

As an issue reporter or repository triager, I want ReplayFix to post a structured markdown comment directly on the original GitHub issue containing the executive verification summary, keyframe bounding box preview, side-by-side DOM `ariaSnapshot` diff (Defect vs. Repaired), link to the Pull Request, and offline `replayfix verify-receipt` command, while automatically applying the appropriate triage label (`replayfix:verified-healed`, `replayfix:unreproduced`, or `replayfix:ambiguous`).

**Why this priority**: Closes the feedback loop on the issue thread with transparent visual and cryptographic evidence, or clear root-cause diagnostics when a bug cannot be reproduced.

**Independent Test**: Test `IssueCommenter::format_verified_comment` and `format_failure_comment` across `VerifiedHealed`, `Unreproduced`, and `Ambiguous` outcomes, verifying the exact label assignment (`replayfix:verified-healed`, `replayfix:unreproduced`, `replayfix:ambiguous`), anti-loop bot marker, and side-by-side `ariaSnapshot` diff table.

**Acceptance Scenarios**:
1. **Given** a verified repair with PR `#45`, **When** the commenter runs, **Then** it posts the structured Evidence Matrix comment linking to PR `#45` and applies label `replayfix:verified-healed`.
2. **Given** an unreproduced issue (Gate 1 GREEN) or ambiguous grounding, **When** triage concludes, **Then** it posts a diagnostic comment explaining the root cause with `triage_report.json` summary and applies `replayfix:unreproduced` or `replayfix:ambiguous`.

---

### User Story 5 — Secure Webhook Server (`replayfix bot --listen <addr>`) & GitHub Actions Composite Runner (Priority: P1)

As a platform security engineer deploying ReplayFix continuously, I want `replayfix bot --listen <addr>` to run a bounded HTTP webhook listener for `issues.opened` and `issue_comment.created` events that enforces constant-time HMAC-SHA256 verification (`X-Hub-Signature-256`) using `GITHUB_WEBHOOK_SECRET`, rejects unauthenticated requests with HTTP 401, filters out bot-authored comments to prevent infinite recursion loops, and pairs with a drop-in GitHub Actions workflow (`action.yml`).

**Why this priority**: Enables both continuous daemon deployment and zero-infrastructure GitHub Actions execution with fail-closed cryptographic webhook authentication.

**Independent Test**: Start the webhook listener on an ephemeral port (`127.0.0.1:0`), send signed and unsigned `issues.opened` and `issue_comment.created` requests, verify HTTP 401 on missing/invalid `X-Hub-Signature-256`, verify HTTP 202 on valid signatures, and verify that bot-authored comments (`user.type == "Bot"` or containing `<!-- replayfix-bot-comment -->`) are ignored with `ignored_bot_loop: true`.

**Acceptance Scenarios**:
1. **Given** an incoming HTTP POST to `/webhook` lacking a valid `X-Hub-Signature-256` HMAC-SHA256 signature, **When** the server inspects the request, **Then** it immediately rejects the request with `HTTP/1.1 401 Unauthorized`.
2. **Given** a valid `issue_comment.created` webhook event authored by a bot or containing ReplayFix's marker, **When** the event filter runs, **Then** the server returns `200 OK` with `"status": "ignored_bot_event"` and does not enqueue a triage job.

---

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001 (GitHub API Client & Multi-Modal Video Extraction)**: `crates/replayfix-core/src/github.rs` MUST implement `GitHubClient`, `IssueVideoAsset`, and markdown media extraction supporting `https://github.com/user-attachments/assets/...` and `.mp4`, `.webm`, `.mov` URLs, downloading into `.replayfix/scratch/issue_<number>.<ext>` with container validation (`validate_video_container`).
- **FR-002 (Ephemeral Scratch Triage & Sovereign Healer Integration)**: `IssueTriageRunner` MUST execute issue reproduction, Muse Spark 1.3 deliberation, Muse Wonder 1.0 AST repair, Gate 2 stability, Gate 3 stress quorum, and permanent inoculation (`tests/repro/rf_issue_<id>.spec.ts`) inside an isolated `ScratchSandbox` ($\Delta	ext{repo}=0$) under a `< 120s` SLA budget.
- **FR-003 (Autonomous Git PR Dispatch & Signed Commit Trailers)**: `GitPrPackager` and `GitHubClient` MUST support issue-specific branch naming (`replayfix/issue-<number>-<slug>`), commit messages containing `Fixes #<number>`, `ReplayFix-Issue: <url>`, and Ed25519-signed `ReplayFix-*` trailers, and opening Pull Requests via GitHub REST API or `gh pr create`.
- **FR-004 (Issue Thread Evidence Commenter & Triage Labels)**: ReplayFix MUST generate and post structured issue comments for both verified repairs (with keyframe bbox, side-by-side DOM `ariaSnapshot` diff, PR link, and `replayfix verify-receipt` command) and unreproduced/ambiguous outcomes, automatically applying `replayfix:verified-healed`, `replayfix:unreproduced`, or `replayfix:ambiguous`.
- **FR-005 (HMAC-SHA256 Webhook Server & Anti-Loop Filter)**: `replayfix bot --listen <addr>` MUST verify `X-Hub-Signature-256` (`sha256=<hex>`) using RFC 2104 HMAC-SHA256 in constant time, reject invalid/missing signatures with HTTP 401, ignore bot-generated events to prevent infinite loops, and process `issues.opened` and `issue_comment.created` payloads asynchronously via a bounded worker queue.
- **FR-006 (Domain Error Taxonomy & Zero-Panic Discipline)**: `CoreError` MUST define `GitHubApi(String)`, `WebhookVerification(String)`, and `IssueIngestion(String)`. All crates MUST uphold `#![forbid(unsafe_code)]`, 0 Clippy warnings (`-D warnings`), and 0 non-test panics.

---

## Success Criteria *(mandatory)*

- **SC-001 (End-to-End Issue Triage & PR Dispatch)**: Running `replayfix bot --issue https://github.com/org/repo/issues/123 --repo <path>` ingests the issue and video, executes the Sovereign Healer in an isolated scratch sandbox, synthesizes `tests/repro/rf_issue_123.spec.ts`, prepares branch `replayfix/issue-123-<slug>` with signed trailers, generates the Evidence Matrix PR and issue comment, applies `replayfix:verified-healed`, and finishes in `< 120s`.
- **SC-002 (Fail-Closed Webhook Security)**: 100% of unsigned or tampered webhook requests to `replayfix bot --listen` are rejected with HTTP 401, and 100% of bot-authored comments are filtered out without recursive re-triage.
- **SC-003 (Zero Regression & $\Delta	ext{repo}=0$)**: All 300+ existing unit and integration tests from v1.3–v1.6 plus all new v1.7 tests pass with 100% parity, and host repository invariance ($\Delta	ext{repo}=0$) holds across all runs.
- **SC-004 (Zero Panics & Clean Lints)**: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `check-panics.ps1` pass with zero warnings and zero non-test panics.
