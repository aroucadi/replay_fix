# Tasks: ReplayFix v1.9.0 Swarm Benchmark Engine

## Phase 1 — Domain Error & Type Foundations
- [ ] T001: Add `Benchmark(String)` and `Sweep(String)` to `CoreError` in `crates/replayfix-core/src/error.rs` and re-export.
- [ ] T002: Define `SweepConfig`, `SweepTask`, `SweepResult`, `WorkerStatus`, and `TelemetryMetrics` structs in `crates/replayfix-core/src/sweep.rs` with `serde` serialization.

## Phase 2 — Dataset Manifest Ingestion & Validation
- [ ] T003: Implement `ManifestParser` in `sweep.rs` to load and validate `benchmarks/metapulse/MANIFEST.json` and external manifest formats.
- [ ] T004: Implement path traversal and resource safety checks (`assert_safe_dataset_path`) on all benchmark assets.

## Phase 3 — Concurrent Sweep Coordinator
- [ ] T005: Implement `SweepCoordinator` in `crates/replayfix-core/src/sweep.rs`, binding worker threads to work-stealing queues with isolated `ScratchSandbox` instances.
- [ ] T006: Integrate ephemeral port allocator (`:0`) ensuring zero TCP port collisions across concurrent web dev servers.
- [ ] T007: Wire centralized `RatePool` and `CircuitBreaker` into the sweep loop, enforcing adaptive backoff across parallel LLM repair requests.

## Phase 4 — Telemetry Aggregation & Schema Serialization
- [ ] T008: Implement `TelemetryAggregator` calculating sweep metrics (mean, p95 latency, token usage, success ratios).
- [ ] T009: Implement serialization to `benchmark_summary.json` conforming to `contracts/benchmark_sweep.schema.json`.

## Phase 5 — Merkle Batch Attestation
- [ ] T010: Wire `crates/replayfix-core/src/notary.rs` to commit every verified sweep receipt to `transparency_log.json` and compute aggregate batch Merkle root.
- [ ] T011: Implement Ed25519 batch signature over the aggregate sweep root.

## Phase 6 — Real-Time Multi-Worker Terminal UI
- [ ] T012: Implement `SweepProgressRenderer` for live multi-worker status display in terminal.
- [ ] T013: Implement clean, non-interactive streaming JSON output for headless CI runners.

## Phase 7 — CLI Command Wiring
- [ ] T014: Implement `crates/replayfix-cli/src/sweep_cmd.rs` with CLI flags: `--dataset`, `--manifest`, `--parallel`, `--mode`, `--json`, `--output`.
- [ ] T015: Wire `replayfix sweep` into `crates/replayfix-cli/src/main.rs`.

## Phase 8 — Comprehensive Testing Battery
- [ ] T016: Add comprehensive unit tests in `crates/replayfix-core/src/sweep.rs` covering parser, queue distribution, port allocation, and telemetry math.
- [ ] T017: Add end-to-end integration tests in `crates/replayfix-cli/tests/v19_parallel_sweep.rs` running concurrent sweeps on multi-bug test fixtures with `--parallel 4`.

## Phase 9 — Qualification & Convergence
- [ ] T018: Execute formatting, strict Clippy (`-D warnings`), panic audit (`check-panics.ps1`), and workspace tests.
- [ ] T019: Confirm 100% specification convergence and empirical validation of 20-bug sweep.

## Phase 10 — Release Packaging & Tagging
- [ ] T020: Bump version to `1.9.0` across workspace `Cargo.toml` files, compile release binary, generate `docs/release-notes/v1.9.0.md`, assemble release artifacts with SHA-256 checksums, merge to `main`, tag `v1.9.0`, and push.
