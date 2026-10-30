# Tasks: Spec 095 — Judge Proof Portal, Cryptographic Evidence Verification, Live Multimodal Cloud Gateway & Bounded Tri-Model Orchestration

**Input**: Design documents from `/specs/095-v0.9.5-judge-proof-portal-live-multimodal-gateway/`  
**Prerequisites**: `plan.md`, `spec.md`, `research.md`, `data-model.md`, `contracts/`, `adrs/`, `security/`

---

## Phase 1: Setup & Baseline Verification

**Purpose**: Establish Spec 095 workspace module wiring and verify inherited v0.9.1 guarantees remain 100% green.

- [X] T001 Register `gateway` and `portal` modules in `crates/replayfix-core/src/lib.rs`
- [X] T002 [P] Create foundational portal verification and bundle data structures in `crates/replayfix-core/src/portal.rs`

---

## Phase 2: Foundational — Cryptographic Presets & Portal Verification Oracle

**Purpose**: Generate deterministic, cryptographically signed benchmark receipts (`RF-01` single audit + Merkle receipt, `RF-01..RF-05` sweep receipt, and tampered adversarial receipt) and Rust-side portal verification helpers that validate `proof_viewer.html` structure and vectors.

- [X] T003 Implement `PortalBundle` and deterministic preset generator in `crates/replayfix-core/src/portal.rs`
- [X] T004 Implement static security and cryptographic vector validator for `proof_viewer.html` in `crates/replayfix-core/src/portal.rs`

---

## Phase 3: User Story 1 — Offline Cryptographic Audit & Merkle Proof Exploration (Priority: P1) 🎯 MVP

**Goal**: Deliver the self-contained single-file `proof_viewer.html` with strict offline CSP, safe drag-and-drop file import, RFC 8785 UTF-16 canonical JSON serializer, RFC 6962 SHA-256 leaf (`0x00`) and internal node (`0x01`) Merkle verifier, pure RFC 8032 Ed25519 signature verifier, 6 honest verification states, and interactive Merkle tree explorer.

**Independent Test**: Load `proof_viewer.html` offline and verify `VERIFIED GENUINE`, `SIGNATURE VALID — SIGNER UNTRUSTED`, `INTEGRITY CHECK FAILED`, `UNSIGNED RECEIPT`, and `MALFORMED RECEIPT` states across valid and tampered receipts.

- [X] T005 [US1] Implement `proof_viewer.html` core HTML5 layout, strict CSP meta header, CSS design system, and accessible keyboard navigation in `proof_viewer.html`
- [X] T006 [US1] Implement bounded drag-and-drop / file importer ($\le 50	ext{MB}$, depth $\le 32$, prototype-pollution stripper, `textContent`-only DOM renderer) in `proof_viewer.html`
- [X] T007 [US1] Implement client-side RFC 8785 UTF-16 canonical JSON, RFC 6962 Merkle tree / inclusion-proof verifier, and pure RFC 8032 Ed25519 signature verifier in `proof_viewer.html`
- [X] T008 [US1] Implement interactive Merkle Tree Explorer and Attestation & Trust Store inspection panels in `proof_viewer.html`
- [X] T009 [US1] Add unit and adversarial security tests for US1 portal cryptographic verification, CSP, and XSS immunity in `crates/replayfix-core/src/portal.rs`

---

## Phase 4: User Story 2 — Five-Bug Benchmark Sweep Visualization (Priority: P2)

**Goal**: Render `sweep_receipt.json` in `proof_viewer.html` with a 5-card grid (`RF-01` through `RF-05`), individual status badges, patch SHA-256 digests, wall-clock execution latency, partial-failure detection (`SWEEP_PARTIAL_FAILURE`), and sweep Merkle root verification.

**Independent Test**: Load `.replayfix/sweep_receipt.json` (and partial-failure sweep receipts) into `proof_viewer.html` and verify all 5 bug cards and aggregate attestation.

- [X] T010 [US2] Implement 5-Bug Benchmark Sweep dashboard grid, partial-sweep status detector, and per-bug drilldown in `proof_viewer.html`
- [X] T011 [US2] Add unit tests for complete and partial sweep receipt rendering and verification in `crates/replayfix-core/src/portal.rs`

---

## Phase 5: User Story 3 — Multimodal Spatial & Video Evidence Inspection (Priority: P3)

**Goal**: Render video keyframe timeline, $120	imes120	ext{px}$ optical-flow ROI boxes, SAM 3.1 `<|box;...|>` segmentation overlays, click centroid $(C_x, C_y)$ markers, DOM `ariaSnapshot` grounding scores, ASR transcript correlation, and explicit missing-artifact banners in `proof_viewer.html`.

**Independent Test**: Inspect the Visual & Multimodal Evidence panel in `proof_viewer.html` across valid SAM/ASR telemetry and missing-media states.

