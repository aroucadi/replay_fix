# Feature Specification: Spec 180 — Distributed Multi-Agent Swarm, Live Streaming Dashboard & Production APM Session Replay Ingestion (Release 1.8.0)

**Feature Branch**: `release/v1.8.0`  
**Created**: 2026-11-02  
**Status**: Complete (Release 1.8.0)  
**Input**: Evolve ReplayFix into an enterprise-grade distributed visual repair cluster and real-time observability center. Integrate production APM session replay ingestion (RRWeb event streams and Sentry crash dumps), a high-concurrency distributed work-stealing triage swarm (`replayfix swarm`), a live streaming async HTTP/WebSocket dashboard (`replayfix dashboard`) broadcasting CDP screencasts and Muse Spark deliberation tokens, a cross-browser/multi-device Playwright inoculation matrix (`--matrix chromium,firefox,webkit,mobile`), and an RFC 6962 Merkle receipt transparency log and key notary (`replayfix notary`).

---

## User Scenarios & Testing *(mandatory)*

### User Story 1 — Production APM & RRWeb Session Replay Ingestion (Priority: P1)

As a DevOps engineer or frontend reliability lead, I want `replayfix ingest --sentry <url-or-file> --rrweb <file>` (and library API `replayfix_core::rrweb`) to ingest production session recordings (RRWeb DOM mutations, mouse clicks, scrolls, input changes) and Sentry crash dumps (unhandled exceptions, console errors, stack traces), automatically extract the failing interaction coordinates and candidate source files, and synthesize a validated `TaskManifest` guarded against path traversal attacks.

**Why this priority**: Bridges real-world end-user session failures and APM telemetry into deterministic ReplayFix reproduction specifications without manual human transcription.

**Independent Test**: Provide sample RRWeb event streams (containing incremental DOM snapshots and click events) and Sentry crash payloads (containing exception types, stack frames, and breadcrumbs), execute `rrweb::ingest_session_replay` and `rrweb::translate_sentry_breadcrumbs`, verify synthesis into a valid `TaskManifest`, and verify rejection of path traversal attempts (`../`) and malformed payloads.

**Acceptance Scenarios**:
1. **Given** an RRWeb JSON recording with incremental mouse interaction and DOM snapshot events, **When** ingested, **Then** ReplayFix extracts the interaction trajectory, calculates target viewport coordinates, and generates reproduction interaction steps.
2. **Given** a Sentry JSON event containing unhandled JavaScript exceptions and breadcrumbs, **When** parsed, **Then** stack frames are extracted, mapped to candidate source files, and combined with RRWeb coordinates into a validated `TaskManifest` passing `TaskManifest::assert_safe_path`.

---

### User Story 2 — High-Concurrency Distributed Work-Stealing Triage Swarm (Priority: P1)

As a platform engineering lead running bulk triage across defect suites or bug queues, I want `replayfix swarm --workers <N>` to coordinate `N` concurrent worker threads with work-stealing job scheduling, dynamic ephemeral port allocations (`:0`), isolated scratch worktrees ($\Delta\text{repo}=0$), and rate-limited Meta Muse model pool concurrency (`ModelPool` / `RatePool`), aggregating all outcomes into a unified `swarm_summary.json`.

**Why this priority**: Scales ReplayFix from single-task execution to multi-core, high-throughput cluster triage without port collisions, worktree contamination, or API rate limit exhaustion.

**Independent Test**: Launch `SwarmCoordinator` with 3 workers on a batch of defect tasks, verify concurrent execution on independent `ScratchSandbox` directories, verify host base repository invariance ($\Delta\text{repo}=0$), verify dynamic ephemeral port isolation, and confirm `swarm_summary.json` generation with zero deadlocks.

**Acceptance Scenarios**:
1. **Given** a batch of triage tasks and `--workers 4`, **When** `SwarmCoordinator::run_swarm` executes, **Then** workers pull tasks via work-stealing queues and execute in parallel without port or file conflicts.
2. **Given** multiple concurrent workers making LLM repair requests, **When** the shared `RatePool` limits token consumption, **Then** requests are throttled cleanly without hitting HTTP 429 errors or deadlocking.

---

### User Story 3 — Real-Time Live Streaming Replay Dashboard (Priority: P1)

