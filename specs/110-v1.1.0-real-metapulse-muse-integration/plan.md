# Implementation Plan: Spec 110 — Real MetaPulse 20-Bug Benchmark Suite, Authenticated Meta AI HTTPS Gateway & Dual-Mode Execution

**Branch**: `main` (`110-v1.1.0-real-metapulse-muse-integration`) | **Date**: 2026-10-30 | **Spec**: [spec.md](spec.md)

---

## 1. Technical Architecture & Dependency Updates

1. **Dependencies (`crates/replayfix-core/Cargo.toml`)**:
   - Add `reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls", "blocking"] }`.
   - Zero OpenSSL C-library dependencies; 100% pure-Rust TLS (`rustls-tls`) for deterministic Windows MSVC builds.
2. **Data Flow**:
   - `CLI (replayfix run --video ... --repo ... --mode <live-ai|deterministic-fixture> --model-intent ... --model-repair ...)`
   - $ightarrow$ `ExecutionMode` validation (`live-ai` checks `META_API_KEY` / `REPLAYFIX_AI_KEY`; if missing $ightarrow$ Exit Code `30` `ERR_GATEWAY_AUTH_MISSING`)
   - $ightarrow$ `MetaPulseManifest::load_and_verify` (`benchmarks/metapulse/MANIFEST.json`)
   - $ightarrow$ `MetaGatewayClient` (30s connect / 90s read timeout, `<UNTRUSTED_DOM_CONTEXT>` XML escaping, HTTP 429/503 exponential backoff) or calibrated `benchmarks/fixtures/` loader
   - $ightarrow$ `AuditReceipt` (`provenance` + `model_provenance` committed into RFC 6962 Merkle tree and signed with Ed25519).
