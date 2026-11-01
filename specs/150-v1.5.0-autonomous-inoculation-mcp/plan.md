# Implementation Plan: Spec 150 — Permanent Inoculation, Agentic Protocol (MCP) & Continuous CI/CD Automation (Release 1.5.0)

**Branch**: `release/v1.5.0` | **Date**: 2026-11-01 | **Spec**: [spec.md](./spec.md)  
**Input**: Feature specification from `/specs/150-v1.5.0-autonomous-inoculation-mcp/spec.md`, `research.md`, and `data-model.md`.

---

## 1. Summary

Release 1.5 evolves ReplayFix into an autonomous verification, permanent inoculation ("Dreaming"), and Model Context Protocol (MCP) platform across six architectural pillars:
1. **Pillar 1 — Permanent Codebase Inoculation (`crates/replayfix-core/src/inoculate.rs`)**: Synthesizes executable Playwright/TypeScript regression specs (`tests/repro/rf_<id>.spec.ts`) using semantic accessibility locators (`getByRole`, `getByText`, `getByTestId`) and maintains the tamper-evident `.replayfix/inoculations.json` registry (`contracts/inoculation.json`).
2. **Pillar 2 — Model Context Protocol (MCP) Universal Server (`crates/replayfix-cli/src/mcp/`)**: Implements a JSON-RPC 2.0 server over `stdio` and Streamable HTTP / `SSE` (`contracts/mcp_tools.json`) exposing 5 tools (`replayfix_reproduce`, `replayfix_verify_patch`, `replayfix_inoculate`, `replayfix_inspect_trace`, `replayfix_doctor`), 3 resources (`replayfix://receipts/{task_id}`, `replayfix://sessions/{session_id}`, `replayfix://benchmarks`), and the `triage_bug` prompt.
3. **Pillar 3 — Autonomous Git Packaging & PR Engine (`crates/replayfix-cli/src/git_packager.rs`)**: Isolates repairs onto `replayfix/fix-<bug_id>` branches, signs commits with RFC 6962 + Ed25519 `ReplayFix-*` trailers, and emits the Evidence Matrix PR markdown (`pr_description.md`).
4. **Pillar 4 — Concurrency Perturbation & Stress Quorum (Gate 3) (`crates/replayfix-core/src/stress.rs`)**: Executes network latency/3G/500ms jitter perturbation and a 3-pass randomized-interval concurrent stability quorum, quarantining flaky patches with `TriageStatus::FlakyNonDeterministic` (Exit Code 21).
5. **Pillar 5 — Multi-Modal Conversational Copilot (`crates/replayfix-cli/src/shell.rs`, `crates/replayfix-core/src/locator.rs`)**: Renders ASR spoken narration aligned with video keyframes, prompts an interactive terminal locator picker on candidate score ties ($|\Delta	ext{score}| < 0.05$ / `500` bps), and streams live CDP/console/network events via `SubstrateEvent`.
6. **Pillar 6 — CI/CD Action & Headless Reporters (`crates/replayfix-core/src/reporters.rs`, `action.yml`)**: Exports SARIF v2.1.0 (`results.sarif`) and JUnit XML (`junit.xml`) reports and provides a drop-in GitHub Actions `action.yml`.

---

## 2. Technical Context

- **Language/Version**: Rust 2021 edition (1.80+)
- **Primary Dependencies**: Existing lean workspace dependencies (`serde`, `serde_json`, `thiserror`, `regex`, `chrono`, `sha2`, `ed25519-dalek`, `rand_core`, `reqwest`, `clap`, `tracing-subscriber`, `crossterm`). Zero bloated external crates added.
- **Safety & Quality Invariants**:
  - `#![forbid(unsafe_code)]` across all crates.
  - Zero `.unwrap()`, `.expect()`, `panic!()`, or `unreachable!()` in non-test production code (`.specify/scripts/check-panics.ps1`).
  - Strongly typed domain error variants in `CoreError` (`McpProtocol(String)`, `Inoculation(String)`, `StressQuorumFailed(String)`, `GitPackaging(String)`).
  - `cargo clippy --workspace --all-targets -- -D warnings` with 0 warnings.

---

## 3. Constitution Check & Invariant Gates (Laws I–X)