As an open-source maintainer or platform operator, I want `replayfix dashboard --port <port>` to serve an air-gapped, zero-dependency async HTTP & WebSocket web dashboard (`dashboard.html`), streaming real-time browser CDP viewport screencast frames (`/ws/screencast/:task_id`), Turn-by-Turn Meta Muse Spark reasoning tokens via SSE (`/events/reasoning/:task_id`), and interactive side-by-side DOM diffs.

**Why this priority**: Delivers instant, live observability into the autonomous reproduction and healing pipeline for maintainers without requiring external SaaS dependencies.

**Independent Test**: Start `DashboardServer` on loopback `127.0.0.1:0`, connect HTTP and WebSocket clients, broadcast synthetic CDP frames and Muse Spark deliberation events, verify low-latency delivery over WebSocket and SSE, and verify `dashboard.html` rendering with strict CSP.

**Acceptance Scenarios**:
1. **Given** an active triage task emitting CDP screencast frames, **When** a browser connects to `/ws/screencast/:task_id`, **Then** base64 JPEG/WebP frames stream in real-time with sub-100ms latency.
2. **Given** Muse Spark 1.3 generating deliberation tokens, **When** listening to `/events/reasoning/:task_id`, **Then** tokens and tool execution events stream live over Server-Sent Events (SSE).

---

### User Story 4 — Cross-Browser & Multi-Device Inoculation Matrix (Priority: P1)

As a QA lead, I want `--matrix chromium,firefox,webkit,mobile` to synthesize parameterized Playwright regression tests covering desktop browsers (Chromium, Firefox, WebKit) and mobile viewports (iPhone 14 @ 390x844 DPR 3.0, Pixel 7 @ 412x915 DPR 2.625) with DPR calibration, ensuring fixes don't regress on responsive viewports.

**Why this priority**: Prevents fixes that work solely on desktop Chromium from breaking mobile layouts or alternative rendering engines.

**Independent Test**: Invoke `InoculationGenerator::synthesize_matrix_spec` with engines `chromium,firefox,webkit` and presets `mobile-portrait,tablet,desktop`, verify generated TypeScript Playwright spec matches parameterized test contracts, and verify DPR calibration math.

**Acceptance Scenarios**:
1. **Given** a verified repair, **When** `inoculate` is run with `--matrix chromium,firefox,webkit,mobile`, **Then** the generated test spec includes project configurations for all selected engines and mobile viewports.
2. **Given** mobile viewport emulation, **When** interaction coordinates are evaluated, **Then** DPR scaling and touch event emulation are calibrated accurately.

---

### User Story 5 — RFC 6962 Merkle Receipt Transparency Log & Key Notary (Priority: P1)

As a security auditor and compliance reviewer, I want `replayfix notary` to maintain an append-only cryptographic transparency ledger (`transparency_log.json`) recording all verified audit receipts as RFC 6962 leaves (`0x00`), and compute cryptographic consistency proofs (`replayfix notary verify-log`) proving historical integrity without rewriting past receipts.

**Why this priority**: Guarantees tamper-evident, verifiable proof that triage receipts are immutable and cryptographically audited across the cluster lifetime.

**Independent Test**: Append multiple audit receipts to `transparency_log.json`, generate RFC 6962 consistency proofs between intermediate tree sizes $m$ and $n$, verify proofs evaluate correctly, and confirm tampered entries fail verification closed.

**Acceptance Scenarios**:
1. **Given** an audit receipt, **When** `replayfix notary append` runs, **Then** the receipt is committed as an RFC 6962 leaf, updating the log root and Ed25519 attestation.
2. **Given** two snapshots of the transparency log at sizes $m < n$, **When** `verify-log` runs, **Then** an RFC 6962 consistency proof verifies that snapshot $m$ is an exact prefix of snapshot $n$.

