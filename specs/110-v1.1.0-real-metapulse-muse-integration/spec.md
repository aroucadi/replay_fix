# Spec 110: Vertical Slice v1.1.0 — Real MetaPulse 20-Bug Benchmark Suite, Authenticated Meta AI HTTPS Gateway & Dual-Mode Execution

**Feature Identity**: `110-v1.1.0-real-metapulse-muse-integration`  
**Target Release**: `v1.1.0`  
**Baseline Foundation**: `v1.0.0` (`b252835`)  
**Status**: Approved for Implementation  

---

## 1. Executive Summary

**ReplayFix v1.1.0** upgrades the evaluation benchmark and cloud gateway from the 5-case foundation into the complete **20-bug MetaPulse Benchmark Suite (`RF-01` through `RF-20`)** with real `.mp4` cursor recordings (`benchmarks/metapulse/videos/`), an immutable cryptographic `benchmarks/metapulse/MANIFEST.json`, a production HTTPS `MetaGatewayClient` (`reqwest` + `rustls-tls`), and explicit dual execution modes (`--mode live-ai` vs. `--mode deterministic-fixture`).

---

## 2. User Stories & Priorities

### User Story 1 — Real MetaPulse 20-Bug Benchmark Suite & Immutable Manifest (Priority: P1) 🎯 MVP
As a benchmark evaluator, I can inspect and execute ReplayFix against the 20 real MetaPulse `.mp4` defect recordings (`RF-01` through `RF-20`), verified against `benchmarks/metapulse/MANIFEST.json` (recording filename, byte size, SHA-256 digest, bug ID, title, and `bug/RF-XX` branch) and native git branch checkout/clone support for `https://github.com/aroucadi/MetaPulse.git`.

### User Story 2 — Production HTTPS `MetaGatewayClient` & `<UNTRUSTED_DOM_CONTEXT>` Fencing (Priority: P2)
As a security and AI integration engineer, I can dispatch outbound authenticated HTTPS requests via `MetaGatewayClient` (`reqwest` with `rustls-tls`, 30s connect / 90s read timeouts, exponential backoff on HTTP 429/503, Bearer token via `META_API_KEY` or `REPLAYFIX_AI_KEY`, and `<UNTRUSTED_DOM_CONTEXT>` XML escaping) across `sam-3.1`, `muse-spark-1.3` (fallback `meta-llama/Llama-3.2-11B-Vision-Instruct`), `muse-wonder-1.0` (fallback `meta-llama/Llama-3.3-70B-Instruct`), and `muse-voice-transcribe-1.0`.

### User Story 3 — Honest Dual Execution Modes (`--mode live-ai` vs. `--mode deterministic-fixture`) (Priority: P3)
As a CLI operator, I can pass `--mode <live-ai | deterministic-fixture>`, `--model-intent <MODEL_ID>`, and `--model-repair <MODEL_ID>` to `replayfix run`. In `--mode live-ai`, missing `META_API_KEY` / `REPLAYFIX_AI_KEY` fails closed immediately with Exit Code `30` (`ERR_GATEWAY_AUTH_MISSING`). In `--mode deterministic-fixture`, pre-recorded calibrated fixtures from `benchmarks/fixtures/` are used and `audit_receipt.json` explicitly records `provenance: "deterministic-fixture"`.

### User Story 4 — Option B Truth Demonstration Video & `release/v1.1.0/` Package (Priority: P4)
As a hackathon judge, I can watch an authentic demonstration video (`release/v1.1.0/demo/replayfix-v1.1.0-demo.mp4` and updated `release/v1.0.0/demo/replayfix-v1.0.0-demo.mp4`) built from real MetaPulse `RF-01_quote_repost_crash_with_cursor.mp4` frames, showing FFmpeg keyframe extraction, fail-closed `AMBIGUOUS_GROUNDING` (Exit `22`), anti-cheat `TEST_MUTATION_PROHIBITED` (Exit `11`), deterministic-fixture `Gate 1 RED -> Repair -> Gate 2 GREEN` (Exit `0`), and offline verification in `proof_viewer.html`.

---

## 3. Normative Functional Requirements & Fail-Closed Exit Codes

- **REQ-110-01**: `benchmarks/metapulse/MANIFEST.json` MUST catalog all 20 `.mp4` videos (`RF-01`..`RF-20`) with exact byte sizes, SHA-256 digests, titles, and target branches (`bug/RF-01`..`bug/RF-20`).
- **REQ-110-02**: `MetaPulseRepoRunner` in `replayfix-core` MUST support cloning or checking out `https://github.com/aroucadi/MetaPulse.git` at `bug/RF-01`..`bug/RF-20` into an ephemeral scratch sandbox while preserving $\Delta	ext{repo}=0$.
- **REQ-110-03**: `MetaGatewayClient` MUST use `reqwest` (`rustls-tls`, `json`, `blocking`) with 30s connect / 90s read timeouts, `META_API_KEY` / `REPLAYFIX_AI_KEY` Bearer auth, HTTP 429/503 retry with exponential backoff, and `<UNTRUSTED_DOM_CONTEXT>` XML escaping.
- **REQ-110-04**: `replayfix run --mode live-ai` without `META_API_KEY` or `REPLAYFIX_AI_KEY` MUST exit with code `30` (`ERR_GATEWAY_AUTH_MISSING`) and print actionable remediation instructions.
- **REQ-110-05**: `audit_receipt.json` MUST include `provenance` (`"live-ai"` or `"deterministic-fixture"`) and `model_provenance` (`model_intent`, `model_repair`).