- **Law I (Substrate Primacy)**: Verified. MCP tool executions, Gate 3 stress sweeps, and inoculation verifications are governed exclusively by the deterministic Rust harness.
- **Law II (Contract Invariance)**: Gate 1, Gate 2, and Gate 3 evaluate identical canonical `BehavioralContract` SHA-256 digests.
- **Law III (Repository Isolation, $\Delta	ext{repo} = 0$)**: All verification and MCP `replayfix_verify_patch` runs execute in ephemeral `ScratchSandbox` directories. Base repository branches (`replayfix/fix-<bug_id>`) are created only when `--package-pr` or `--inoculate` is explicitly invoked.
- **Law IV (DOM Grounding & Fail-Closed Ambiguity)**: Synthesized inoculation specs use grounded `getByRole`/`getByText`/`getByTestId` selectors; ties within $|\Delta	ext{score}| < 0.05$ invoke the interactive picker in TTY or fail closed (`AMBIGUOUS_GROUNDING`, Exit 22) in headless mode.
- **Law V (Regression Parity)**: Synthesized inoculation tests fail on the unpatched baseline and pass on the patched state.
- **Law VI–IX (Cryptographic Merkle Receipts & Ed25519)**: Commit trailers and `audit_receipt.json` bind RFC 6962 Merkle roots and RFC 8032 Ed25519 signatures.
- **Law X (BeautifulRust Baseline)**: Enforced by compiler lints and `check-panics.ps1`.

---

## 4. Module & File Layout

```text
contracts/
├── mcp_tools.json        # [NEW] Canonical MCP tools, resources, and prompts schema
└── inoculation.json      # [NEW] Canonical .replayfix/inoculations.json schema

action.yml                # [NEW] GitHub Actions custom composite action

crates/replayfix-core/
├── src/
│   ├── error.rs          # [MOD] Add McpProtocol, Inoculation, StressQuorumFailed, GitPackaging
│   ├── inoculate.rs      # [NEW] InoculationGenerator, InoculationSpec, InoculationRegistry
│   ├── stress.rs         # [NEW] Gate 3 StressQuorumRunner, PerturbationProfile, StressQuorumReport
│   ├── reporters.rs      # [NEW] SARIF v2.1.0 (results.sarif) & JUnit XML (junit.xml) exporters
│   ├── browser.rs        # [MOD] Add SET_NETWORK_PERTURBATION command in PLAYWRIGHT_BRIDGE_JS
│   ├── locator.rs        # [MOD] Add interactive tie-breaker support (|Δscore| < 0.05 / 500 bps)
│   ├── observer.rs       # [MOD] Add CdpTelemetry event variant for console/network/CDP streaming
│   ├── receipt.rs        # [MOD] Add optional stress_quorum and inoculation_record fields
│   ├── engine.rs         # [MOD] Wire Gate 3 StressQuorumRunner and inoculation synthesis
│   └── lib.rs            # [MOD] Export inoculate, stress, and reporters modules

crates/replayfix-cli/
├── src/
│   ├── main.rs           # [MOD] Add Mcp & Inoculate subcommands; --stress-quorum, --inoculate, --package-pr, --sarif, --junit flags
│   ├── git_packager.rs   # [NEW] Isolated branch creation, Ed25519 commit trailers, Evidence Matrix PR markdown
│   ├── mcp/              # [NEW] MCP JSON-RPC 2.0 server over stdio and HTTP/SSE
│   │   ├── mod.rs        # Server router, initialize, tools/list, tools/call, resources, prompts
│   │   ├── stdio.rs      # Bounded line-delimited JSON-RPC 2.0 stdio transport
│   │   └── sse.rs        # Bounded TcpListener HTTP/SSE transport (/sse, /message, /mcp)
│   ├── shell.rs          # [MOD] Add ASR keyframe alignment view, ambiguity picker integration, CDP log stream
│   └── intent.rs         # [MOD] Add /inoculate, /mcp, /trace conversational intents
└── tests/
    ├── cli.rs            # [EXISTING] Preserved 100%
    ├── agent_ux.rs       # [EXISTING] Preserved 100%
    └── v15_release.rs    # [NEW] Comprehensive integration & adversarial tests for Pillars 1–6
```