---

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001 (RRWeb Event Parser & Stream Ingestion)**: `crates/replayfix-core/src/rrweb.rs` MUST implement `RrwebParser` parsing RRWeb JSON event streams, extracting DOM snapshots, mouse clicks, scrolls, and input mutations into structured interaction trajectories.
- **FR-002 (Sentry & APM Breadcrumb Translator)**: `crates/replayfix-core/src/rrweb.rs` MUST implement `SentryIngestion` extracting unhandled exception types, stack frames, console messages, and network breadcrumbs, mapping them to candidate source files.
- **FR-003 (Unified Validated Manifest Synthesis)**: ReplayFix MUST synthesize extracted visual coordinates and stack trace targets into a validated `TaskManifest`, enforcing `TaskManifest::assert_safe_path` against directory traversal (`../`), UNC paths, and DOS device names.
- **FR-004 (Work-Stealing Swarm Coordinator)**: `crates/replayfix-core/src/swarm.rs` MUST implement `SwarmCoordinator` managing `N` concurrent worker threads (`--workers <N>`) with a thread-safe work-stealing job queue.
- **FR-005 (Ephemeral Port Allocation & Worktree Immutability)**: Every swarm worker MUST execute within an isolated `ScratchSandbox`, allocate dynamic ports (`:0`) via `EphemeralPort`, and guarantee $\Delta\text{repo}=0$ on the host repository.
- **FR-006 (Bounded Model Pool & Rate Limiter)**: Swarm workers MUST share a centralized `RatePool` and `CircuitBreaker` dynamically governing Meta Muse token quotas and preventing HTTP 429 rate limit errors.
- **FR-007 (Embedded Async/Threaded Dashboard Server)**: `crates/replayfix-core/src/dashboard.rs` MUST implement `DashboardServer` binding to loopback `127.0.0.1:<port>`, serving an air-gapped SPA `dashboard.html` with strict CSP and CORS validation.
- **FR-008 (Real-Time CDP Screencast & Reasoning Streams)**: `DashboardServer` MUST provide RFC 6455 WebSocket streaming (`/ws/screencast/:task_id`) for CDP viewport frames and Server-Sent Events (`/events/reasoning/:task_id`) for Muse Spark deliberation tokens.
- **FR-009 (Multi-Engine & Responsive Inoculation Matrix)**: `crates/replayfix-core/src/inoculate.rs` MUST support `--matrix chromium,firefox,webkit,mobile` with viewport presets (`mobile-portrait`, `tablet`, `desktop`) and DPR calibration.
- **FR-010 (RFC 6962 Merkle Transparency Ledger)**: `crates/replayfix-core/src/notary.rs` MUST implement an append-only `TransparencyLog` chaining `AuditReceipt` records as RFC 6962 leaves (`0x00`) with cryptographic consistency proofs.
- **FR-011 (Ed25519 Aggregate Attestation & Key Transparency)**: `TransparencyLog` MUST support Ed25519 signing and multi-signature verification against trusted public keys.
- **FR-012 (Domain Error Taxonomy & Zero-Panic Discipline)**: `CoreError` MUST define typed variants `Swarm(String)`, `Dashboard(String)`, `SessionReplay(String)`, `Notary(String)`. Production code MUST uphold `#![forbid(unsafe_code)]`, zero Clippy warnings (`-D warnings`), and zero non-test panics (`check-panics.ps1`).

---

## Success Criteria *(mandatory)*

- **SC-001 (APM & Session Ingestion Parity)**: RRWeb event streams and Sentry crash dumps are translated into valid `TaskManifest` specs matching schema requirements in $< 500\text{ms}$.
- **SC-002 (Distributed Swarm Concurrency)**: `replayfix swarm --workers 4` executes multiple triage tasks concurrently with independent ports and worktrees, verifying $\Delta\text{repo}=0$ and generating `swarm_summary.json`.
- **SC-003 (Live Dashboard Streaming)**: `replayfix dashboard --port <port>` streams CDP screencast frames over WebSocket and Muse Spark reasoning tokens over SSE with $< 100\text{ms}$ latency.
- **SC-004 (Responsive Inoculation Matrix)**: Synthesized Playwright regression test matrices run across Chromium, Firefox, WebKit, and mobile viewport presets with accurate DPR calibration.
- **SC-005 (Cryptographic Transparency Consistency)**: `replayfix notary verify-log` verifies append-only consistency across all log entries using RFC 6962 consistency proofs.
- **SC-006 (Zero Regression & Zero Panic)**: All 310+ existing tests plus new v1.8 unit/integration tests pass with 100% parity, 0 Clippy warnings, and 0 production panics.
