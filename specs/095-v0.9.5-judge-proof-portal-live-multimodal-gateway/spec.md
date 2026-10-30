# 📋 Spec 095: Vertical Slice v0.9.5 — Judge Proof Portal, Cryptographic Evidence Verification, Live Multimodal Cloud Gateway & Bounded Tri-Model Orchestration

**Feature Identity**: `095-v0.9.5-judge-proof-portal-live-multimodal-gateway`  
**Target Release**: `v0.9.5`  
**Git Branch**: `main`  
**Constitutional Invariants & Governing Principles**:
- **Law I (Substrate Primacy & Fail-Closed Authority)**: The deterministic Rust harness is the sole verification authority. No LLM response, UI badge, or model consensus can certify a repair.
- **Law II (Base Worktree Immutability)**: The target repository remains strictly read-only ($\Delta\text{repo}=0$) across all single runs and sweeps.
- **Law III (Cryptographic Non-Repudiation)**: Every receipt is bound by RFC 6962 Merkle tree hashing and signed via Pure Ed25519 (RFC 8032). The portal must independently compute hashes and verify signatures using native Web Crypto / client-side primitives rather than trusting pre-computed status fields.
- **Law IV (Honest Provenance & Verification States)**: Never display "VERIFIED GENUINE" unless schema, canonicalization, Merkle root, inclusion proofs, signature, signer trust, and on-disk artifact integrity all pass. Clearly distinguish live provider results from fixtures and mock results.
- **Law V (Air-Gapped / Offline Portal Integrity)**: The Judge Proof Portal (`proof_viewer.html`) is a 100% self-contained single-file application requiring zero external network calls, zero CDN dependencies, and zero backend services for offline receipt verification.
- **Law VI (Zero-Audio Rule & Media Safety)**: Audio payloads are strictly prohibited on text-only reasoning and candidate generation tasks; audio is processed only by verified transcription endpoints with bounded duration.
- **Law VII (Bounded Execution & Circuit Breaking)**: Cloud gateway operations are subject to explicit per-request timeouts, token-bucket rate limits, HTTP 429 backoff with jitter, and a three-state circuit breaker (Closed, Open, Half-Open).

---

## 1. Executive Summary & Value Proposition

ReplayFix v0.9.1 established an independently qualified, 4-layer zero-trust verification engine with strict RFC 6962 inclusion proofs, RFC 8785 canonical JSON, pure Ed25519 signing, and benchmark isolation across the five MetaPulse defect scenarios (`RF-01` to `RF-05`).

**Spec 095 (v0.9.5)** delivers the visual and operational centerpiece for hackathon judges, auditors, and systems engineers:
1. **Self-Contained Offline Judge Proof Portal (`proof_viewer.html`)**: A zero-dependency, single-file HTML/CSS/JS application that loads `audit_receipt.json` and `sweep_receipt.json`, executes full client-side 4-layer cryptographic verification, renders an interactive RFC 6962 Merkle tree with live node math, displays multimodal video/DOM/SAM evidence, and previews syntax-highlighted unified diffs with active regression parity.
2. **Live Multimodal Cloud Gateway (`replayfix-core::gateway`)**: A bounded, observable API client interfacing with Meta's official model endpoints (`muse-wonder-1.0`, `muse-spark-1.3`, `sam-3-1`, and `muse-voice-transcribe-1.0`) with rate pooling, circuit breakers, fallback adapters (`meta-models/muse-spark-open`), and request/response accounting.
3. **End-to-End Release Qualification**: Comprehensive adversarial test suites covering hostile JSON, XSS injection, proof tampering, rate exhaustion, and fallback resilience.

---

## 2. User Scenarios & Acceptance Criteria

### User Story 1 (US1) - Offline Cryptographic Audit & Merkle Proof Exploration (Priority: P1)
As a hackathon judge or compliance auditor, I want to load an `audit_receipt.json` or `sweep_receipt.json` into an offline browser application, so that I can mathematically verify the repair's authenticity and inspect the Merkle tree leaves without installing developer toolchains or trusting a server.

**Why this priority**: The portal is the primary judge-facing evaluation surface for Track 1. Independent mathematical auditability without network dependencies proves that ReplayFix's claims are real and untampered.

