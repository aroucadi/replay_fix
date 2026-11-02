# Implementation Plan: Spec 180 — Distributed Multi-Agent Swarm, Live Streaming Dashboard & Production APM Session Replay Ingestion (Release 1.8.0)

**Branch**: `release/v1.8.0` | **Date**: 2026-11-02 | **Spec**: [spec.md](./spec.md)  
**Input**: Feature specification from `/specs/180-v1.8.0-fleet-dashboard-swarm/spec.md`

## Summary

Release 1.8.0 evolves ReplayFix from a single-target sovereign healer and GitHub bot into an enterprise-grade distributed visual repair cluster and real-time observability center. It introduces five architectural pillars:
1. **Production APM Session Replay & Log Ingestion (`crates/replayfix-core/src/rrweb.rs`)**: Ingesting RRWeb event streams (DOM mutations, clicks, scrolls, inputs) and Sentry crash dumps (unhandled exceptions, stack traces, breadcrumbs) into validated `TaskManifest` specs.
2. **Distributed Work-Stealing Triage Swarm (`crates/replayfix-core/src/swarm.rs`)**: High-throughput multi-worker coordination (`--workers <N>`) with work-stealing job queue, isolated scratch worktrees ($\Delta\text{repo}=0$), dynamic ephemeral ports (`:0`), and rate-limited token pool governance (`ModelPool`/`RatePool`), outputting `swarm_summary.json`.
3. **Real-Time Live Streaming Dashboard (`crates/replayfix-core/src/dashboard.rs`)**: Embedded zero-dependency HTTP & WebSocket server on `--port <port>` serving `dashboard.html`, streaming real-time CDP screencast frames (`/ws/screencast/:task_id`), Muse Spark reasoning tokens over SSE (`/events/reasoning/:task_id`), and REST endpoints.
4. **Cross-Browser & Multi-Device Inoculation Matrix (`crates/replayfix-core/src/inoculate.rs`)**: Synthesizing permanent Playwright test matrices across browser engines (`chromium,firefox,webkit`) and mobile viewport presets (`mobile-portrait`, `tablet`, `desktop`) with DPR calibration.
5. **RFC 6962 Merkle Receipt Transparency Log & Key Notary (`crates/replayfix-core/src/notary.rs`)**: Append-only cryptographic transparency ledger (`transparency_log.json`) chaining audit receipts with RFC 6962 consistency proofs and aggregate Ed25519 attestations.

## Technical Context

**Language/Version**: Rust 2021 (`1.75+`)  
**Primary Dependencies**: `serde`, `serde_json`, `clap`, `reqwest`, `ed25519-dalek`, `sha2`, `regex`, `thiserror`, `chrono`  
**Testing**: `cargo test --workspace --all-targets -- --test-threads=2` (all 310+ existing tests + new v1.8 tests)  
**Quality Standards**: `#![forbid(unsafe_code)]`, zero `.unwrap()`/`.expect()`/`panic!()` in non-test code, 0 Clippy warnings (`-D warnings`), `check-panics.ps1` clean.

## Constitution Check

- **Law I (Substrate Primacy & Fail-Closed Authority)**: All swarm triage verdicts and consistency checks are computed strictly by deterministic Rust logic.
- **Law II (Behavioral Contract Invariance)**: Gate 1 and Gate 2 evaluate the exact same contract verified by SHA-256 digest across all swarm workers.
- **Law III (Inviolable Base Workspace Isolation)**: Base repository immutability ($\Delta\text{repo}=0$) is maintained across all concurrent swarm workers via independent `ScratchSandbox` directories.
- **Law IV (Child-Owned Ephemeral Provenance)**: Swarm workers and dashboard listeners bind to ephemeral ports (`:0`) to avoid port collisions and resource leaks.
- **Law V (Regression Parity)**: 100% test parity is required across existing suites before certifying candidate fixes.
- **Law VI (Monotonic Event Ordering)**: Ingestion event timelines (RRWeb mutations before click, crash breadcrumbs before exception) are strictly ordered.
- **Law VII (Model Honesty & Concurrency Governance)**: Concurrent workers share a bounded `RatePool` and `CircuitBreaker` to respect rate limits without silent failures.
- **Law VIII (Untrusted Input Isolation & Security)**: Production RRWeb streams, APM breadcrumbs, and dashboard WebSocket payloads are strictly validated against directory traversal (`TaskManifest::assert_safe_path`), CORS origins, and prompt injection.
- **Law IX (Tamper-Evident Receipts & Key Transparency)**: Receipts are appended to `transparency_log.json` as RFC 6962 leaves (`0x00`) with verifiable consistency proofs and Ed25519 signatures.
- **Law X (BeautifulRust Standards)**: `#![forbid(unsafe_code)]`, zero non-test panics, explicit `CoreError` variants (`Swarm`, `Dashboard`, `SessionReplay`, `Notary`).

## Module Architecture & Data Flow

```text
[RRWeb Event Stream / Sentry Crash Dump]
       │
       ▼
[crates/replayfix-core/src/rrweb.rs]
  ├── RrwebParser::parse_events() ➔ DOM snapshots, click coordinates, input mutations
  ├── SentryIngestion::parse_crash() ➔ Unhandled exception, stack frames, breadcrumbs
  └── TaskManifest::synthesize() ➔ Validated reproduction manifest (assert_safe_path)
       │
       ▼
[crates/replayfix-core/src/swarm.rs — SwarmCoordinator]
  ├── Work-Stealing Task Queue partitions tasks across N workers
  ├── RatePool & CircuitBreaker throttle Meta Muse token consumption
  ├── Each Worker:
  │     ├── Provisions independent ScratchSandbox (Δrepo = 0)
  │     ├── Allocates dynamic EphemeralPort (:0)
  │     ├── Runs Sovereign Healer pipeline
  │     ├── Broadcasts live CDP screencast frames to Dashboard broadcaster
  │     └── Emits Muse Spark reasoning tokens to Dashboard SSE broadcaster
  └── Writes swarm_summary.json (contracts/swarm_task.schema.json)
       │
       ▼
[crates/replayfix-core/src/dashboard.rs — DashboardServer]
  ├── HTTP listener serving air-gapped dashboard.html (:3000 or ephemeral)
  ├── WebSocket /ws/screencast/:task_id ➔ Streams base64 CDP frames
  ├── SSE /events/reasoning/:task_id    ➔ Streams deliberation tokens & tool calls
  └── REST endpoints /api/status, /api/tasks, /api/receipts
       │
       ▼
[crates/replayfix-core/src/inoculate.rs — InoculationGenerator]
  └── Synthesizes Playwright matrix across chromium, firefox, webkit & mobile viewports
       │
       ▼
[crates/replayfix-core/src/notary.rs — TransparencyLog]
  ├── Appends verified audit receipts as RFC 6962 leaves (0x00)
  ├── Computes RFC 6962 consistency proofs between log snapshots (m, n)
  └── Evaluates aggregate Ed25519 attestation signatures
```
