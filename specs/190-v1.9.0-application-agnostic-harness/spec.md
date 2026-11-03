# Spec: ReplayFix v1.9.0 Pure Application-Agnostic Harness & Universal Repro Engine

## 1. Overview & Non-Negotiable Invariant
By the conclusion of v1.9.0, the error `"has no built-in fixture for task"` must be **PERMANENTLY EXTINCT**.
ReplayFix accepts two fundamental inputs:
1. Video recording (or APM session replay / user interaction coordinates).
2. Access to any web application repository (React, Next.js, Vite, SPA, Express, Remix).

ReplayFix automatically boots the application dev server on an ephemeral loopback port (`:0`), grounds user interaction via Playwright CDP, establishes causal baseline Gate 1 RED based on observable browser runtime failures (uncaught exceptions, network 5xx, or DOM stalls), synthesizes a frozen Playwright repro spec, generates and validates a surgical patch ($\Delta\text{repo} = 0$), verifies Gate 2 GREEN stability, and produces an immutable audit receipt.

## 2. Functional Requirements (FR-001 – FR-012)
- **FR-001 (Dynamic Task Manifest Ingestion)**: Introduce `DynamicReproTask` in `crates/replayfix-core/src/manifest.rs` allowing execution of any arbitrary task ID, repository path, video path, or interaction specification without requiring pre-registered entries in `fixtures.rs`.
- **FR-002 (Fixture Decoupling & Demotion)**: Demote `Fixture::for_task()` strictly to internal offline regression testing. The primary CLI entry point in `main.rs` and `MockHealer` MUST never fail due to an unregistered task ID.
- **FR-003 (Universal Application Stack Detection & Booting)**: `ManagedSpaServer` in `environment.rs` inspects `package.json` scripts (`dev`, `start`, `serve`), detects Vite, Next.js, React-Scripts, Astro, Remix, or Node/Express, binds dynamically to port `:0`, and polls HTTP health with exponential backoff before CDP connection.
- **FR-004 (Universal Defect Detection & Elimination of Hardcoded Selectors)**: Completely remove hardcoded selector lists (`#cart-summary-output, #discount-output, #inventory-output...`). Causally establish Gate 1 RED via generic observable failure signals:
  - Uncaught page exceptions (`page.on('pageerror')` capturing TypeError, ReferenceError, promise rejection).
  - Failed network responses (HTTP status >= 400).
  - DOM mutation stalls (interaction completed but expected DOM update failed to occur within deadline).
  - Optional user-specified assertion selectors when provided.
- **FR-005 (Dynamic Playwright Repro Spec Synthesis)**: Dynamically synthesize standalone `@playwright/test` specs (`tests/repro/repro_<task_id>.spec.ts`) capturing setup, calibrated geometric action replay, and negative error assertion, protected by SHA-256 test freeze (`TestFreezeGuard`).
- **FR-006 (Bounded Autonomous Repair for Arbitrary Repositories)**: In live AI mode (`Meta Muse Spark 1.3 / Wonder 1.0` or configured gateway), synthesize AST search/replace block patches directly against the identified target source file using runtime diagnostic context. In mock mode on unregistered tasks, gracefully report requiring live credentials rather than panicking with a missing fixture error.
- **FR-007 (High-Concurrency Parallel Swarm Sweep Engine)**: Expose `replayfix sweep` CLI subcommand supporting `--dataset <path-or-manifest>`, `--parallel <N>`, `--mode <deterministic-fixture|live-ai>`, `--json`, and `--output <path>`.
- **FR-008 (Real-Time Terminal Fleet Progress Renderer & TUI)**: Sub-second non-flickering ANSI progress dashboard displaying worker ID, active bug, stage, elapsed time, and status, with clean streaming NDJSON for CI/CD runners.
- **FR-009 (Comprehensive Telemetry Aggregation & Pass-Rate Matrix)**: Serialize run outcomes into `benchmark_summary.json` matching `contracts/benchmark_sweep.schema.json` and `telemetry_matrix.json` matching `contracts/telemetry_matrix.schema.json` (calculating mean latency, p95 latency, Red/Green rates, token consumption).
- **FR-010 (Cryptographic Batch Merkle Root Attestation)**: Aggregate RFC 6962 leaves for every swept bug into an Ed25519-signed batch Merkle root committed to `transparency_log.json`.
- **FR-011 (Real-World Benchmark Qualification)**: Verify reproduction and sweep across:
  1. Real MetaPulse repository on disk (`D:\rouca\DVM\workPlace\MetaPulse`) across branches (`bug/RF-01` .. `bug/RF-20`).
  2. Vyra fitness application (`benchmarks/vyra/lower_body_duplicate_bug.mp4`).
  3. Open-source component benchmark (`react-date-picker` / `re-resizable`).
- **FR-012 (Zero Regression & Substrate Primacy)**: Maintain 100% passing parity across all 350+ existing unit and integration tests while forbidding unsafe code and ensuring zero production panics.

## 3. Success Criteria (SC-001 – SC-006)
- **SC-001**: Running `replayfix repro --task-id ARBITRARY-99 --repo ./any-repo` never outputs `"has no built-in fixture for task"`.
- **SC-002**: 20-bug sweep on real MetaPulse completes in $< 90\text{ seconds}$ in deterministic fixture mode.
- **SC-003**: Strictly $\Delta\text{repo} = 0$ across all concurrent workers.
- **SC-004**: Zero Clippy warnings (`-D warnings`) and zero non-test panics (`check-panics.ps1`).
- **SC-005**: Authentic `benchmark_summary.json` and `telemetry_matrix.json` generated matching draft-07 schemas.
- **SC-006**: Complete release bundle in `release/v1.9.0/` with verified `SHA256SUMS.txt`.
