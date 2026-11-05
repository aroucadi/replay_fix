# 📚 ReplayFix v3.0.0 Documentation Hub

Welcome to the official technical documentation for **ReplayFix v3.0.0** — the universal, zero-shot, video-driven autonomous debugging and repair harness built in safe, deterministic Rust (`#![forbid(unsafe_code)]`) and powered by Meta AI (`muse-spark-1.3`, `muse-wonder-1.0`, and `sam-3.1`).

---

## 🧭 Jury & Engineer Navigation Matrix

| Document | What It Covers | Primary Audience |
| :--- | :--- | :--- |
| [**Architecture & Zero-Shot Substrate**](architecture.md) | Two-Gate causal FSM, Universal Babel (`['env', 'flow', 'react']`) component harness, optical-flow drag trajectories, physical bounding-box jitter oracle, and the 9-stage `StageExecutionMatrix`. | Hackathon Jury, Systems Architects |
| [**CLI & Operator Guide**](cli-reference.md) | Exhaustive reference for `repro`, `heal`, `verify-receipt`, `notary`, `swarm`, `dashboard`, `inoculate`, `ingest`, and `doctor`, including all flags and exit codes (`0`–`40`). | Judges, Developers, SREs |
| [**Trust & Cryptographic Model**](trust-architecture.md) | RFC 6962 SHA-256 Merkle tree construction, RFC 8032 Ed25519 digital signatures, append-only transparency ledger, and 4-layer offline verification (`proof_viewer.html`). | Security Auditors, Cryptography Reviewers |
| [**Agent State Machine**](agent-state.md) | Multi-turn `muse-spark-1.3` / `muse-wonder-1.0` deliberation lifecycle, 24h static prefix caching, `<UNTRUSTED_CONTEXT>` fencing, and `BlockPatcher` scope verification. | AI Researchers, Core Contributors |
| [**Engineering Principles (BeautifulRust)**](engineering-principles-rust.md) | The 10 Inviolable Laws, `#![forbid(unsafe_code)]`, zero-panic discipline (`check-panics.ps1`), and integer-only nano-USD token accounting. | Rust Engineers |
| [**Windows Code Signing & WDAC**](windows-code-signing.md) | Authenticode signing, Smart App Control compatibility, and reproducible release packaging. | Release & Security Engineers |
| [**Release Notes (`v3.0.0` Latest)**](release-notes/v3.0.0.md) | Detailed breakdown of the `v3.0.0` Universal Zero-Shot milestone and full changelog history (`v0.0.0` – `v3.0.0`). | All Readers |

---

## ⚡ The ReplayFix v3.0.0 End-to-End Pipeline

```text
[Bug Video .mp4 / WebM] + [Any Target Git Repo]
           │
           ▼
  [1. Video & Spatial Perception]
  • FFmpeg keyframe + dense 100ms frame-pair ROI differencing (RoiDiffer::pair_change)
  • Meta SAM 3.1 segmentation with transparent local_diff_heuristic fallback
  • Extracts initial click (x, y) and terminal drag vector (drag_end_x, drag_end_y)
           │
           ▼
  [2. Ephemeral Scratch Sandbox & Universal Harness]
  • Clones repo into OS temp dir on kernel-assigned port :0 (Δrepo = 0 invariant)
  • Auto-synthesizes Babel Standalone ['env', 'flow', 'react'] component mount if no app server exists
           │
           ▼
  [3. Gate 1: Counterfactual RED in Live Chromium]
  • Grounds (x, y) via Playwright CDP elementFromPoint + ariaSnapshot (margin >= 0.25)
  • Replays click or 10-step drag; measures runtime errors and physical bounding-box jitter (jitter_score_bps)
  • MUST fail (RED) before any repair is attempted
           │
           ▼
  [4. Meta Muse Bounded AST Repair (Max 3 Attempts)]
  • TestFreezeGuard locks all test files & package.json test scripts via SHA-256
  • Muse Spark 1.3 / Muse Wonder 1.0 synthesizes strict 5-field JSON block patch
  • BlockPatcher verifies single-match uniqueness & enclosing lexical/method symbols
           │
           ▼
  [5. Gate 2: 2-Run Consecutive GREEN & Cryptographic Receipt]
  • Verifies patched component across 2 consecutive live Chromium runs
  • Emits StageExecutionMatrix + RFC 6962 Merkle Receipt signed with Ed25519
```

---

## 🛡️ Four Core Guarantees for Judges

1. **Zero Fixture Cheating in Live Mode**: When running against external benchmarks (`react-resizable`, etc.), ReplayFix performs zero task-ID lookups, zero hardcoded diffs, and zero selector overrides.
2. **Honest Stage Provenance (`StageExecutionMatrix`)**: Every receipt and CLI run explicitly classifies all 9 pipeline stages as `REAL`, `HEURISTIC`, `MOCKED`, `HARDCODED`, `SKIPPED`, `FAILED`, or `UNVERIFIED`.
3. **Strict Base Repository Immutability ($\Delta	ext{repo} = 0$)**: The target repository's SHA-256 tree hash is verified before and after execution; all mutations happen exclusively in an ephemeral scratch sandbox unless `--apply` is explicitly passed.
4. **4-Layer Cryptographic Verifiability**: Every artifact in `.replayfix/artifacts/` is hashed into an RFC 6962 Merkle tree and signed with an Ed25519 keypair, verifiable via `replayfix verify-receipt` or offline in `proof_viewer.html`.
