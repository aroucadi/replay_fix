# Tasks: Spec 110 — Real MetaPulse Benchmark Suite, Authenticated Meta AI Gateway & Dual Execution Modes (v1.1.0)

## Phase 1: SpecKit 110 & Phase A MetaPulse Benchmark Migration
- [x] T001 Establish `specs/110-v1.1.0-real-metapulse-muse-integration/` (`spec.md`, `plan.md`, `research.md`, `contracts/`, `tasks.md`)
- [x] T002 Copy 20 MetaPulse `.mp4` videos (`RF-01`..`RF-20`) into `benchmarks/metapulse/videos/` and `BENCHMARK_METADATA.md` into `benchmarks/metapulse/`
- [x] T003 Generate immutable `benchmarks/metapulse/MANIFEST.json` with SHA-256, byte size, bug ID, title, and `bug/RF-XX` branch for all 20 cases
- [x] T004 Create calibrated deterministic fixtures in `benchmarks/fixtures/` and secondary toy-cart archive in `tests/fixtures/toy_cart/`
- [x] T005 Implement MetaPulse 20-bug catalog, `MANIFEST.json` verifier, and `MetaPulseRepoRunner` (`https://github.com/aroucadi/MetaPulse.git` branch checkout/clone) in `crates/replayfix-core/src/fixtures.rs`

## Phase 2: Phase B Production HTTPS Transport (`gateway.rs`)
- [x] T006 Add `reqwest` (`json`, `rustls-tls`, `blocking`, `default-features = false`) to `crates/replayfix-core/Cargo.toml`
- [x] T007 Implement `MetaGatewayClient` in `crates/replayfix-core/src/gateway.rs` with 30s connect / 90s read timeouts, `META_API_KEY` / `REPLAYFIX_AI_KEY` Bearer auth, HTTP 429/503 retry with exponential backoff, `<UNTRUSTED_DOM_CONTEXT>` XML escaping, and fallback models (`meta-llama/Llama-3.2-11B-Vision-Instruct`, `meta-llama/Llama-3.3-70B-Instruct`)

## Phase 3: Phase C Dual Execution Modes & CLI Integration
- [x] T008 Add `provenance` and `model_provenance` (`ModelProvenanceRecord`) to `AuditReceipt` in `crates/replayfix-core/src/receipt.rs`
- [x] T009 Expose `--mode <live-ai | deterministic-fixture>`, `--model-intent <MODEL_ID>`, and `--model-repair <MODEL_ID>` in `crates/replayfix-cli/src/main.rs` with Exit Code `30` (`ERR_GATEWAY_AUTH_MISSING`) when `live-ai` is invoked without `META_API_KEY` / `REPLAYFIX_AI_KEY`

## Phase 4: Phase D Option B Demonstration Video & Release v1.1.0 Packaging
- [x] T010 Execute `replayfix` against real MetaPulse videos (`RF-01`, `RF-02`, `RF-03`) and generate the authentic Option B demonstration screencast video using real `RF-01_quote_repost_crash_with_cursor.mp4` frames in `release/v1.1.0/demo/` and `release/v1.0.0/demo/`
- [x] T011 Bump workspace version to `1.1.0`, build & Authenticode-sign `target/release/replayfix.exe`, package `release/v1.1.0/` (`SHA256SUMS.txt`, `proof_viewer.html`), update `CHANGELOG.md` and `docs/release-notes/v1.1.0.md`, and pass all quality gates
