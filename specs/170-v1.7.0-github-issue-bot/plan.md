# Implementation Plan: Spec 170 — Autonomous GitHub Issue Ingestion Bot, Automated PR Dispatch & Continuous Triage Engine (Release 1.7.0)

**Branch**: `release/v1.7.0` | **Date**: 2026-11-02 | **Spec**: [spec.md](./spec.md)  
**Input**: Feature specification from `/specs/170-v1.7.0-github-issue-bot/spec.md`

## Summary

Release 1.7.0 evolves ReplayFix from a local CLI sovereign healer into an autonomous GitHub issue ingestion bot and continuous triage engine (`replayfix bot`). It connects directly to GitHub issues and webhook streams, extracts user-recorded bug videos (`.mp4`, `.webm`, `.mov`), executes the v1.6 Sovereign Healer inside an ephemeral scratch sandbox ($\Delta	ext{repo}=0$), dispatches signed Pull Requests (`replayfix/issue-<id>-<slug>`) with permanent Playwright inoculation specs (`tests/repro/rf_issue_<id>.spec.ts`), posts verifiable Evidence Matrix comments and triage labels (`replayfix:verified-healed`, `replayfix:unreproduced`, `replayfix:ambiguous`), and provides a fail-closed HMAC-SHA256 webhook server (`replayfix bot --listen <addr>`).

Key Architectural Deliverables:
1. **`crates/replayfix-core/src/github.rs`**:
   - `GitHubClient` & `GitHubTransport` trait (supporting live GitHub REST API v3 via `reqwest::blocking::Client` / `gh` CLI credentials `GITHUB_TOKEN` / `GH_TOKEN`, plus deterministic offline transport for hermetic tests).
   - Multi-modal Markdown media parser (`extract_video_urls_from_markdown`) extracting `https://github.com/user-attachments/assets/...` and `.mp4`/`.webm`/`.mov` URLs from issue bodies and comments.
   - Media downloader & FFmpeg/container header validator caching into `.replayfix/scratch/issue_<number>.mp4`.
   - RFC 2104 HMAC-SHA256 constant-time signature verifier (`verify_webhook_signature`) validating `X-Hub-Signature-256`.
   - Anti-loop webhook event filter (`WebhookEventPayload`, `WebhookFilterOutcome`) ignoring bot-authored comments and non-actionable events.
   - Issue comment formatter & triage label manager (`TriageLabel::VerifiedHealed`, `TriageLabel::Unreproduced`, `TriageLabel::Ambiguous`).
2. **`crates/replayfix-cli/src/git_packager.rs`**:
   - Issue-aware branch naming (`GitPrPackager::issue_branch_name(issue_number, title_slug)` $	o$ `replayfix/issue-<id>-<slug>`).
   - Extended `CommitTrailers` supporting `issue_url: Option<String>` and `Fixes #<number>` commit header alongside Ed25519-signed `ReplayFix-*` trailers.
3. **`crates/replayfix-cli/src/bot.rs` & `crates/replayfix-cli/src/main.rs`**:
   - Top-level `replayfix bot` subcommand supporting `--issue <url>`, `--listen <addr>`, `--repo <path>`, `--output <dir>`, `--webhook-secret <secret>`, `--max-requests <n>`, `--create-pr`, `--comment-issue`, `--apply-labels`, and `--json`.
   - `IssueTriageRunner` orchestrating media ingestion $	o$ `ScratchSandbox` ($\Delta	ext{repo}=0$) $	o$ v1.6 Sovereign Healer (`DeliberationEngine` + `StressQuorumRunner` + `InoculationGenerator` for `tests/repro/rf_issue_<id>.spec.ts`) $	o$ PR dispatch $	o$ Issue thread comment & label automation within a 120s SLA budget.
   - Bounded HTTP webhook daemon (`WebhookServerHandle` / `run_webhook_listener`) with HMAC-SHA256 verification (`HTTP 401` on failure) and background worker dispatch.
4. **`contracts/github_webhook.json` & `action.yml`**:
   - Canonical JSON Schema contract for GitHub webhook payloads and triage outcomes.
   - Updated composite GitHub Action (`action.yml`) supporting autonomous issue bot triage (`issue-url` input) in addition to standard verification.

## Technical Context

