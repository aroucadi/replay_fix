# Implementation Plan: 040-v0.4-playwright-cdp-grounding — Playwright CDP Grounding & Live Browser Verification

**Branch**: `release/v0.4.0-playwright-cdp` | **Date**: 2026-10-27 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/040-v0.4-playwright-cdp-grounding/spec.md`

---

## Summary

Add two focused modules to `crates/replayfix-core`:
1. `crates/replayfix-core/src/locator.rs`: Implements deterministic spatial-semantic locator grounding (`BoundingBox`, `DomCandidate`, `GroundedTarget`, `GroundingTelemetry`, `LocatorResolver`) with integer basis-point scoring (`0..=10_000` bps), the **DOM Grounding Invariant** (every candidate must exist in the live DOM/accessibility snapshot and produce a resilient selector free of `.first()` / `:nth-match`), and the **Fail-Closed Ambiguity Guard** (`TriageStatus::AmbiguousGrounding` / `AMBIGUOUS_GROUNDING` exit code `22` whenever no candidate matches or the top two candidates differ by $|\Delta S| < 0.25$, i.e. `< 2500` bps).
2. `crates/replayfix-core/src/browser.rs`: Implements an embedded localhost HTTP fixture server (`SandboxServer`) bound to `127.0.0.1:<ephemeral_port>` serving the isolated `ScratchSandbox`, plus a line-delimited JSON-lines stdio IPC client (`PlaywrightBridge`) that invokes Node.js with an embedded bridge script (`playwright_bridge.js`) to launch Chromium (headless or `--headed`), run `document.elementFromPoint(x, y)` + `.closest(...)` reverse probing, capture `ariaSnapshot`, click the grounded selector, and observe runtime exceptions (`pageerror`) and post-action DOM text.

Wire `MockHealer` / `LiveBrowserHealer` and `replayfix repro --live-browser` (with optional `--headed`) to execute Gate 1 (RED), disk patch inside `ScratchSandbox`, and Gate 2 (GREEN) against the live Chromium browser, verify $\Delta \text{repo} = 0$, and write `audit_receipt.json` (schema `"0.4"`, `substrate: "playwright-cdp"`, `grounding_telemetry`) or `triage_report.json`.

---

## Technical Context

**Language/Version**: Rust 1.99.1 (Edition 2021) + Node.js 24 / Playwright 1.64+
**New Dependencies**: None. Uses `std::net::TcpListener`, `std::io::{BufRead, BufReader, Write}`, `std::process::{Command, Stdio}`, `std::thread`, and existing `serde` / `serde_json` / `sha2`.
**Storage**: Ephemeral scratch directories in `std::env::temp_dir()`; JSON receipts in `--output-dir` (default `.replayfix/`).
**Testing**: `cargo test --workspace` — unit tests in `locator.rs`, `browser.rs`, `engine.rs`, `receipt.rs` and CLI integration tests in `crates/replayfix-cli/tests/cli.rs`.
**Target Platform**: Windows (MSVC), macOS, Linux.
**Constraints**: Zero external Rust crate additions; integer basis-point scores so all receipt structs remain `Eq` and compatible with RFC 8785 `canonical_json`; fail-closed on ambiguous or ungrounded coordinates; clean teardown of HTTP server and browser processes.

---

## Constitution Check

| Constitutional Invariant | Status | Alignment & Evidence |
| :--- | :---: | :--- |
| **Law I: Substrate Primacy** | **PASS** | `LocatorResolver` and `PhaseMachine` in Rust hold sole authority over grounding acceptance and gate verdicts. |
| **Law II: Contract Invariance** | **PASS** | Both Gate 1 and Gate 2 evaluate the identical `BehavioralContract` (verified via canonical SHA-256 digest) and the grounded target selector. |
| **Law III: Workspace Isolation ($\Delta \text{repo} = 0$)** | **PASS** | All HTML/JS serving, bridge script materialization, and patch writes happen strictly inside `ScratchSandbox` in `std::env::temp_dir()`; pre/post `WorktreeManifest` verifies $\Delta \text{repo} = 0$. |
| **Law IV: Ephemeral Provenance** | **PASS** | `SandboxServer` binds directly to the OS-assigned `127.0.0.1:<ephemeral_port>` and serves the scratch sandbox. |
| **Law VI: Monotonic Ordering** | **PASS** | Live browser gate events (`SETUP -> TARGET -> ACTION -> ASSERT`) are validated in strict sequence; navigation or bridge failures are classified as `Invalid` (`BRIDGE_EXECUTION_FAILED`), never mistaken for a causal `RED` defect. |
| **Law VII: Model Honesty** | **PASS** | Receipt records `substrate: "playwright-cdp"` for live browser runs and `substrate: "mock-static-oracle"` for `--mock-heal` simulated runs. |
| **Law X: BeautifulRust & Vertical Micro-Slicing** | **PASS** | `unsafe_code = "forbid"`, zero `.unwrap()` / `.expect()` in production source, `Debug` derived on all public types, zero new external crates. |

---

## Project Structure

```text
specs/040-v0.4-playwright-cdp-grounding/
├── spec.md  plan.md  research.md  data-model.md  quickstart.md  tasks.md
└── checklists/requirements.md

crates/replayfix-core/src/
├── lib.rs        # export locator and browser modules
├── error.rs      # + Browser / Grounding error variants
├── locator.rs    # NEW: BoundingBox, DomCandidate, GroundedTarget, GroundingTelemetry, LocatorResolver
├── browser.rs    # NEW: SandboxServer, PlaywrightBridge, BridgeRequest/Response, embedded playwright_bridge.js
├── fixtures.rs   # + Click hotspot coordinates (x, y) and HTML shell for RF-01 live browser fixture
├── engine.rs     # + run_live_browser_in_repo alongside run_in_repo; grounding telemetry propagation
└── receipt.rs    # Schema "0.4", + AmbiguousGrounding (exit 22), + BridgeExecutionFailed (exit 20), + grounding_telemetry

crates/replayfix-cli/
├── src/main.rs   # + --live-browser flag (and --mock-candidates for deterministic grounding tests), UI step rendering
└── tests/cli.rs  # Integration tests for live browser grounding, ambiguity abstention (exit 22), and backward compatibility
```

---

## Complexity Tracking

Zero new Rust crate dependencies added. `SandboxServer` uses `std::net::TcpListener` on the ephemeral port allocated for the run, and `PlaywrightBridge` uses line-delimited JSON over `std::process::Command` stdio pipes with explicit newline flushing.