- [X] T012 [US3] Implement Multimodal Evidence timeline scrubber, calibrated SVG/Canvas SAM 3.1 bounding-box & centroid overlay, ASR transcript alignment table, and missing-artifact banner in `proof_viewer.html`
- [X] T013 [US3] Add coordinate transformation bounds and malformed SAM mask negative tests in `crates/replayfix-core/src/portal.rs`

---

## Phase 6: User Story 4 — Unified Diff & Active Regression Parity Inspection (Priority: P4)

**Goal**: Render syntax-highlighted unified diffs (+green, -red, `@@` hunks) and formal regression parity ($F_{	ext{patched}} \setminus F_{	ext{baseline}} = \emptyset$, `TestFreezeGuard`, `DiffGuard` status) in `proof_viewer.html`.

**Independent Test**: Inspect the Unified Diff & Regression Parity panel in `proof_viewer.html` and confirm line-by-line diff highlighting and regression parity verification.

- [X] T014 [US4] Implement safe text-based syntax-highlighted Unified Diff viewer and Active Regression Parity ($F_{	ext{patched}} \setminus F_{	ext{baseline}} = \emptyset$) panel in `proof_viewer.html`
- [X] T015 [US4] Add diff rendering and regression-parity verification tests in `crates/replayfix-core/src/portal.rs`

---

## Phase 7: User Story 5 — Bounded Live Meta Multimodal Cloud Gateway (Priority: P5)

**Goal**: Implement `crates/replayfix-core/src/gateway.rs` supporting Meta Model API (`https://api.meta.ai/v1/responses` for `muse-wonder-1.0`, `muse-spark-1.3`, `sam-3-1`, `muse-voice-transcribe-1.0`), Baseten fallback (`https://inference.baseten.co/v1` `meta-models/muse-spark-open`), token-bucket `RatePool`, 3-state `CircuitBreaker`, Zero-Audio Rule, WAV header verification, and secret redaction.

**Independent Test**: Run `cargo test --package replayfix-core gateway` and CLI integration tests verifying rate limits, HTTP 429 `Retry-After`, circuit breaker transitions, fallback routing, Zero-Audio Rule enforcement, and credential redaction.

- [X] T016 [US5] Implement `GatewayProvider`, `ModelRole`, `GatewayRequest`, `GatewayResponse`, and redacted `GatewayConfig` in `crates/replayfix-core/src/gateway.rs`
- [X] T017 [US5] Implement token-bucket `RatePool` (RPM/TPM, `max_in_flight`, HTTP 429 `Retry-After` and exponential backoff with jitter) in `crates/replayfix-core/src/gateway.rs`
- [X] T018 [US5] Implement 3-state `CircuitBreaker` (`Closed`, `Open`, `HalfOpen`) and automatic `BasetenFallback` routing in `crates/replayfix-core/src/gateway.rs`
- [X] T019 [US5] Implement Zero-Audio Rule guard, 24 kHz mono WAV metadata validator, and temporal transcript-to-keyframe alignment in `crates/replayfix-core/src/gateway.rs`
- [X] T020 [US5] Add comprehensive unit and adversarial gateway tests (rate limits, 429 backoff, circuit breaker, fallback, zero-audio, credential redaction) in `crates/replayfix-core/src/gateway.rs`
- [X] T021 [US5] Wire `export-proof` CLI subcommand in `crates/replayfix-cli/src/main.rs` and add end-to-end CLI integration tests in `crates/replayfix-cli/tests/cli.rs`

---

## Phase 8: Polish, Release Qualification & Documentation

**Purpose**: Execute full quality gates, update traceability matrix, bump version to `0.9.5`, update `CHANGELOG.md`, `docs/release-notes/v0.9.5.md`, and `docs/agent-state.md`, and perform independent cold review.

- [X] T022 Update `specs/095-v0.9.5-judge-proof-portal-live-multimodal-gateway/verification/requirement-traceability.md` with final verified status
- [X] T023 Bump workspace crate versions to `0.9.5` in `crates/replayfix-core/Cargo.toml` and `crates/replayfix-cli/Cargo.toml`
- [X] T024 Update `CHANGELOG.md`, `docs/release-notes/v0.9.5.md`, and `docs/agent-state.md` for `v0.9.5`
- [X] T025 Run mandatory quality gates (`scripts/check-panics.ps1`, `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace`, `git diff --check`)

---

## Dependencies & Execution Order

- **Phase 1 & 2 (Setup & Foundational)**: Must complete first (`T001`–`T004`).
- **Phase 3–6 (US1–US4 Portal Deliverables)**: Build incrementally inside `proof_viewer.html` and `crates/replayfix-core/src/portal.rs` (`T005`–`T015`).
- **Phase 7 (US5 Cloud Gateway & CLI)**: Can be built in `crates/replayfix-core/src/gateway.rs` and `crates/replayfix-cli/` (`T016`–`T021`).
- **Phase 8 (Release Qualification)**: Runs after all user stories are implemented and tested (`T022`–`T025`).
