# Tasks: Spec 180 — Distributed Multi-Agent Swarm, Live Streaming Dashboard & Production APM Session Replay Ingestion (Release 1.8.0)

**Input**: Design documents from `/specs/180-v1.8.0-fleet-dashboard-swarm/`  
**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), JSON contracts in `contracts/` and `specs/180-v1.8.0-fleet-dashboard-swarm/contracts/`

## Phase 1: Foundational Error Taxonomy & Domain Types

- [x] T001 Add `Swarm(String)`, `Dashboard(String)`, `SessionReplay(String)`, and `Notary(String)` error variants to `CoreError` in `crates/replayfix-core/src/error.rs` and register new module exports in `crates/replayfix-core/src/lib.rs`

## Phase 2: Pillar 1 — Production APM & RRWeb Session Replay Ingestion

- [x] T002 [US1] Implement `crates/replayfix-core/src/rrweb.rs` containing `RrwebParser` (parsing DOM snapshots, incremental mutations, mouse clicks, scrolls, text inputs) and `SentryIngestion` (parsing unhandled exceptions, stack frames, breadcrumbs)
- [x] T003 [US1] Implement automated manifest synthesis in `rrweb.rs` combining interaction trajectories and stack frames into validated `TaskManifest` specs with path traversal guards (`TaskManifest::assert_safe_path`)

## Phase 3: Pillar 2 — High-Concurrency Distributed Work-Stealing Swarm

- [x] T004 [US2] Implement `crates/replayfix-core/src/swarm.rs` with `SwarmCoordinator`, thread-safe work-stealing task queue (`SwarmTask`), per-worker isolated `ScratchSandbox` ($\Delta\text{repo}=0$), dynamic ephemeral ports (`EphemeralPort`), and rate-limited `RatePool` / `CircuitBreaker` token governance
- [x] T005 [US2] Implement swarm summary aggregation and serialization into `swarm_summary.json` matching `contracts/swarm_task.schema.json`

## Phase 4: Pillar 3 — Real-Time Live Streaming Dashboard & Broadcaster

- [x] T006 [US3] Implement `crates/replayfix-core/src/dashboard.rs` with embedded async/threaded HTTP & WebSocket `DashboardServer`, RFC 6455 WebSocket streaming (`/ws/screencast/:task_id`), Server-Sent Events reasoning stream (`/events/reasoning/:task_id`), and REST status/receipt endpoints
- [x] T007 [US3] Author standalone, zero-dependency `dashboard.html` with real-time WebSocket screencast viewer, SSE deliberation feed, side-by-side DOM diff, and strict CSP

## Phase 5: Pillar 4 — Cross-Browser & Multi-Device Inoculation Matrix

- [x] T008 [US4] Extend `InoculationGenerator` in `crates/replayfix-core/src/inoculate.rs` to support multi-browser engine targets (`chromium,firefox,webkit`) and viewport presets (`mobile-portrait` 390x844 DPR 3.0, `tablet` 820x1180 DPR 2.0, `desktop` 1920x1080 DPR 1.0) with DPR calibration

## Phase 6: Pillar 5 — RFC 6962 Merkle Receipt Transparency Log & Key Notary

- [x] T009 [US5] Implement `crates/replayfix-core/src/notary.rs` with append-only `TransparencyLog` (`transparency_log.json`), RFC 6962 consistency proof calculation and verification algorithms, and Ed25519 aggregate multi-attestation

## Phase 7: CLI Subcommands & Tool Wiring

- [x] T010 [US1] Implement `replayfix ingest --sentry <url-or-file> --rrweb <file>` in `crates/replayfix-cli/src/ingest_cmd.rs`
- [x] T011 [US2] Implement `replayfix swarm --workers <N>` in `crates/replayfix-cli/src/swarm_cmd.rs`
- [x] T012 [US3] Implement `replayfix dashboard --port <port>` in `crates/replayfix-cli/src/dashboard_cmd.rs`
- [x] T013 [US5] Implement `replayfix notary` (`append`, `verify-log`, `consistency-proof`) in `crates/replayfix-cli/src/notary_cmd.rs`
- [x] T014 Wire all new subcommands (`Ingest`, `Swarm`, `Dashboard`, `Notary`) and flags (`--matrix`) into `crates/replayfix-cli/src/main.rs`

## Phase 8: Comprehensive Tests & Verification

- [x] T015 Add comprehensive unit and integration tests in `crates/replayfix-core` and `crates/replayfix-cli/tests/v18_swarm_dashboard_apm.rs` validating RRWeb ingestion, Sentry crash translation, swarm concurrency, live dashboard HTTP/WebSocket/SSE, multi-device inoculation matrix, and RFC 6962 notary consistency proofs

## Phase 9: Empirical Qualification & Convergence

- [x] T016 Run full qualification battery: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace --all-targets -- --test-threads=2`, `check-panics.ps1`
- [x] T017 Execute `/speckit-converge` confirming 100% convergence across all Functional Requirements and Success Criteria

## Phase 10: Release Packaging, Merge & Tagging

- [x] T018 Bump workspace crates to `1.8.0`, update `Cargo.lock`, `CHANGELOG.md`, `README.md`, write release notes `docs/release-notes/v1.8.0.md`
- [x] T019 Build release binaries (`cargo build --release -p replayfix-cli`), assemble `release/v1.8.0/` bundle (`replayfix.exe`, `proof_viewer.html`, `dashboard.html`, `SHA256SUMS.txt`)
- [x] T020 Commit changes to `release/v1.8.0`, merge into `main` with `--no-ff`, tag `v1.8.0`, and push to remote