**Independent Test**: Open `proof_viewer.html` in an air-gapped browser (`file://`), drop an `audit_receipt.json`, and confirm that the client-side Web Crypto engine recalculates all leaf hashes, internal nodes, Merkle root, and Ed25519 signature, displaying exact cryptographic status without external requests.

**Acceptance Scenarios**:
1. **Given** a valid, signed `audit_receipt.json` and its sibling `merkle_receipt.json`, **When** dropped into `proof_viewer.html`, **Then** the portal reports `VERIFIED GENUINE` (or `SIGNATURE VALID — SIGNER UNTRUSTED` if the key is not pinned), displays total leaves, root hash, and signing timestamp.
2. **Given** a receipt whose Merkle root or leaf hash has been altered by 1 bit, **When** imported into the portal, **Then** the portal reports `INTEGRITY CHECK FAILED`, highlights the invalid leaf in red, and presents the recomputed vs. expected hash mismatch.
3. **Given** an interactive tree visualization, **When** a user clicks on an individual leaf (e.g., `01_candidate.diff`), **Then** the inspector displays the leaf metadata, canonical bytes, SHA-256 leaf hash ($0x00 \parallel B$), sibling hashes, and step-by-step parent derivations up to the root.

---

### User Story 2 (US2) - Five-Bug Benchmark Sweep Visualization (Priority: P2)
As a benchmark evaluator, I want to inspect the results of the complete `RF-01` through `RF-05` benchmark sweep in the portal, so that I can see the pass/fail breakdown, execution latencies, and aggregate cryptographic attestation across all five cases.

**Why this priority**: Proves multi-bug generalization and performance claims ($<60$s execution SLA) across distinct bug classes.

**Independent Test**: Drop `.replayfix/sweep_receipt.json` into `proof_viewer.html` and verify that all 5 benchmark cards display correct statuses, durations, patch hashes, and the sweep Merkle root.

**Acceptance Scenarios**:
1. **Given** a valid `sweep_receipt.json`, **When** imported, **Then** the portal switches to Sweep View, showing a 5-card grid (`RF-01` to `RF-05`) with green checks for verified repairs, target file paths, patch SHA-256 digests, and wall-clock execution time.
2. **Given** a partial sweep where one bug failed verification, **When** imported, **Then** the portal reports `SWEEP_PARTIAL_FAILURE`, marks the failing bug card with its triage status code, and preserves individual receipts.

---

### User Story 3 (US3) - Multimodal Spatial & Video Evidence Inspection (Priority: P3)
As an autonomous systems reviewer, I want to inspect visual recordings, keyframes, DOM snapshots, and SAM 3.1 segmentation overlays, so that I can verify that ReplayFix grounded the defect visually and spatially before attempting repair.

**Why this priority**: Grounding and perception are key innovations of ReplayFix; demonstrating SAM 3.1 segmentation and click coordinate snapping proves multi-modal competence.

**Independent Test**: Load a receipt with video telemetry into `proof_viewer.html`, select keyframes on a timeline scrubber, and verify that the canvas/SVG overlay displays the bounding box, polygon, and click centroid coordinates.

**Acceptance Scenarios**:
1. **Given** keyframe images and SAM segmentation metadata in the receipt, **When** navigating the timeline, **Then** the portal displays the corresponding frame, overlays the SAM segmentation mask/polygon with high-contrast borders, and pinpoints the $(x, y)$ click centroid.
2. **Given** missing video or keyframe assets, **When** viewed in the portal, **Then** the portal displays an explicit "Artifact Not Provided / Missing" placeholder without crashing or inventing synthetic pixels.

---

### User Story 4 (US4) - Unified Diff & Active Regression Parity Inspection (Priority: P4)
As a lead software architect, I want to review the exact unified diff synthesized by the model and verify that pre-existing test suites maintained 100% pass parity, so that I can confirm no regressions were introduced.

**Why this priority**: Guarantees causal code quality and enforces Law VIII (Regression Parity).

**Independent Test**: Inspect the Diff & Regression tab in `proof_viewer.html` for a repaired case and verify syntax-highlighted additions/deletions and the empty diff set $F_{\text{patched}} \setminus F_{\text{baseline}} = \emptyset$.

