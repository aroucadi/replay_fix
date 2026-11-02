# Implementation Plan: Spec 160 — Sovereign Autonomous Healer: Multi-Turn Muse Spark Agent Loop & Sub-60s Macro Pipeline (Release 1.6.0)

**Branch**: `release/v1.6.0` | **Date**: 2026-11-02 | **Spec**: [spec.md](./spec.md)  
**Input**: Feature specification from `/specs/160-v1.6.0-sovereign-healer-macro/spec.md`

## Summary

Release 1.6.0 transforms ReplayFix into an autonomous sovereign healer governed by Meta Muse Spark 1.3 as the multi-turn agent loop brain, paired with Meta Muse Wonder 1.0 for deep-deliberation AST repair with encrypted reasoning backtracking (`reasoning.encrypted_content`), executed through a sub-60-second end-to-end CLI macro (`replayfix heal --video <path> --repo <path>`).

Key Architectural Deliverables:
1. **`crates/replayfix-core/src/deliberation.rs`**: Core multi-turn agent loop coordinator (`DeliberationEngine`) managing turns, thoughts, tool invocations (`replayfix_reproduce`, `replayfix_inspect_trace`, `replayfix_verify_patch`, `replayfix_inoculate`), error backtracking, and execution lineage with `prompt_cache_key: "replayfix-v1.6-spark-agent-loop"`.
2. **`crates/replayfix-core/src/gateway.rs` & `repair.rs`**: Live and mock tool-calling support on `muse-spark-1.3`, Wonder 1.0 AST patch synthesis with `previous_response_id` and encrypted reasoning token persistence, and token accounting.
3. **`crates/replayfix-cli/src/heal.rs` & `main.rs`**: Unified `replayfix heal` subcommand orchestrating Video Ingestion ➔ FFmpeg Keyframes ➔ SAM 3.1 ➔ Voice Lock ➔ Ephemeral Server (:0) ➔ Spark Multi-Turn Loop ➔ Wonder AST ➔ Gate 2 Stability ➔ Gate 3 Stress Quorum ➔ Inoculation ➔ RFC 6962 Merkle + Ed25519 Receipt in `< 60 seconds`.
4. **`crates/replayfix-core/src/receipt.rs` & `portal.rs`**: Hardened `proof_viewer.html` and `audit_receipt.json` with an interactive multi-turn deliberation timeline, side-by-side accessibility tree DOM diffs, and Merkle leaf verification.

## Technical Context

**Language/Version**: Rust 2021 (`1.75+`)  
**Primary Dependencies**: `serde`, `serde_json`, `clap`, `tokio`, `reqwest`, `ed25519-dalek`, `sha2`  
**Testing**: `cargo test --workspace --all-targets -- --test-threads=2` (100% parity across 283+ existing tests + new v1.6 tests)  
**Constraints**: `#![forbid(unsafe_code)]`, zero unhandled `.unwrap()` outside tests, 0 Clippy warnings (`-D warnings`), `check-panics.ps1` clean.

## Constitution Check

- **Law I (Substrate Primacy & Fail-Closed Authority)**: The Rust harness is the sole judicial authority. Spark directs strategy and issues tool calls, but verdicts (Gate 1 RED, Gate 2 GREEN, Gate 3 Quorum) are determined strictly by deterministic substrate tests.
- **Law II (Behavioral Contract Invariance)**: Gate 1 and Gate 2 evaluate the exact same immutable `BehavioralContract` verified via canonical RFC 8785 digests.
- **Law III (Inviolable Base Workspace Isolation)**: Base repo remains read-only ($\Delta\text{repo}=0$). All repairs and server boots run inside ephemeral shadow scratch directories.
- **Law IV (Child-Owned Ephemeral Provenance)**: Dev server binds to OS-assigned port 0 (`:0`), monitored by the child process supervisor.
- **Law V (Regression Parity)**: Existing test suite must maintain 100% pass parity before a patch can be certified.
- **Law VI (Monotonic Event Ordering)**: Failures qualify as Gate 1 RED only if setup and target resolution succeed before assertion failure.
- **Law VII (Model Honesty & Provider Configuration)**: Exact model slugs (`muse-spark-1.3`, `muse-wonder-1.0`, `sam-3.1`, `muse-voice-transcribe-1.0`) and fallback flags are honestly recorded.
- **Law IX (Tamper-Evident Receipts)**: RFC 6962 standard Merkle tree with Ed25519 signature binds deliberation timeline, keyframes, DOM diffs, and test outputs.
- **Law X (BeautifulRust Standards)**: Zero-panic architecture, type-driven state machines, structured `thiserror` domain error handling in `CoreError`.

## Module Architecture & Workflow

```
replayfix heal --video <video.mp4> --repo <path>
       │
       ▼
[crates/replayfix-cli/src/heal.rs]
  ├── Phase 1: Video Ingestion & FFmpeg Keyframe Extraction
  ├── Phase 2: SAM 3.1 Contours & Muse Voice ASR Lock
  ├── Phase 3: Ephemeral Scratch Sandbox & Port 0 Dev Server Boot
  │
  ├── Phase 4: DeliberationEngine (crates/replayfix-core/src/deliberation.rs)
  │     ├── Turn 1: Spark calls `replayfix_reproduce` ➔ Gate 1 RED Baseline
  │     ├── Turn 2: Spark diagnoses causal symptom ➔ delegates to Muse Wonder 1.0 (AST block patch)
  │     ├── Turn 3: Spark calls `replayfix_verify_patch` ➔ Gate 2 GREEN Stability
  │     │             └── [If fails]: Turn 4 Backtracking with `previous_response_id` & `encrypted_content`
  │     ├── Turn 4: Gate 3 Stress Quorum (flakiness detection under network throttling)
  │     └── Turn 5: Spark calls `replayfix_inoculate` ➔ writes `tests/repro/rf_<id>.spec.ts`
  │
  └── Phase 5: Cryptographic Attestation & Receipt Export
        ├── RFC 6962 Merkle Tree (10+ canonical leaves) + Ed25519 Signature
        ├── audit_receipt.json with full deliberation session & lineage
        └── proof_viewer.html with Deliberation Timeline & DOM Diff Inspector
```