**Language/Version**: Rust 2021 (`1.75+`)  
**Primary Dependencies**: `serde`, `serde_json`, `clap`, `reqwest`, `ed25519-dalek`, `sha2`, `regex`, `thiserror`  
**Testing**: `cargo test --workspace --all-targets -- --test-threads=2` (all 300+ existing tests + new v1.7 unit and integration tests)  
**Constraints**: `#![forbid(unsafe_code)]`, zero `.unwrap()`/`.expect()`/`panic!()` in non-test code, 0 Clippy warnings (`-D warnings`), `check-panics.ps1` clean.

## Constitution Check

- **Law I (Substrate Primacy & Fail-Closed Authority)**: All issue triage verdicts (Gate 1 RED, Gate 2 GREEN, Gate 3 Stress Quorum) are governed strictly by the deterministic Rust harness.
- **Law II (Behavioral Contract Invariance)**: Gate 1 and Gate 2 evaluate the identical `BehavioralContract` verified by SHA-256 digest.
- **Law III (Inviolable Base Workspace Isolation)**: Host repository remains strictly unmodified ($\Delta	ext{repo}=0$) during triage; verified via `WorktreeManifest` before and after sandbox execution.
- **Law IV (Child-Owned Ephemeral Provenance)**: Sandbox dev servers and test webhook listeners bind to ephemeral ports (`:0`) with clean teardown.
- **Law V (Regression Parity)**: Existing test suite parity is enforced before certifying any issue repair.
- **Law VI (Monotonic Event Ordering)**: Gate 1 RED requires setup and target resolution prior to assertion failure.
- **Law VII (Model Honesty & Provider Configuration)**: Exact model slugs (`muse-spark-1.3`, `muse-wonder-1.0`, `sam-3.1`, `muse-voice-transcribe-1.0`) and execution modes are recorded in receipts and comments.
- **Law VIII (Untrusted Input Isolation & Security)**: Issue titles, bodies, comments, and webhook payloads are sanitized against prompt injection, ANSI escape injection, and path traversal (`TaskManifest::assert_safe_path`). Webhook payloads require valid HMAC-SHA256 signatures (`X-Hub-Signature-256`).
- **Law IX (Tamper-Evident Receipts)**: Every PR and issue comment includes RFC 6962 Merkle roots and Ed25519 signatures verifiable offline via `replayfix verify-receipt`.
- **Law X (BeautifulRust Standards)**: `#![forbid(unsafe_code)]`, zero non-test panics, explicit `CoreError` variants (`GitHubApi`, `WebhookVerification`, `IssueIngestion`).

## Module Architecture & Data Flow

```text
replayfix bot --issue <url>  OR  POST /webhook (X-Hub-Signature-256)
       │
       ▼
[crates/replayfix-core/src/github.rs]
  ├── verify_webhook_signature() ➔ Constant-time RFC 2104 HMAC-SHA256 check
  ├── filter_webhook_event()     ➔ Ignore bot comments & non-triage actions
  ├── GitHubClient::ingest_issue_bundle()
  │     ├── Fetch issue metadata + comments (REST API v3 / gh CLI / mock)
  │     ├── extract_video_urls_from_markdown() (.mp4, .webm, .mov, user-attachments)
  │     ├── download_and_validate_video() ➔ .replayfix/scratch/issue_<id>.mp4
  │     └── Sanitize untrusted narrative context
       │
       ▼
[crates/replayfix-cli/src/bot.rs — IssueTriageRunner]
  ├── Capture pre-run WorktreeManifest (Δrepo = 0 baseline)
  ├── Create isolated ScratchSandbox & allocate EphemeralPort (:0)
  ├── Execute v1.6 Sovereign Healing Loop:
  │     ├── VideoTelemetry::ingest (SAM 3.1 + Muse Voice ASR lock)
  │     ├── DeliberationEngine (Muse Spark 1.3 multi-turn + Muse Wonder 1.0 AST)
  │     ├── StressQuorumRunner (Gate 3 concurrency & jitter quorum)
  │     └── InoculationGenerator (tests/repro/rf_issue_<id>.spec.ts)
  ├── Verify post-run WorktreeManifest (Δrepo = 0 certified)
  ├── Package Branch & PR (replayfix/issue-<id>-<slug>) with signed CommitTrailers
  └── Post Issue Evidence Comment & Triage Label (replayfix:verified-healed / unreproduced / ambiguous)
```