**Acceptance Scenarios**:
1. **Given** a verified repair receipt, **When** selecting the Diff view, **Then** the portal renders a syntax-highlighted side-by-side or unified diff (+green additions, -red deletions) with line numbers and file headers.
2. **Given** regression parity telemetry in the receipt, **When** inspecting the Regression panel, **Then** the portal displays total test suites, test cases, 0 new failures, 0 added skips, and confirms that `package.json` test scripts remained frozen.

---

### User Story 5 (US5) - Bounded Live Meta Multimodal Cloud Gateway (Priority: P5)
As a developer running ReplayFix in connected mode, I want the harness to interface reliably with Meta model endpoints (`muse-wonder-1.0`, `muse-spark-1.3`, `sam-3-1`, `muse-voice-transcribe-1.0`), so that autonomous repair, spatial segmentation, and audio-visual correlation can leverage live frontier models under bounded quotas.

**Why this priority**: Completes the live cloud integration track while maintaining strict fail-closed safety, circuit breakers, and rate controls.

**Independent Test**: Run the gateway against a simulated or live endpoint and verify that requests carry correct auth headers, model slugs, token-bucket rate limits, and circuit breaker trip/reset states.

**Acceptance Scenarios**:
1. **Given** a valid API key, **When** dispatching repair candidate synthesis, **Then** the gateway routes to `muse-spark-1.3` or `muse-wonder-1.0` at `https://api.meta.ai/v1/responses`, recording token consumption and latency.
2. **Given** HTTP 429 or network errors exceeding the threshold, **When** the error rate trips the circuit breaker, **Then** the gateway opens the circuit, transitions to half-open after cooldown, or routes to an approved fallback endpoint (`meta-models/muse-spark-open`) without hanging.
3. **Given** a text-only task, **When** constructing the payload, **Then** the Zero-Audio Rule ensures no audio attachments or audio processing tokens are billed.

---

## 3. Functional Requirements

### 3.1 Offline Judge Proof Portal (`proof_viewer.html`)
- **REQ-095-01**: The portal MUST be packaged as a single, self-contained HTML file (`proof_viewer.html`) containing all HTML, CSS, and JavaScript, with zero external network requests, fonts, or scripts.
- **REQ-095-02**: The portal MUST support drag-and-drop and standard file input for `audit_receipt.json`, `sweep_receipt.json`, and `.zip` archives containing evidence artifacts.
- **REQ-095-03**: The portal MUST enforce strict input validation: reject inputs $>50\text{MB}$, bound string lengths, sanitize all text rendered into the DOM to prevent XSS/HTML injection, and avoid `eval()` or dynamic script execution.
- **REQ-095-04**: The portal MUST implement RFC 8785 canonical JSON sorting (lexicographical UTF-16 code units) and RFC 6962 Merkle tree hashing ($0x00$ leaf prefix, $0x01$ node prefix) using the browser's native `SubtleCrypto` API (SHA-256).
- **REQ-095-05**: The portal MUST implement pure Ed25519 signature verification (via pure JS or WebCrypto where supported) against the canonical signing payload `REPLAYFIX-ATTESTATION:v0.9:<schema>:<root>:<size>:<timestamp>`.
- **REQ-095-06**: The portal MUST display distinct, mutually exclusive honest verification states:
  - `VERIFIED GENUINE` (all cryptographic, Merkle, and pinned-key checks pass)
  - `SIGNATURE VALID — SIGNER UNTRUSTED` (valid Ed25519 signature, unpinned key)
  - `INTEGRITY CHECK FAILED` (Merkle root or leaf hash mismatch)
  - `UNSIGNED RECEIPT` (receipt lacks attestation)
  - `MALFORMED RECEIPT` (JSON parsing or schema validation error)
- **REQ-095-07**: The portal MUST provide an interactive Merkle tree explorer showing all evidence leaves, leaf hashes, sibling hashes, and step-by-step parent derivations up to the root.
- **REQ-095-08**: The portal MUST render syntax-highlighted unified diffs, changed file paths, and formal regression parity status ($F_{\text{patched}} \setminus F_{\text{baseline}} = \emptyset$).
- **REQ-095-09**: The portal MUST render multimodal evidence: video keyframes, bounding box overlays, SAM segmentation polygons, click coordinate centroids, and explicit placeholders for missing artifacts.
- **REQ-095-10**: The portal MUST support sweep receipts, displaying all 5 benchmark cases (`RF-01` through `RF-05`), individual repair statuses, patch hashes, execution timings, and the sweep Merkle root.

