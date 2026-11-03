# Plan: ReplayFix v1.9.0 Swarm Benchmark Engine

## Architecture Overview
The Swarm Benchmark Engine integrates the v1.8 `SwarmCoordinator` and `WorkStealingQueue` into a dedicated batch sweep architecture in `crates/replayfix-core/src/sweep.rs`.

```
[Manifest / Dataset]
         │
         ▼
[ManifestParser & Path Sanitizer]
         │
         ▼
[SweepCoordinator] ──────┐
   │ (WorkStealingQueue) │
   ├── Worker 1 ── ScratchSandbox 1 + Ephemeral Port (:0) + RatePool Client
   ├── Worker 2 ── ScratchSandbox 2 + Ephemeral Port (:0) + RatePool Client
   ├── Worker 3 ── ScratchSandbox 3 + Ephemeral Port (:0) + RatePool Client
   └── Worker N ── ScratchSandbox N + Ephemeral Port (:0) + RatePool Client
         │
         ▼
[TelemetryAggregator & Progress Stream] ──► Terminal UI (ANSI / JSON stream)
         │
         ▼
[Notary Merkle Attestation] ──────────────► Batch Merkle Root + Ed25519 Signature
         │
         ▼
[benchmark_summary.json & telemetry_matrix.json]
```

## Phases
1. Domain Error & Type Foundations (`CoreError::Benchmark`, `CoreError::Sweep`, data contracts).
2. Dataset Manifest Ingestion & Path Traversal Prevention.
3. Concurrent Sweep Coordinator with isolated ScratchSandboxes and dynamic ports.
4. Telemetry Aggregation (mean, p95, rates, token accounting).
5. Merkle Batch Attestation & Ed25519 signature.
6. Multi-Worker Progress Renderer (TUI + Headless NDJSON).
7. CLI subcommand wiring (`replayfix sweep`).
8. Unit and E2E Integration Testing Battery.
9. Qualification, Clippy & Panic Audits.
10. Release Packaging, Version Bumps, and Git Tagging.
