# Plan: ReplayFix v1.9.0 Application-Agnostic Harness & Swarm Engine

## System Architecture

```
[Input: Video / Telemetry]       [Input: Arbitrary Web Repo]
           │                                 │
           ▼                                 ▼
[Perception & Hotspot Detector]    [Universal Dev Server (ManagedSpaServer)]
- Decodes video via FFmpeg         - Inspects package.json scripts (dev/start)
- SAM 3.1 visual centroid snap     - Binds dynamic ephemeral loopback port (:0)
- Optical flow trajectory          - Polls HTTP readiness up to 30s
           │                                 │
           └────────────────┬────────────────┘
                            ▼
           [Playwright CDP Autonomous Grounding]
           - Navigates to ephemeral port URL
           - Evaluates element at calibrated coordinates
           - Captures ariaSnapshot accessibility tree
                            │
                            ▼
           [Generic Defect Detection (Gate 1 RED)]
           - Listens to page.on('pageerror') (uncaught TypeErrors, exceptions)
           - Listens to page.on('response') (HTTP 4xx/5xx API failures)
           - Measures DOM mutation stalls
           - Synthesizes frozen tests/repro/repro_<task_id>.spec.ts
                            │
                            ▼
           [Autonomous Bounded Repair (Meta Muse / LLM)]
           - Feeds Gate 1 diagnostic + target component source code
           - Generates minimal AST search/replace unified diff
           - Applies patch inside isolated ScratchSandbox (Δrepo = 0)
           - Preflight node --check / lint
                            │
                            ▼
           [Gate 2 Verification & Stability]
           - Executes synthesized Playwright test (2 consecutive runs)
           - Checks regression parity against full repository test suite
                            │
                            ▼
           [Parallel Swarm Sweep & Attestation]
           - WorkStealingQueue orchestrates N concurrent workers
           - Notary commits leaves to transparency_log.json
           - Emits benchmark_summary.json + telemetry_matrix.json
```

## Phases
1. **Dynamic Task Model & Elimination of Fixture Lock-in**: Implement `DynamicReproTask` in `manifest.rs`, decouple `engine.rs` from static `Fixture`, remove hardcoded output selectors.
2. **Universal Dev Server & Failure Detection**: Harden `ManagedSpaServer` for Next.js, Vite, React-Scripts, Express; generic `pageerror` and network status defect detection in `browser.rs`.
3. **Dynamic Playwright Repro Spec Generator**: Implement automated synthesis of standalone, SHA-256 frozen Playwright reproduction specs in `inoculate.rs` and `engine.rs`.
4. **Parallel Swarm Sweep Coordinator & Telemetry Aggregator**: Complete `SweepCoordinator` in `sweep.rs` with `RatePool` and `CircuitBreaker`.
5. **Real-Time Terminal UI & CLI Subcommand Wiring**: Implement `SweepProgressRenderer` and wire `replayfix sweep` subcommand in `sweep_cmd.rs` and `main.rs`.
6. **Empirical Validation**: Run sweep across the 20-bug MetaPulse suite (`benchmarks/metapulse/MANIFEST.json`), Vyra, and open-source benchmarks.
7. **Qualification & Release Packaging**: Format, Clippy, panic audit, tests, version bumps, release notes, and Git release.
