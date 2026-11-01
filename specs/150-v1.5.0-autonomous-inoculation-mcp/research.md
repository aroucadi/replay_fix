# Research & Architectural Decisions: Spec 150 (Release 1.5.0)

**Feature**: Permanent Inoculation, Agentic Protocol (MCP) & Continuous CI/CD Automation  
**Date**: 2026-11-01

---

## Decision 1: Model Context Protocol (MCP) Transport & JSON-RPC 2.0 Engine
- **Decision**: Implement a pure-Rust JSON-RPC 2.0 server in `crates/replayfix-cli/src/mcp/` supporting both `stdio` (newline-delimited JSON-RPC 2.0 messages on stdin/stdout) and Streamable HTTP / `SSE` (via `std::net::TcpListener` on `127.0.0.1:<port>` with `/sse` event stream and `/message` or `/mcp` POST JSON-RPC endpoint) without adding external async runtime dependencies.
- **Rationale**:
  - `SandboxServer` in `crates/replayfix-core/src/browser.rs` already proves that `std::net::TcpListener` + `serde_json` is fast, deterministic, and zero-overhead.
  - External coding agents (Cursor, Claude Code, Windsurf, Antigravity) connect over `stdio` by default and optionally via HTTP/SSE.
  - Enforcing strict request payload caps (4 MiB max line/body) and path validation (`TaskManifest::assert_safe_path`) guarantees STRIDE safety.
- **Alternatives Considered**:
  - Adding `axum`/`tokio`/`tower`: Rejected to preserve the lean dependency graph and avoid mixing sync/async runtimes unnecessarily.

## Decision 2: Permanent Inoculation ("Dreaming") Synthesis & Registry
- **Decision**: Create `crates/replayfix-core/src/inoculate.rs` containing `InoculationGenerator`, `InoculationRecord`, and `InoculationRegistry` (`.replayfix/inoculations.json`).
- **Rationale**:
  - Synthesizes idiomatic `@playwright/test` TypeScript specs (`tests/repro/rf_<id>.spec.ts`) using `LocatorResolver` grounded selectors (`page.getByRole`, `page.getByText`, `page.getByTestId`) and `ActionTrajectory` steps.
  - Embeds cryptographic provenance comments (`REPLAYFIX-INOCULATION-SPEC`, `CONTRACT-SHA256`, `PATCH-SHA256`) and verifies that selectors contain zero positional anti-patterns (`.first()`, `.last()`, `.nth()`, `:nth-match`, `:nth-child`).
  - Updates `.replayfix/inoculations.json` atomically so CI pipelines and agents can audit all active regression guards.
- **Alternatives Considered**:
  - Emitting ReplayFix-proprietary JSON specs only: Rejected because Pillar 1 requires native execution via `npx playwright test` without ReplayFix runtime binaries.

## Decision 3: Concurrency Perturbation & Stress Quorum (Gate 3)
- **Decision**: Create `crates/replayfix-core/src/stress.rs` implementing `StressQuorumConfig`, `PerturbationProfile` (3G network latency, 200ms–500ms jitter, slow API fulfillment), `StressPassResult`, and `StressQuorumRunner` (perturbation pass + 3-pass randomized-interval concurrent stability sweep).
- **Rationale**:
  - Extends `PLAYWRIGHT_BRIDGE_JS` in `browser.rs` with `SET_NETWORK_PERTURBATION` (injecting artificial latency/jitter into Playwright route fulfillment) and runs a 3-pass concurrency quorum in `stress.rs`.
  - Any failure or non-deterministic divergence transitions the run to `Phase::Rejected` with `TriageStatus::FlakyNonDeterministic` (Exit Code 21).
  - Preserves 100% backward compatibility with existing 2-gate tests when Gate 3 is not triggered or when deterministic patches pass cleanly.
- **Alternatives Considered**:
  - OS-level packet filtering (`tc`/`netsh`): Rejected because it requires root/Administrator privileges and mutates host networking state.

## Decision 4: Autonomous Git Branch & PR Packaging Engine
- **Decision**: Create `crates/replayfix-cli/src/git_packager.rs` implementing `GitPrPackager`, `CommitTrailers`, and `EvidenceMatrixGenerator`.
- **Rationale**:
  - Creates isolated branches (`replayfix/fix-<bug_id>` or `replayfix/inoculate-<bug_id>`) from the clean baseline commit.
  - Signs commit metadata with the harness Ed25519 key (`KeypairManager`) and formats canonical RFC 6962 trailers (`ReplayFix-Verified`, `ReplayFix-Substrate`, `ReplayFix-Merkle-Root`, `ReplayFix-Signature`, `ReplayFix-Gate1-Status`, `ReplayFix-Gate2-Status`).
  - Emits `pr_description.md` containing the Evidence Matrix (video keyframe bounding boxes, side-by-side DOM `ariaSnapshot` diffs, gate logs, token accounting, and standalone verification command).
- **Alternatives Considered**:
  - Calling `gh pr create` directly without local branch/markdown artifacts: Rejected because offline/airgapped environments and local git workflows require self-contained branch + markdown packaging first.

## Decision 5: Multi-Modal Copilot Enhancements & CI Reporters
- **Decision**:
  - Extend `LocatorResolver` and TUI (`shell.rs` / `tui/dialog.rs`) with an ambiguity tie-breaker threshold (`AMBIGUITY_TIE_THRESHOLD_BPS = 500`, i.e., $|\Delta	ext{score}| < 0.05$) that prompts the operator interactively when in a TTY and fails closed (`AMBIGUOUS_GROUNDING`, Exit 22) in non-interactive mode.
  - Add `SubstrateEvent::CdpTelemetry` and ASR keyframe transcript rendering in `shell.rs`.
  - Implement `crates/replayfix-core/src/reporters.rs` for SARIF v2.1.0 (`results.sarif`) and JUnit XML (`junit.xml`) generation, plus root `action.yml` for GitHub Actions.
- **Rationale**: Completes Pillars 5 and 6 with zero external crate bloat.
