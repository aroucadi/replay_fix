# Tasks: Spec 170 — Autonomous GitHub Issue Ingestion Bot, Automated PR Dispatch & Continuous Triage Engine (Release 1.7.0)

**Input**: Design documents from `/specs/170-v1.7.0-github-issue-bot/`  
**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [contracts/github_webhook.schema.json](./contracts/github_webhook.schema.json), `contracts/github_webhook.json`

## Phase 1: Foundational Error Taxonomy & GitHub Domain Types

- [x] T001 Add `GitHubApi(String)`, `WebhookVerification(String)`, and `IssueIngestion(String)` error variants to `CoreError` in `crates/replayfix-core/src/error.rs` and export the new `github` module in `crates/replayfix-core/src/lib.rs`

## Phase 2: User Story 1 & 5 — Multi-Modal GitHub Issue Ingestion & HMAC-SHA256 Webhook Security

- [x] T002 [US1] Implement `GitHubClient`, `GitHubTransport`, `extract_video_urls_from_markdown`, container header validation (`validate_video_container`), and `IssueTriageBundle` ingestion in `crates/replayfix-core/src/github.rs`
- [x] T003 [US5] Implement RFC 2104 constant-time HMAC-SHA256 webhook signature verifier (`verify_webhook_signature` for `X-Hub-Signature-256`) and anti-loop event filter (`filter_webhook_event`) in `crates/replayfix-core/src/github.rs`
- [x] T004 [US4] Implement `TriageLabel` (`replayfix:verified-healed`, `replayfix:unreproduced`, `replayfix:ambiguous`), `IssueCommenter` (structured Evidence Matrix comment + side-by-side DOM `ariaSnapshot` diff + diagnostic failure comment), and PR creation types in `crates/replayfix-core/src/github.rs`

## Phase 3: User Story 2 & 3 — Autonomous PR Dispatch & Issue-Specific Inoculation

- [x] T005 [US2/US3] Extend `InoculationGenerator` in `crates/replayfix-core/src/inoculate.rs` and `GitPrPackager` / `CommitTrailers` in `crates/replayfix-cli/src/git_packager.rs` to support issue-numbered specs (`tests/repro/rf_issue_<id>.spec.ts`), branches (`replayfix/issue-<id>-<slug>`), `Fixes #<id>` headers, and `ReplayFix-Issue: <url>` trailers

## Phase 4: User Story 2, 3, 4 & 5 — `replayfix bot` CLI Subcommand, Triage Runner & Webhook Server

- [x] T006 [US2/US3/US4/US5] Implement `crates/replayfix-cli/src/bot.rs` (`BotArgs`, `IssueTriageRunner`, `WebhookServerHandle`, `run_webhook_listener`, and `run_bot`) and wire `Commands::Bot` into `crates/replayfix-cli/src/main.rs`
- [x] T007 [US5] Update `action.yml` to support autonomous GitHub issue triage mode (`issue-url` input) alongside standard verification

## Phase 5: Empirical Verification, Convergence & Release Packaging

- [x] T008 Add comprehensive unit and integration tests in `crates/replayfix-core/src/github.rs` and `crates/replayfix-cli/tests/v17_github_bot.rs` covering markdown video extraction, HMAC-SHA256 webhook security (HTTP 401 vs 202), anti-loop bot filtering, end-to-end `replayfix bot --issue <url>` triage, PR packaging, issue commenting, and label assignment
- [x] T009 Run full qualification battery (`cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace --all-targets -- --test-threads=2`, `check-panics.ps1`) and verify `/speckit-converge`
- [x] T010 Bump workspace version to `1.7.0`, update `.gitignore`, `CHANGELOG.md`, `README.md`, `docs/release-notes/v1.7.0.md`, assemble `release/v1.7.0/` (`replayfix.exe`, `proof_viewer.html`, `SHA256SUMS.txt`), merge `release/v1.7.0` to `main`, tag `v1.7.0`, and push

## Phase 6: Convergence

- [x] T011 [FR-001/FR-004/FR-007/FR-008] Harden `GitHubClient`, `IssueCommenter`, and `BotArgs` with FFmpeg container geometry probing (`VideoPreprocessor::probe_video`), visual keyframe preview + `ariaSnapshot` diffs + `audit_receipt.json` links in issue comments, `validate_markdown_file_paths` via `TaskManifest::assert_safe_path`, and secret redaction (`redact_github_secrets`, custom `Debug` on `GitHubClient` and `BotArgs`)
- [x] T012 [SC-001..SC-005] Add adversarial CSO & FFmpeg container geometry integration tests in `crates/replayfix-cli/tests/v17_github_bot.rs` and refresh `release/v1.7.0/` binary and `SHA256SUMS.txt`
