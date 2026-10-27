# Tasks: 040-v0.4-playwright-cdp-grounding — Playwright CDP Grounding & Live Browser Verification

**Spec**: [spec.md](spec.md) | **Plan**: [plan.md](plan.md)
**Rule**: The workspace must compile and all tests must pass after every task.

---

## Phase 1: Setup & Error Taxonomy

- [x] T001 Bump workspace crates to `0.4.0` in `crates/replayfix-core/Cargo.toml` and `crates/replayfix-cli/Cargo.toml`
- [x] T002 Add `CoreError::Browser(String)` and `CoreError::Grounding(String)` in `crates/replayfix-core/src/error.rs` and declare `pub mod locator` and `pub mod browser` in `crates/replayfix-core/src/lib.rs`

---

## Phase 2: User Story 1 — Deterministic Spatial-Semantic Locator Grounding & Ambiguity Guard (Priority: P1) 🎯 MVP

- [x] T003 [P] [US1] Implement `BoundingBox`, `DomCandidate`, `GroundedTarget`, `GroundingTelemetry`, and `LocatorResolver` (spatial-semantic basis-point scoring `0..=10_000` bps, DOM Grounding Invariant against `ariaSnapshot`, `.first()` / `:nth-match` ban, and Fail-Closed Ambiguity Guard when $|\Delta S| < 2_500$ bps) in `crates/replayfix-core/src/locator.rs`
- [x] T004 [US1] Add unit tests in `crates/replayfix-core/src/locator.rs` covering clear single-candidate resolution, ambiguity tie rejection ($|\Delta S| < 0.25$), ungrounded `ariaSnapshot` rejection, out-of-viewport coordinate rejection, and `.first()` selector ban

---

## Phase 3: User Story 3 — Line-Delimited JSON-Lines IPC & Ephemeral Server Lifecycle (Priority: P3)

- [x] T005 [P] [US3] Extend `Fixture` in `crates/replayfix-core/src/fixtures.rs` with click hotspot coordinates `(click_x, click_y)`, viewport dimensions, and `index_html` fixture page for `RF-01`
- [x] T006 [US3] Implement `SandboxServer` (localhost HTTP server on `127.0.0.1:0` serving `ScratchSandbox` files with clean shutdown) and `PlaywrightBridge` (embedded `playwright_bridge.js` with line-delimited JSON-lines stdio IPC, explicit newline flushing, `elementFromPoint` + `.closest()` reverse probing, `ariaSnapshot` capture, and `pageerror` collection) in `crates/replayfix-core/src/browser.rs`
- [x] T007 [US3] Add unit tests in `crates/replayfix-core/src/browser.rs` verifying `SandboxServer` serves files on an ephemeral port and shuts down cleanly, JSON-lines protocol serialization, and live Chromium reverse-probing + click execution

---

## Phase 4: User Story 2 — Live Playwright Browser Two-Gate Verification & Receipt v0.4 (Priority: P2)

- [x] T008 [P] [US2] Update `SCHEMA_VERSION` to `"0.4"`, add `TriageStatus::AmbiguousGrounding` (`AMBIGUOUS_GROUNDING`, exit code `22`) and `TriageStatus::BridgeExecutionFailed` (`BRIDGE_EXECUTION_FAILED`, exit code `20`), and add `grounding_telemetry: Option<GroundingTelemetry>` to `AuditReceipt` in `crates/replayfix-core/src/receipt.rs`
- [x] T009 [US2] Implement `MockHealer::run_live_browser_in_repo` (and candidate override hook for deterministic ambiguity tests) in `crates/replayfix-core/src/engine.rs`, driving Gate 1 (RED in Chromium) -> disk patch in `ScratchSandbox` -> Gate 2 (GREEN in Chromium), verifying $\Delta \text{repo} = 0$, and recording `substrate: "playwright-cdp"` and `grounding_telemetry`
- [x] T010 [US2] Add engine unit tests in `crates/replayfix-core/src/engine.rs` verifying live browser two-gate execution, ambiguity abstention (`AMBIGUOUS_GROUNDING`, exit code `22`), and ineffective patch rejection in browser mode
- [x] T011 [US2] Update `crates/replayfix-cli/src/main.rs` to add `--live-browser` (and `--mock-ambiguous` for CLI ambiguity testing), honor `--headed`, display CDP grounding telemetry in the terminal UI, and preserve `--mock-heal` backward compatibility
- [x] T012 [US2] Add CLI integration tests in `crates/replayfix-cli/tests/cli.rs` covering `--live-browser` end-to-end verification, `AMBIGUOUS_GROUNDING` exit code `22` with `triage_report.json`, and backward-compatible `--mock-heal`

---

## Phase 5: Polish, Quality Gates & Shipping

- [x] T013 Run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, and `cargo test --workspace`
- [x] T014 Execute mandatory review (`gstack-review`), QA (`gstack-qa`), and security (`gstack-cso`) audits; address any findings
- [x] T015 Update `README.md`, `CHANGELOG.md`, `docs/release-notes/v0.4.0.md`, `docs/agent-state.md`, and mark completed tasks in `specs/040-v0.4-playwright-cdp-grounding/tasks.md`
- [x] T016 Execute `gstack-ship`: commit, merge `release/v0.4.0-playwright-cdp` to `main`, tag `v0.4.0`, push to `origin`, and verify remote state
