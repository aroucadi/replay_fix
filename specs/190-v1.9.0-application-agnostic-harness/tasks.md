# Tasks: ReplayFix v1.9.0 Application-Agnostic Harness & Swarm Engine

## Phase 1 — Dynamic Task Model & Decoupling from Static Fixtures
- [ ] T001: Implement `DynamicReproTask` in `crates/replayfix-core/src/manifest.rs` and re-export in `lib.rs`.
- [ ] T002: Refactor `MockHealer::run_live_browser_in_repo` and `engine.rs` to operate on generic behavioral contracts without requiring static `Fixture`.
- [ ] T003: Remove hardcoded `#cart-summary-output, #discount-output, #inventory-output...` from `crates/replayfix-core/src/engine.rs` and `browser.rs`.
- [ ] T004: In `crates/replayfix-cli/src/main.rs`, permanently delete the `"has no built-in fixture for task"` usage error and support arbitrary repositories.

## Phase 2 — Universal Application Stack & Ephemeral Dev Server
- [ ] T005: Enhance `ManagedSpaServer` in `crates/replayfix-core/src/environment.rs` to detect Next.js, Vite, React-Scripts, Remix, Astro, and Express dev scripts from `package.json`.
- [ ] T006: Enhance `PlaywrightBridge` in `crates/replayfix-core/src/browser.rs` to capture uncaught page errors (`pageerror`), console error logs, and HTTP 4xx/5xx network failures as primary defect evidence.

## Phase 3 — Dynamic Playwright Repro Spec Synthesis
- [ ] T007: Implement dynamic Playwright repro test synthesis in `crates/replayfix-core/src/inoculate.rs` generating `tests/repro/repro_<task_id>.spec.ts`.
- [ ] T008: Integrate SHA-256 test freeze (`TestFreezeGuard`) over synthesized repro specs to prevent patch mutations.

## Phase 4 — High-Concurrency Parallel Swarm Sweep Engine
- [ ] T009: Implement `SweepCoordinator` in `crates/replayfix-core/src/sweep.rs` leveraging work-stealing queues, `EphemeralPort`, `ScratchSandbox`, and `RatePool`.
- [ ] T010: Implement `TelemetryAggregator` calculating sweep metrics (mean, p95 latency, Gate 1 Red rate, Gate 2 Green rate, token accounting).
- [ ] T011: Commit swept bug receipts to `transparency_log.json` and generate an Ed25519-signed batch Merkle root in `benchmark_summary.json`.

## Phase 5 — Terminal Fleet UI & CLI Subcommand Wiring
- [ ] T012: Implement `SweepProgressRenderer` in `crates/replayfix-core/src/sweep_ui.rs` for live progress and NDJSON streaming.
- [ ] T013: Implement `crates/replayfix-cli/src/sweep_cmd.rs` and wire `replayfix sweep` subcommand in `crates/replayfix-cli/src/main.rs`.

## Phase 6 — Benchmark Suite & Real Repository Qualification
- [ ] T014: Add unit tests in `sweep.rs`, `environment.rs`, and `manifest.rs` for dynamic task ingestion and dev server detection.
- [ ] T015: Add integration tests in `crates/replayfix-cli/tests/v19_parallel_sweep.rs` verifying concurrent execution (`--parallel 4`), $\Delta\text{repo} = 0$, and valid summary JSON.
- [ ] T016: Verify execution against the 20-bug MetaPulse suite (`benchmarks/metapulse/MANIFEST.json`).
- [ ] T017: Verify execution against the Vyra benchmark (`benchmarks/vyra/lower_body_duplicate_bug.mp4`).

## Phase 7 — Qualification, Release Packaging & Git Tagging
- [ ] T018: Execute `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `.specify/scripts/check-panics.ps1`.
- [ ] T019: Bump workspace version to `1.9.0` in `Cargo.toml` files, compile release binary, update `dashboard.html` and `proof_viewer.html`, and package `release/v1.9.0/`.
- [ ] T020: Merge `release/v1.9.0` to `main`, create annotated tag `v1.9.0`, and push to origin.
