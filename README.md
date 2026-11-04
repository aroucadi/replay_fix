<div align="center">

# 🎬 ReplayFix v2.0.0

### Sovereign Autonomous Visual Bug Repair Harness & Verifiable Notary

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE.md)
[![Rust](https://img.shields.io/badge/Rust-1.85%2B%20(Pure%20Safe)-orange?logo=rust)](https://www.rust-lang.org)
[![Meta AI Models](https://img.shields.io/badge/Meta%20AI-Muse%20%26%20SAM%203.1-0081FB?logo=meta)](https://ai.meta.com)
[![SWE-bench Causal](https://img.shields.io/badge/Verification-Dual--Gate%20Causal-success)](#-the-two-gate-causal-verification-oracle)
[![Security Invariant](https://img.shields.io/badge/%CE%94repo%20%3D%200-Strictly%20Isolated-blueviolet)](#-the-inviolable-engineering-invariants)
[![Merkle Proof](https://img.shields.io/badge/RFC%206962-Ed25519%20Signed-teal)](#-cryptographic-notary--merkle-audit-receipts)
[![MCP Native](https://img.shields.io/badge/MCP-Native%20Server-7B2CBF)](https://modelcontextprotocol.io)

<p align="center">
  <b>Give it a video. Give it a repo. Get a verified, cryptographically signed fix in under 60 seconds.</b>
</p>

<p align="center">
  <a href="https://www.youtube.com/watch?v=cPDIFQmVn9I"><b>Watch 4K Demo Video</b></a> •
  <a href="#-quickstart--cli-reference"><b>Quickstart</b></a> •
  <a href="#-core-architecture--the-meta-ai-trio"><b>Meta AI Architecture</b></a> •
  <a href="#-the-two-gate-causal-verification-oracle"><b>Two-Gate Oracle</b></a> •
  <a href="#-live-streaming-dashboard--swarm"><b>Live Dashboard & Swarm</b></a> •
  <a href="proof_viewer.html"><b>Judge Proof Portal</b></a>
</p>

</div>

---

## 💡 Why ReplayFix?

When users encounter software defects, they capture a screen recording, drop a video in Slack, or submit an issue ticket. Senior software engineers burn **30% to 40% of their sprints** trying to manually reproduce what broke: guessing interaction steps, setting up local ports, and wrestling with timing flakes.

Generic AI coding assistants fail at visual bugs because:
1. **Blind to pixels & layouts**: They cannot perceive screen resolutions, device pixel ratios (DPR), or dynamic viewport letterboxing.
2. **Hallucinate UI selectors**: They guess fragile selectors that drift from the live DOM.
3. **Reward hacking & test tampering**: They soften test assertions or edit test suites to fake a green pass.
4. **No causal attribution**: They guess patches without proving the bug actually failed first.

**ReplayFix solves this with systems-grade rigor.** Built in 100% deterministic, zero-panic Rust (`#![forbid(unsafe_code)]`), ReplayFix turns raw screen recordings (`.mp4`/`.webm`) and APM session logs into verified, regression-free source patches certified by an **RFC 6962 Merkle tree receipt** signed with **RFC 8032 Ed25519**.

---

## 🔬 The Two-Gate Causal Verification Oracle

ReplayFix enforces counterfactual causality: **no code is touched until the defect is mathematically isolated and reproduced.**

```text
               ┌────────────────────────────────────────────────────────┐
               │    Raw User Video (.mp4) / APM Session Replay (.json)  │
               └───────────────────────────┬────────────────────────────┘
                                           │
                                           ▼
               ┌────────────────────────────────────────────────────────┐
               │  Meta SAM 3.1 & CDP Spatial-Semantic Element Snapping  │
               └───────────────────────────┬────────────────────────────┘
                                           │
                                           ▼
               ┌────────────────────────────────────────────────────────┐
               │   Gate 1: Counterfactual Reproduction Test Synthesis   │
               └───────────────────────────┬────────────────────────────┘
                                           │
                      ┌────────────────────┴────────────────────┐
                      ▼                                         ▼
          [Ephemeral Scratch Worktree]                  [Host Repository]
          Dynamic Ephemeral Port :0                     STRICTLY READ-ONLY
          Must FAIL (RED) with AssertionError           Δrepo = 0 Invariant
                      │                                         │
                      └────────────────────┬────────────────────┘
                                           │
                                           ▼
               ┌────────────────────────────────────────────────────────┐
               │  Meta Muse Wonder: AST-Bounded Search/Replace Patch    │
               └───────────────────────────┬────────────────────────────┘
                                           │
                                           ▼
               ┌────────────────────────────────────────────────────────┐
               │    Gate 2: 2-Run Stability Pass & Regression Parity    │
               │    Must PASS (GREEN x2) with Zero Suite Regressions    │
               └───────────────────────────┬────────────────────────────┘
                                           │
                                           ▼
               ┌────────────────────────────────────────────────────────┐
               │  RFC 6962 Merkle Tree Audit Receipt + Ed25519 Seal     │
               └────────────────────────────────────────────────────────┘
```

1. **Gate 1 (Counterfactual RED)**: ReplayFix synthesizes a Playwright test and executes it against the unpatched codebase. It MUST fail with a verified `AssertionError`.
2. **Gate 2 (2-Run Stability GREEN)**: Once Meta Muse Wonder generates an AST search/replace block patch, the test must pass **two consecutive stability runs** and survive Gate 3 latency perturbations.
3. **Regression Parity Guarantee**: The full pre-existing test suite must pass with 100% parity ($\mathcal{F}_{\text{patched}} \setminus \mathcal{F}_{\text{baseline}} = \emptyset$).
4. **Anti-Cheat Test Freeze**: Synthesized tests are hashed via SHA-256 before repair; patches that touch test files or relax assertions are rejected with exit code `11` (`SECURITY_VIOLATION`).

---

## 🧠 Core Architecture & The Meta AI Trio

ReplayFix orchestrates Meta's state-of-the-art open model ecosystem as a unified neural verification engine:

```text
┌────────────────────────────────────────────────────────────────────────────────────────┐
│                        REPLAYFIX v2.0.0 SOVEREIGN NEURAL PIPELINE                      │
│                                                                                        │
│   ┌───────────────────────────────────┐    ┌───────────────────────────────────────┐   │
│   │   1. SPATIAL PERCEPTION           │    │   2. CAUSAL REASONING                 │   │
│   │   Meta SAM 3.1                    │    │   Meta Muse Spark 1.3                 │   │
│   │   • Video keyframe extraction     │───▶│   • 305+ tok/s deliberation stream    │   │
│   │   • Coordinate & DPR calibration  │    │   • Action trajectory reconstruction  │   │
│   │   • Sub-pixel centroid snapping   │    │   • Gate 1 RED test synthesis         │   │
│   └───────────────────────────────────┘    └───────────────────┬───────────────────┘   │
│                                                                │                       │
│                                                                ▼                       │
│   ┌───────────────────────────────────┐    ┌───────────────────────────────────────┐   │
│   │   4. SOVEREIGN VERIFICATION       │    │   3. DETERMINISTIC REPAIR             │   │
│   │   Rust Verification Harness       │    │   Meta Muse Wonder 1.0                │   │
│   │   • Ephemeral ScratchSandbox      │◀───│   • AST search/replace block patching │   │
│   │   • Gate 2 GREEN stability pass   │    │   • Zero hallucination, zero style    │   │
│   │   • RFC 6962 Merkle proof notary  │    │     drift                             │   │
│   └───────────────────────────────────┘    └───────────────────────────────────────┘   │
└────────────────────────────────────────────────────────────────────────────────────────┘
```

* **Meta SAM 3.1 (Segment Anything Model)**: Ingests video frames from FFmpeg, isolates optical flow regions of interest (ROI), and snaps imprecise human clicks to exact semantic DOM bounding boxes and ARIA accessibility trees.
* **Meta Muse Spark**: High-velocity multimodal agent brain streaming reasoning at over 300 tokens/sec. Reconstructs user intent, flags missing state validations, and outputs executable Playwright reproduction specs.
* **Meta Muse Wonder**: Surgical AST search/replace block patch generator. It repairs the root cause directly inside the target code without rewriting unrelated files.

---

## 🛡️ The Inviolable Engineering Invariants

| Invariant | Standard | Enforcement Mechanism |
| :--- | :--- | :--- |
| **Substrate Primacy** | Law I | Deterministic Rust harness is the sole verification authority—never unverified LLM text. |
| **Fail-Closed Verification** | Law II | Unhandled errors, timeouts, or ambiguous DOM matches fail closed. |
| **Base Repo Immutability** | Law IV | $\Delta\text{repo} = 0$. All runs occur in isolated, ephemeral temp clones on dynamic ports (`:0`). |
| **Anti-Cheating Test Freeze** | Law V | Reproduction tests are frozen via SHA-256 before repair; patches modifying tests fail closed. |
| **Regression Parity** | Law VI | Pre-existing test suite must maintain 100% pass parity. Zero regressions tolerated. |
| **Zero-Panic Systems Baseline** | Law X | Strictly `#![forbid(unsafe_code)]` with 0 non-test panics (`check-panics.ps1`). |

---

## ⚡ Live Streaming Dashboard & Distributed Swarm

ReplayFix includes an enterprise-grade distributed triage cluster and real-time observability server:

```bash
# Launch multi-worker work-stealing triage swarm across isolated worktrees:
replayfix swarm --workers 4 --bugs RF-01,RF-02,RF-03,RF-04 --output .replayfix

# Boot live streaming WebSocket & SSE observability dashboard:
replayfix dashboard --port 3848 --artifacts .replayfix
```

* **Live CDP Screencast Broadcaster**: Streams real-time Chromium viewport screencasts (JPEG/WebP over WebSocket) as the bug is reproduced and repaired.
* **Muse Spark Deliberation Feed**: Streams turn-by-turn agent reasoning tokens and tool executions live to connected maintainer browsers.
* **Side-by-Side Proof Diff**: Interactive UI displaying before/after DOM `ariaSnapshot` trees, bounding boxes, optical flow vectors, and Merkle tree inclusion paths.

---

## 🔐 Cryptographic Notary & Merkle Audit Receipts

Every verified repair produces an **RFC 6962 Merkle Tree Audit Receipt** signed with an **RFC 8032 Ed25519** keypair:

```bash
# Verify a repair receipt through all 4 cryptographic layers:
replayfix verify-receipt .replayfix/audit_receipt.json --trusted-key <HEX_PUBKEY>

# Append receipt and verify consistency proofs on the append-only transparency ledger:
replayfix notary append --receipt .replayfix/audit_receipt.json
replayfix notary verify-log --log .replayfix/transparency_log.json
```

Inspect receipts offline without a server using **`proof_viewer.html`**—an air-gapped single-file verification portal with client-side WebCrypto Ed25519 signature checks (`default-src 'none'`).

---

## 🚀 Quickstart & CLI Reference

### 1. Installation

```bash
# Clone the repository
git clone https://github.com/aroucadi/replay_fix.git
cd replay_fix

# Build release binary (Pure Rust, Zero Unsafe)
cargo build --release --workspace

# Run self-diagnostic environment check
./target/release/replayfix doctor
```

### 2. Autonomous Reproduction & Repair (`replayfix repro`)

```bash
# Provide a video of the bug and access to the repository:
replayfix repro \
  --video tests/fixtures/videos/repro_demo.mp4 \
  --repo ./target-app \
  --output-dir .replayfix/
```

### 3. Production APM & RRWeb Telemetry Ingestion (`replayfix ingest`)

```bash
# Transform RRWeb session streams and Sentry crash dumps into reproducible manifests:
replayfix ingest --rrweb session_events.json --sentry crash_dump.json --output .replayfix/manifest.json
```

### 4. Cross-Browser & Multi-Device Inoculation (`replayfix inoculate`)

```bash
# Synthesize permanent Playwright regression tests calibrated for desktop & mobile DPR:
replayfix inoculate --manifest .replayfix/manifest.json --matrix chromium,firefox,webkit,mobile
```

### 5. Automated GitHub Issue Triage Bot (`replayfix bot`)

```bash
# Run continuous GitHub issue ingestion, reproduction, and PR dispatch:
replayfix bot --repo owner/target-repo --poll-interval 60 --auto-pr
```

---

## 📊 Deterministic Exit Code Taxonomy

| Exit Code | Verdict | Meaning |
| :---: | :--- | :--- |
| **`0`** | `VERIFIED_REPAIRED` | Defect reproduced (Gate 1 RED), repaired, verified (Gate 2 GREEN x2), and receipt signed. |
| **`1`** | `DOCTOR_ERROR` | Missing system dependency (`ffmpeg`, `git`, `playwright`, or `node`). |
| **`2`** | `USAGE_ERROR` | Invalid CLI arguments, malformed manifest, or unrecognized model slug. |
| **`10`** | `GATE1_NOT_RED` | Defect did not reproduce on Gate 1 (counterfactual baseline failed). |
| **`11`** | `SECURITY_VIOLATION` | Path traversal, base repo mutation detected, or anti-cheat test freeze violated. |
| **`21`** | `QUARANTINE_FAILED` | Repair budget exhausted or non-deterministic flaky failure. |
| **`31`** | `REGRESSION_DETECTED` | Candidate patch broke pre-existing regression test suite. |
| **`40`** | `RECEIPT_INVALID` | Artifact tampering, Merkle inclusion mismatch, or invalid Ed25519 signature. |

---

## 🏆 Benchmark & Qualification Sweep

ReplayFix includes the **MetaPulse 20-Bug Benchmark Suite** (`RF-01` through `RF-20`), testing real-world visual regressions across responsive web apps:

```bash
# Run the complete benchmark sweep:
replayfix sweep --suite metapulse --concurrency 4 --output-dir target/sweep-results/
```

- **326+ Automated Tests**: 100% pass rate with zero ignored tests.
- **Zero-Panic Quality**: Strictly 0 non-test panics (`check-panics.ps1`).
- **Clippy Clean**: Compiles under `cargo clippy --workspace --all-targets -- -D warnings` with 0 warnings.

---

## 📄 License & Trust Model

Distributed under the **MIT License**. See [LICENSE.md](LICENSE.md) for details.

- **Security Architecture**: [docs/trust-architecture.md](docs/trust-architecture.md)
- **Windows Authenticode Signing**: [docs/windows-code-signing.md](docs/windows-code-signing.md)
- **Changelog & Milestones**: [CHANGELOG.md](CHANGELOG.md)

<div align="center">
  <sub>Built for the <b>Meta Global AI Developer Hackathon 2026</b> • Powered by Meta Muse Spark, Muse Wonder & SAM 3.1</sub>
</div>
