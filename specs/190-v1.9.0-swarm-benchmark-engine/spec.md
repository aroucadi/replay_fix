# Spec: ReplayFix v1.9.0 High-Throughput Parallel Benchmark Swarm Engine

## 1. Overview
ReplayFix v1.9.0 provides a high-throughput, parallel benchmark swarm engine capable of autonomously executing triage, reproduction, repair, and verification sweeps across the 20-bug MetaPulse benchmark suite (`benchmarks/metapulse/MANIFEST.json`) or custom test suites.

## 2. Functional Requirements (FR-001 – FR-012)
- **FR-001 (CLI Sweep Interface)**: CLI command `replayfix sweep` accepting `--dataset <path>`, `--manifest <path>`, `--parallel <N>`, `--mode <live-ai|deterministic-fixture>`, `--json`, and `--output <path>`.
- **FR-002 (Dataset Manifest Validation)**: `ManifestParser` parses and validates suite manifests, checking bug identifiers, git branches, MP4 media existence, and SHA-256 integrity.
- **FR-003 (Dataset Path Sanitization)**: `assert_safe_dataset_path` enforces boundary isolation, rejecting path traversal attempts, symlink escapes, or access to sensitive filesystem locations.
- **FR-004 (Work-Stealing Concurrency)**: `SweepCoordinator` dynamically orchestrates N parallel worker tasks using lockless/work-stealing queues, minimizing thread starvation and worker idle time.
- **FR-005 (Ephemeral Port Allocation)**: Each worker allocates unique dynamic loopback ports (`:0`) for ephemeral local web servers, guaranteeing zero TCP port collisions across concurrent runs.
- **FR-006 (Strict Scratch Isolation)**: Each triage job executes within an ephemeral `ScratchSandbox`, ensuring the base repository remains completely pristine ($\Delta\text{repo} = 0$).
- **FR-007 (RatePool & Circuit Breaker)**: Centralized token rate pool and circuit breaker regulate parallel LLM repair requests under `--mode live-ai` to avoid HTTP 429 rate limit errors and cascade failures.
- **FR-008 (Real-Time Terminal Telemetry)**: Interactive terminal dashboard renders live multi-worker progress (Worker ID, active bug, stage, elapsed time, status) with ANSI flicker-free refresh.
- **FR-009 (Headless CI JSON Stream)**: When `--json` or `CI=true` is set, suppress ANSI codes and stream structured NDJSON progress events for clean automated pipelines.
- **FR-010 (Telemetry Aggregation & Pass-Rate Matrix)**: `TelemetryAggregator` calculates empirical metrics including mean latency, p95 latency, Gate 1 Red rate, Gate 2 Green rate, and token consumption.
- **FR-011 (Merkle Batch Attestation)**: Every executed bug receipt RFC 6962 leaf is committed to the transparency log, and an aggregate batch Merkle root is calculated and signed with Ed25519.
- **FR-012 (Schema-Compliant Summary)**: The run generates `benchmark_summary.json` matching `contracts/benchmark_sweep.schema.json` and `telemetry_matrix.json` matching `contracts/telemetry_matrix.schema.json`.

## 3. Success Criteria (SC-001 – SC-006)
- **SC-001 (Execution Speed)**: Complete the 20-bug MetaPulse sweep in $< 90\text{ seconds}$ in deterministic fixture mode with `--parallel 4`.
- **SC-002 (Repository Integrity)**: Strictly $\Delta\text{repo} = 0$ throughout concurrent execution across all worker threads.
- **SC-003 (Cryptographic Proof)**: Emit authentic `benchmark_summary.json` with valid RFC 6962 batch Merkle root and Ed25519 signature.
- **SC-004 (Zero Panics & Zero Clippy Warnings)**: Zero `.unwrap()`, `.expect()`, or panics in production code; passes `cargo clippy --workspace --all-targets -- -D warnings`.
- **SC-005 (Regression Parity)**: 100% pass rate across all existing test suites (v1.3 through v1.8) and new v1.9 integration tests.
- **SC-006 (Zero Resource Leak)**: All temporary worktrees, sockets, child processes, and ephemeral directories are cleanly recycled upon completion.