### 3.2 Live Multimodal Cloud Gateway (`crates/replayfix-core/src/gateway.rs`)
- **REQ-095-11**: The gateway MUST support Meta Model API endpoints (`https://api.meta.ai/v1/responses`) and model slugs (`muse-wonder-1.0`, `muse-spark-1.3`, `sam-3-1`, `muse-voice-transcribe-1.0`).
- **REQ-095-12**: The gateway MUST implement token-bucket rate limiting with configurable requests-per-minute (RPM) and tokens-per-minute (TPM) ceilings, honoring HTTP 429 `Retry-After` headers.
- **REQ-095-13**: The gateway MUST implement a 3-state circuit breaker (`Closed`, `Open`, `Half-Open`) with configurable consecutive failure threshold (default: 3) and cooldown duration (default: 30s).
- **REQ-095-14**: The gateway MUST support approved fallback endpoints (e.g., `https://inference.baseten.co/v1` for `meta-models/muse-spark-open`) when the primary provider circuit is open.
- **REQ-095-15**: The gateway MUST enforce the Zero-Audio Rule: text-only repair requests MUST NOT attach audio data or invoke speech processing.
- **REQ-095-16**: The gateway MUST preserve live vs. mock provenance, recording provider identity, request latency, and token consumption in telemetry artifacts.

---

## 4. Key Entities & Data Models

- **`AuditReceipt` (Schema 0.9)**:
  - `schema_version`: String (`"0.9"`).
  - `status`: String (`"VERIFIED_REPAIRED"`, `"QUARANTINE_FAILED"`, etc.).
  - `task_id`: String (e.g., `"RF-01"`).
  - `contract_id`: String.
  - `contract_sha256`: String (64-char hex).
  - `gate1`, `patch`, `gate2`: Structured verification records.
  - `merkle_root`: Optional string (64-char hex).
  - `attestation`: Optional `AttestationRecord`.
  - `created_at`: RFC 3339 timestamp.
- **`SweepReceipt` (Schema 0.9)**:
  - `schema_version`: String (`"0.9"`).
  - `total_bugs`, `repaired_bugs`, `failed_bugs`: Integers.
  - `results`: Map of bug ID to `SweepBugResult`.
  - `merkle_root`: String (64-char hex).
  - `ed25519_attestation`: `AttestationRecord`.
  - `elapsed_wall_clock_ms`: Integer.
  - `created_at`: RFC 3339 timestamp.
- **`EvidenceLeaf`**:
  - `artifact_type`: String (`"candidate_patch"`, `"syntax_preflight"`, etc.).
  - `file_name`: String (e.g., `"01_candidate.diff"`).
  - `content_sha256`: String (64-char hex).
  - `size_bytes`: Integer.
  - `timestamp`: RFC 3339 timestamp.
- **`GatewayRequestConfig`**:
  - `model_slug`: String.
  - `endpoint_url`: String.
  - `timeout_ms`: Integer.
  - `max_retries`: Integer.
  - `fallback_endpoint`: Optional string.

---

## 5. Success Criteria & Verification Gates

- **SC-095-01**: Single-file deliverable `proof_viewer.html` loads locally in Chrome, Firefox, and Safari via `file://` with 0 external network requests, 0 console errors, and full offline verification capability.
- **SC-095-02**: Client-side Merkle verification in `proof_viewer.html` produces bit-for-bit identical roots and verification verdicts as `crates/replayfix-core/src/merkle.rs` across all test vectors.
- **SC-095-03**: 100% of tampered leaves, modified roots, and untrusted signatures are detected and flagged with appropriate error badges in the portal.
- **SC-095-04**: The live cloud gateway handles simulated rate exhaustion (HTTP 429) and network outages with circuit-breaker tripping, cooldown, and seamless fallback.
- **SC-095-05**: 100% of workspace tests pass (`cargo test --workspace`), 0 non-test panics (`scripts/check-panics.ps1`), 0 Clippy warnings (`-D warnings`), and clean formatting (`cargo fmt`).
