<div align="center">

# 🎬 ReplayFix v3.0.0

### Universal Zero-Shot Video-to-Code Repair Harness & Cryptographic Verification Notary

[![Version](https://img.shields.io/badge/Release-v3.0.0%20Universal%20Zero--Shot-00E5FF)](docs/release-notes/v3.0.0.md)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE.md)
[![Rust](https://img.shields.io/badge/Rust-1.85%2B%20(%23!%5Bforbid(unsafe_code)%5D)-orange?logo=rust)](https://www.rust-lang.org)
[![Meta AI Models](https://img.shields.io/badge/Meta%20AI-Muse%20Spark%201.3%20%7C%20Muse%20Wonder%201.0%20%7C%20SAM%203.1-0081FB?logo=meta)](https://ai.meta.com)
[![Verification](https://img.shields.io/badge/Oracle-Two--Gate%20Live%20Playwright%20CDP-success)](#-the-two-gate-causal-verification-oracle)
[![Security Invariant](https://img.shields.io/badge/%CE%94repo%20%3D%200-Strictly%20Isolated-blueviolet)](#-the-inviolable-engineering-invariants)
[![Merkle Proof](https://img.shields.io/badge/RFC%206962-Ed25519%20Signed-teal)](#-cryptographic-notary--merkle-audit-receipts)

<p align="center">
  <b>Point ReplayFix at any bug screen recording (<code>.mp4</code>) and any target repository.<br/>
  It reproduces the bug in a live browser (Gate 1 RED), synthesizes a surgical AST patch via Meta Muse, proves the fix across consecutive browser runs (Gate 2 GREEN), and seals an Ed25519-signed RFC 6962 Merkle receipt — with zero base-repo mutations (<code>Δrepo = 0</code>).</b>
</p>

<p align="center">
  <a href="https://www.youtube.com/watch?v=cPDIFQmVn9I"><b>🎥 Watch 4K Demo Video</b></a> •
  <a href="#-60-second-jury-evaluation-guide"><b>⚡ 60-Second Jury Guide</b></a> •
  <a href="#-the-two-gate-causal-verification-oracle"><b>🔬 Two-Gate Oracle</b></a> •
  <a href="#-universal-zero-shot-architecture-v300"><b>🧠 Zero-Shot Architecture</b></a> •
  <a href="docs/README.md"><b>📚 Full Documentation Hub</b></a> •
  <a href="proof_viewer.html"><b>🛡️ Judge Proof Portal</b></a>
</p>

</div>

---

## ⚡ 60-Second Jury Evaluation Guide

Judges can verify **ReplayFix v3.0.0** in three copy-paste commands — watching a real Chromium browser reproduce and repair an open-source drag-jitter bug (`react-resizable` Issue #136) zero-shot from an `.mp4` video:

### Step 1: Build & Verify Environment (`10s`)
```bash
cargo build --quiet
./target/debug/replayfix doctor
```

### Step 2: Run Live Zero-Shot Reproduction & Repair in a Visible Browser (`40s`)
Ensure `META_API_KEY` is set in `.env` or your environment, then run:
```bash
./target/debug/replayfix repro 
  --video benchmarks/react-resizable/DEMO_react-resizable_issue-136.mp4 
  --repo benchmarks/react-resizable/repo 
  --live-browser 
  --headed
```

#### What You Will See Live on Screen:
```text
  ✔ Isolation      PASS  Scratch workspace initialized (ephemeral port :56581, base tree 3292f8c92bc3)
  ✔ Video Probe    PASS  Probed recording & extracted ROI keyframes (1280x720, 10080ms)
  ✔ SAM 3.1 Snap   PASS  Snapped hotspot to contour centroid -> CSS (610, 358)
  ✔ Trajectory     PASS  Aggregated multi-step interaction trajectory (7 actions sequenced)
  ✔ CDP Grounding  PASS  page.getByRole('slider', { name: 'Resize Handle NW', exact: true }) (score: 0.95)
  ✔ Gate 1 (RED)   PASS  Defect reproduced causal baseline (bounding-box drag jitter > 500 bps)
  ✔ Test Freeze    PASS  Anti-cheating SHA-256 test freeze verified (f860e50e7d45)
  ✔ Muse Repair    PASS  model: muse-spark-1.3 (effort: xhigh, attempt 1/3), cache: 2531/2607 tokens cached
  ✔ Syntax Check   PASS  Preflight validation passed in sandbox (target: lib/Resizable.js)
  ✔ Repair         PASS  Patch applied in scratch sandbox: lib/Resizable.js (+23 -14)
  ✔ Gate 2 (GREEN) PASS  Behavior verified across 2 consecutive headed Chromium runs

  ◆ Execution Authenticity & Forensic Matrix
       REAL    Video Ingestion               Live provider / browser verified
     HEURISTIC SAM Spatial Grounding         Derived via heuristic / optical flow
       REAL    Interaction Trajectory        Live provider / browser verified
       REAL    Deliberation Loop             Live provider / browser verified
       REAL    Code Patch Synthesis          Live provider / browser verified
       REAL    Gate 1 Baseline (RED)         Live provider / browser verified
       REAL    Gate 2 Stability (GREEN)      Live provider / browser verified
      SKIPPED  Gate 3 Stress Quorum          Not required for this invocation
       REAL    Cryptographic Attestation     Live provider / browser verified

  → Next: headed Chromium replay verified — inspect audit receipt at .replayfix/audit_receipt.json or pass --apply to apply patch.
```

### Step 3: Verify the Cryptographic Receipt (`5s`)
```bash
./target/debug/replayfix verify-receipt .replayfix/audit_receipt.json
```
Or open **`proof_viewer.html`** in any browser and drag-and-drop `.replayfix/audit_receipt.json` to inspect the 4-layer RFC 6962 Merkle inclusion proof and RFC 8032 Ed25519 signature completely offline.

---

## 💡 Why ReplayFix? (The Problem & Breakthrough)

When users encounter visual or interaction bugs, they record their screen and drop an `.mp4` into an issue ticket. Engineers spend **30–40% of their debugging time** manually translating video pixels into DOM selectors, local server setups, and reproduction steps.

Existing AI coding agents fail on video-reported UI bugs because they:
1. **Cannot map video pixels to live DOM nodes**: They guess selectors without calibrating viewport dimensions, Device Pixel Ratio (DPR), or ARIA trees.
2. **Fail on raw UI component libraries**: Repositories like `react-resizable` or `react-date-picker` export raw ES6/Flow/TypeScript components without a standalone dev server.
3. **Hallucinate fixes without causal proof**: They propose patches without proving the bug failed before the patch (Gate 1 RED) or stabilized after the patch (Gate 2 GREEN).
4. **Tamper with tests to fake passes**: Without cryptographic test-freeze guards, agents weaken assertions to turn red tests green.

### What Makes ReplayFix v3.0.0 Different
| Capability | Typical Coding Agent | ReplayFix v3.0.0 (Universal Zero-Shot) |
| :--- | :--- | :--- |
| **Input Modality** | Text prompt / stack trace | Raw `.mp4`/`.webm` screen recording + RRWeb/Sentry logs |
| **Target Repo Support** | Requires pre-configured app server | **Universal**: Full-stack apps **and** raw Flow/TS/JSX component libraries |
| **DOM Grounding** | Guesses CSS selectors from source | Live Playwright CDP `elementFromPoint(x, y)` + `ariaSnapshot` margin scoring |
| **Drag & Physics Bugs** | Blind to layout oscillation/jitter | Step-by-step physical bounding-box acceleration telemetry (`jitter_score_bps`) |
| **Verification Authority** | LLM self-assessment | Counterfactual Two-Gate FSM (Gate 1 RED $	o$ Patch $	o$ Gate 2 GREEN $	imes 2$) |
| **Stage Honesty** | Opaque fallbacks | Explicit 9-stage **Forensic Provenance Matrix** (`REAL` / `HEURISTIC` / `MOCKED` / `SKIPPED`) |
| **Audit Trail** | Unverified diff | **RFC 6962 Merkle Tree** + **RFC 8032 Ed25519** digital signature |

---

## 🔬 The Two-Gate Causal Verification Oracle

ReplayFix enforces strict counterfactual causality: **no candidate patch is ever accepted unless the harness first proves the defect reproduces on the unpatched repository.**

```text
               ┌────────────────────────────────────────────────────────┐
               │    Raw Bug Video (.mp4) + Target Git Repository        │
               └───────────────────────────┬────────────────────────────┘
                                           │
                                           ▼
               ┌────────────────────────────────────────────────────────┐
               │  1. Perception & Trajectory Extraction                 │
               │  • FFmpeg keyframe + inter-frame optical-flow ROI      │
               │  • SAM 3.1 / Frame-Pair Centroid & Drag Displacement   │
               └───────────────────────────┬────────────────────────────┘
                                           │
                                           ▼
               ┌────────────────────────────────────────────────────────┐
               │  2. Ephemeral Scratch Sandbox & Universal Harness      │
               │  • Isolated temp worktree on kernel port :0 (Δrepo = 0)│
               │  • Auto-synthesizes Babel [env, flow, react] harness   │
               └───────────────────────────┬────────────────────────────┘
                                           │
                                           ▼
               ┌────────────────────────────────────────────────────────┐
               │  3. Gate 1 (Counterfactual RED) in Live Chromium       │
               │  • Grounds (x, y) via CDP elementFromPoint + ARIA tree │
               │  • Replays click/drag; MUST fail (error or jitter>500) │
               └───────────────────────────┬────────────────────────────┘
                                           │
                                           ▼
               ┌────────────────────────────────────────────────────────┐
               │  4. Meta Muse Bounded AST Repair (Max 3 Attempts)      │
               │  • SHA-256 TestFreezeGuard prevents test tampering     │
               │  • Exact single-match search/replace + symbol check    │
               └───────────────────────────┬────────────────────────────┘
                                           │
                                           ▼
               ┌────────────────────────────────────────────────────────┐
               │  5. Gate 2 (2x Consecutive GREEN) + Cryptographic Seal │
               │  • Re-verifies in live Chromium twice consecutively    │
               │  • Emits RFC 6962 Merkle Receipt + Ed25519 Signature   │
               └────────────────────────────────────────────────────────┘
```

---

## 🧠 Universal Zero-Shot Architecture (`v3.0.0`)

ReplayFix v3.0.0 introduces four key architectural subsystems in `crates/replayfix-core`:

### 1. Universal Zero-Shot Component Mount Harness (`discovery.rs`)
When a target repository is a UI library (such as `react-resizable`) without a runnable application server or bundled demo HTML:
- **Static Asset Verification**: Inspects candidate HTML files (`examples/*.html`, `public/index.html`) and rejects broken templates that reference unbuilt webpack bundles.
- **In-Browser Babel Transpilation (`['env', 'flow', 'react']`)**: Synthesizes a zero-config HTML harness with a recursive CommonJS `dynamicRequire` loader that strips Flow annotations (`// @flow`, `import type`, `export type`) and transpiles ES6/Flow/JSX modules on the fly.
- **Physical Offset-Parent Drag Surface (`SelfContainedDraggableCore`)**: Computes drag deltas relative to `node.offsetParent.getBoundingClientRect()` and assigns axis-specific ARIA slider labels (`Resize Handle NW`, `Resize Handle SE`), faithfully reproducing real DOM layout oscillation bugs on unpatched code.

### 2. Multi-Frame Optical-Flow Trajectory & SAM 3.1 Perception (`perception.rs`, `video.rs`)
- **Frame-Pair Displacement (`RoiDiffer::pair_change`)**: Computes motion vectors across dense 100ms video frames to extract both the initial interaction hotspot `(click_x, click_y)` and drag terminal coordinates `(drag_end_x, drag_end_y)`.
- **Transparent Fallback Provenance**: Queries live `sam-3.1` on `https://api.meta.ai/v1/responses` and automatically falls back to deterministic local frame-pair segmentation (`local_diff_heuristic`) if upstream returns empty tokens, recording `HEURISTIC` in the receipt's `StageExecutionMatrix`.

### 3. Live Playwright CDP Grounding & Physical Jitter Oracle (`browser.rs`)
- **Zero-Positional-Index Grounding**: Resolves `(x, y)` coordinates using `document.elementFromPoint(x, y)` cross-referenced against Playwright's `ariaSnapshot`. Positional `.first()` or `.nth()` selectors are strictly forbidden; ambiguous ties (`margin < 0.25`) fail closed with `AMBIGUOUS_GROUNDING` (exit code `22`).
- **Step-by-Step Bounding-Box Jitter Telemetry (`DRAG_AND_OBSERVE`)**: Executes 10-step mouse drag trajectories while sampling `targetLocator.boundingBox()` after every frame. Direction reversals and acceleration spikes (`|curDx - prevDx| > 2px`) are quantified in basis points (`jitter_score_bps`), failing Gate 1 when `jitter_score_bps > 500` and passing Gate 2 when `jitter_score_bps <= 500`.

### 4. Meta Muse 1.3 / 1.0 Bounded AST Repair (`repair.rs`, `gateway.rs`)
- **24-Hour Prefix Caching**: Separates the static security instruction prefix (`replayfix-v0.6-harness-prefix-v1`) from untrusted repository context wrapped in `<UNTRUSTED_CONTEXT>` fences, achieving **>95% prompt cache hits** on retry/verification turns.
- **Lexical Scope & Symbol Verification (`BlockPatcher`)**: Tracks parenthesis depth and ES6/Flow class method declarations (`resizeHandler(...)`) to guarantee every patch matches uniquely (`count == 1`) and touches only declared symbols.

---

## 🛡️ The Inviolable Engineering Invariants

| Law | Invariant | Enforcement Mechanism |
| :---: | :--- | :--- |
| **Law I** | **Substrate Primacy** | The deterministic Rust state machine (`HealEngine`) is the sole authority—never unverified model text. |
| **Law II** | **Fail-Closed Verification** | Ambiguous DOM grounding, syntax errors, or non-RED Gate 1 baselines abort immediately with typed exit codes. |
| **Law IV** | **Base Repo Immutability ($\Delta	ext{repo} = 0$)** | All builds, servers, and patches execute inside an ephemeral OS temp worktree (`ScratchSandbox`). Base repo tree SHA-256 is verified before and after. |
| **Law V** | **Anti-Cheating Test Freeze** | `TestFreezeGuard` hashes all test files and `package.json` test scripts via SHA-256 prior to repair; any mutation aborts with exit code `11`. |
| **Law VII** | **Bounded Repair Budget** | Hard ceiling of **3 attempts** and **180 seconds** wall-clock budget per defect run. |
| **Law X** | **Zero-Panic Safe Rust** | Workspace-wide `#![forbid(unsafe_code)]` and zero non-test `.unwrap()`/`.expect()`/`panic!()`. |

---

## 🛠️ Complete CLI Command Reference

| Command | Purpose | Example |
| :--- | :--- | :--- |
| **`replayfix repro`** | Zero-shot video-to-code reproduction, repair & 2-gate browser verification | `replayfix repro --video bug.mp4 --repo ./app --live-browser --headed` |
| **`replayfix heal`** | Autonomous multi-turn `muse-spark-1.3` deliberation & `muse-wonder-1.0` repair | `replayfix heal --video bug.mp4 --repo ./app --mode live-ai` |
| **`replayfix doctor`** | Audit local environment (`git`, `node`, `playwright`, `ffmpeg`, loopback) | `replayfix doctor` |
| **`replayfix verify-receipt`** | Verify 4-layer RFC 6962 Merkle proofs & Ed25519 signatures | `replayfix verify-receipt .replayfix/audit_receipt.json` |
| **`replayfix notary`** | Append receipts & verify RFC 6962 consistency proofs on transparency log | `replayfix notary verify-log --log .replayfix/transparency_log.json` |
| **`replayfix swarm`** | Multi-worker work-stealing triage cluster across isolated sandboxes | `replayfix swarm --workers 4 --bugs RF-01,RF-02,RF-03,RF-04` |
| **`replayfix dashboard`** | Live WebSocket/SSE observability deck streaming browser frames & tokens | `replayfix dashboard --port 3848 --artifacts .replayfix` |
| **`replayfix inoculate`** | Generate permanent cross-browser/mobile Playwright regression specs | `replayfix inoculate --manifest manifest.json --matrix chromium,mobile` |
| **`replayfix ingest`** | Convert RRWeb session replays & Sentry crash dumps into task manifests | `replayfix ingest --rrweb events.json --sentry crash.json` |

---

## 📊 Deterministic Exit Code Taxonomy

| Exit Code | Verdict Constant | Meaning |
| :---: | :--- | :--- |
| **`0`** | `VERIFIED_REPAIRED` | Defect reproduced (Gate 1 RED), repaired, verified (Gate 2 GREEN $	imes 2$), and receipt signed. |
| **`1`** | `DOCTOR_ERROR` | Missing system dependency (`ffmpeg`, `git`, `playwright`, or `node`). |
| **`2`** | `USAGE_ERROR` | Invalid CLI arguments, malformed manifest, or disallowed model slug. |
| **`10`** | `GATE1_NOT_RED` | Defect did not reproduce on Gate 1 (counterfactual baseline failed). |
| **`11`** | `SECURITY_VIOLATION` | Path traversal, base repo mutation ($\Delta	ext{repo} 
eq 0$), or anti-cheat test freeze violation. |
| **`20`** | `BRIDGE_EXECUTION_FAILED` | Playwright CDP bridge timeout or browser crash. |
| **`21`** | `QUARANTINE_REPAIR_FAILED` | Bounded 3-attempt repair budget exhausted or flaky Gate 3 stress quorum failure. |
| **`22`** | `AMBIGUOUS_GROUNDING` | Multiple DOM elements tied within `< 0.25` score margin at target coordinates. |
| **`30`** | `GATE2_NOT_GREEN` | Candidate patch failed to apply, failed syntax preflight, or failed Gate 2 verification. |
| **`31`** | `REGRESSION_DETECTED` | Candidate patch broke pre-existing regression test suite. |
| **`40`** | `RECEIPT_INVALID` | Artifact tampering, Merkle inclusion mismatch, or invalid Ed25519 signature. |

---

## 📚 Documentation Hub

Explore the complete engineering documentation in [`docs/`](docs/README.md):

- [**Architecture & Two-Gate Verification Substrate**](docs/architecture.md) — Deep dive into the FSM, zero-shot Babel component harness, optical-flow drag trajectory, and physical jitter telemetry.
- [**CLI & Operator Reference**](docs/cli-reference.md) — Exhaustive flag tables, environment variables, and workflow recipes.
- [**Trust & Cryptographic Architecture**](docs/trust-architecture.md) — RFC 6962 Merkle tree specification, Ed25519 signing payload format, and 4-layer verification.
- [**Agent Deliberation State Machine**](docs/agent-state.md) — Multi-turn `muse-spark-1.3` deliberation loop, prefix caching, and encrypted reasoning continuity.
- [**Rust Engineering Principles**](docs/engineering-principles-rust.md) — `#![forbid(unsafe_code)]`, zero-panic enforcement, and deterministic integer accounting.
- [**Release Notes (`v0.0.0` – `v3.0.0`)**](docs/release-notes/v3.0.0.md) — Full evolutionary history of ReplayFix.

---

## 📄 License

Distributed under the **MIT License**. See [LICENSE.md](LICENSE.md) for details.
