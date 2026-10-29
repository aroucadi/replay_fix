# 📋 Spec 090: Vertical Slice v0.9.0 — Cryptographic Merkle Receipts, Ed25519 Attestation, Five-Bug Sweep & Tri-Model Verification

**Feature Identity**: `090-v0.9-cryptographic-merkle-receipts-multimodel-verification`  
**Target Release**: `v0.9.0`  
**Git Branch**: `release/v0.9.0-cryptographic-merkle-receipts-multimodel-verification`  
**Constitutional Invariants**:
- Invariant 1 (Substrate Primacy): Deterministic Rust harness is the sole verification authority; LLM model pooling cannot bypass Gate 1 / Gate 2 verification.
- Invariant 2 (Fail-Closed Verification): Invalid Merkle roots, malformed inclusion proofs, signature mismatches, untrusted signers, or test mutations fail closed.
- Invariant 4 (Repository Isolation): Every bug in the five-bug sweep operates in its own isolated ephemeral scratch clone ($\Delta\text{repo}=0$).
- Invariant 5 (Cryptographic Proof of Provenance): Every repair receipt is bound by an RFC 6962 Merkle tree with domain-separated hashing and signed by an Ed25519 attestation key.
- Invariant 6 (Sub-60s Bounded Latency): The five-bug benchmark sweep executes with bounded tri-model concurrency targeting $\le 60$ seconds total wall-clock execution under offline/mocked providers.

---

## 1. Executive Summary & Value Proposition

ReplayFix v0.8.2 delivered observability, live GitHub issue ingestion, human approval gates, and atomic artifact storage. Milestone v0.9.0 elevates ReplayFix to an enterprise-grade, cryptographically verifiable autonomous repair protocol:
1. **Cryptographic Merkle Receipts**: RFC 6962 compliant Merkle trees over all disaggregated fact artifacts (`candidate.patch`, `preflight.json`, `differential_diagnosis.json`, `regression_parity.json`, `audit_receipt.json`, `approval_audit.json`, `trace.zip`).
2. **Ed25519 Attestation**: Digital signature over the versioned Merkle root and receipt metadata, supported by an independent verification CLI (`replayfix verify-receipt`).
3. **Authoritative 5-Bug Benchmark Suite (`RF-01` - `RF-05`)**: Standardized MetaPulse defect fixtures with complete behavioral contracts, buggy sources, clean patches, and interactive HTML host environments.
4. **Tri-Model Orchestration & Pooling**: Concurrent synthesis across three model candidates (`muse-spark-1.3`, `muse-wonder-1.0`, `muse-spark-1.3-contributor`) with cancellation propagation and first-verified-green acceptance.
5. **High-Performance Benchmark Sweep (`replayfix sweep`)**: Automated execution of `RF-01` through `RF-05` in isolated sandboxes, producing a unified cryptographic sweep receipt in $< 60$ seconds.

---

## 2. User Stories & Acceptance Criteria

### User Story 1 (US1) - RFC 6962 Merkle Receipt & Inclusion Proofs
As an auditor or security engineer, I want ReplayFix to commit all verification facts into an RFC 6962 Merkle tree and emit verifiable inclusion proofs for every artifact, so that I can mathematically prove that no artifact has been altered, reordered, or omitted.

- **Acceptance Criteria**:
  - Emits `.replayfix/artifacts/merkle_receipt.json` containing:
    - `merkle_root`: Hex-encoded SHA-256 root.
    - `tree_size`: Total leaf count.
    - `leaves`: Array of `EvidenceLeaf` with `artifact_type`, `file_name`, `content_sha256`, `size_bytes`, `timestamp`.
    - `inclusion_proofs`: Map of file name to `MerkleInclusionProof` (`leaf_index`, `tree_size`, `audit_path`).
  - Hashing rules:
    - Leaf hashing: $\text{SHA-256}(0x00 \parallel \text{LeafCanonicalBytes})$
    - Internal hashing: $\text{SHA-256}(0x01 \parallel \text{LeftHash} \parallel \text{RightHash})$
  - Inclusion proof verification function returns `true` for valid paths, `false` for tampered leaves, paths, or roots.

### User Story 2 (US2) - Ed25519 Attestation & Independent Verifier CLI
As a consumer of a repair receipt, I want to verify its cryptographic signature and public key against an established trust policy via `replayfix verify-receipt`, so that I know the receipt came from an authorized runner.

- **Acceptance Criteria**:
  - Attestation block in `audit_receipt.json` / `merkle_receipt.json`:
    - `public_key`: 32-byte Ed25519 public key in hex.
    - `signature`: 64-byte Ed25519 signature in hex.
    - `signing_payload`: `REPLAYFIX-ATTESTATION:v0.9:<schema_version>:<merkle_root>:<tree_size>:<timestamp>`.
  - Verifier command: `replayfix verify-receipt <path>`
    - Validates Merkle tree integrity.
    - Validates cryptographic signature.
    - Validates against `--trusted-key <hex>` if supplied.
    - Exits 0 on valid trusted receipt, exits 40 on signature/Merkle verification failure (`RECEIPT_VERIFICATION_FAILED`).

### User Story 3 (US3) - Authoritative Five-Bug Benchmark Suite (`RF-01` - `RF-05`)
As an AI evaluation lead or benchmark judge, I want ReplayFix to support five distinct, self-contained MetaPulse benchmark bugs with behavioral contracts, so that the autonomous repair harness can be rigorously evaluated across varied defect categories.

- **Acceptance Criteria**:
  - `RF-01`: Cart Summary Crash (`cartItemCount` in `src/cart-summary.js`, null dereference).
  - `RF-02`: Stale Discount Calculation (`applyDiscount` in `src/discount.js`, state arithmetic bug).
  - `RF-03`: Inventory Underflow (`decrementQuantity` in `src/inventory.js`, negative quantity boundary).
  - `RF-04`: Cart State Reset on Tab Switch (`switchTab` in `src/cart-store.js`, lifecycle persistence).
  - `RF-05`: Double Submit Idempotency (`submitOrder` in `src/order.js`, async debouncing).
  - Each fixture defines: `target_path`, `buggy_source`, `builtin_diff`, `index_html`, `click_x`, `click_y`, `viewport_width`, `viewport_height`, `expected_output_text`, and `BehavioralContract`.

### User Story 4 (US4) - Bounded Tri-Model Orchestration & First-Verified-Green Pooling
As an automated repair orchestrator, I want ReplayFix to dispatch candidate patch synthesis to three model configurations concurrently and select the first candidate that passes two-gate verification, so that repair latency and token expenditure are minimized.

- **Acceptance Criteria**:
  - Configurable model pool: `muse-spark-1.3`, `muse-wonder-1.0`, `muse-spark-1.3-contributor` (or offline test doubles).
  - Dispatches tasks concurrently with cancellation tokens.
  - When a model returns a candidate, it is evaluated through `node --check` and sandbox verification.
  - Upon qualifying Gate 2 (Green) x2 + Regression Parity, winning patch is committed, and remaining model tasks for that bug are immediately aborted.

### User Story 5 (US5) - High-Performance Five-Bug Benchmark Sweep (`replayfix sweep`)
As a benchmark evaluator, I want to execute `replayfix sweep` across all five bugs and receive an aggregate cryptographic receipt in $< 60$ seconds, so that the entire suite can be validated in a single automated step.

- **Acceptance Criteria**:
  - Command: `replayfix sweep [--bugs RF-01,RF-02,RF-03,RF-04,RF-05] [--mock-heal] [--live-browser]`
  - Runs each bug in an isolated `ScratchSandbox`.
  - Outputs `.replayfix/sweep_receipt.json` with aggregate status, per-bug results, overall Merkle root, and Ed25519 signature.
  - Under `--mock-heal`, total sweep execution completes in $< 60$ seconds (typically $< 10$ seconds).

---

## 3. Exit Codes Taxonomy Aligned for v0.9.0

| Exit Code | Identifier | Description |
|---|---|---|
| 0 | `SUCCESS` | Verification, repair, sweep, or receipt verification succeeded |
| 1 | `DOCTOR_FAILED` | Environment doctor dependency check failed |
| 2 | `USAGE_OR_INPUT_ERROR` | Invalid CLI arguments or malformed flags |
| 10 | `INTERNAL_OR_SYSTEM_ERROR` | Internal engine or I/O failure |
| 11 | `SECURITY_VIOLATION` | Path traversal, DOS device, or test file mutation attempt |
| 12 | `USER_REJECTED_PATCH` | Operator declined candidate patch application in interactive gate |
| 13 | `NON_INTERACTIVE_REFUSAL` | Base repo patch requested without `--yes` in non-interactive session |
| 20 | `BRIDGE_EXECUTION_FAILED` | Playwright or CDP browser process error |
| 21 | `QUARANTINE_REPAIR_FAILED` | Autonomous repair exhausted attempts without passing Gate 2 |
| 22 | `AMBIGUOUS_GROUNDING` | Visual perception element grounding ambiguity |
| 23 | `INVALID_ISSUE_URL` | Malformed GitHub issue URL |
| 24 | `ISSUE_FETCH_FAILED` | GitHub API or network issue retrieval failure |
| 30 | `GATE2_NOT_GREEN` | Repaired code failed verification |
| 31 | `REGRESSION_DETECTED` | Candidate patch caused regression in pre-existing test suite |
| 32 | `QUARANTINE_INVERTED_BASELINE` | Clean reference failed reproduction baseline |
| 40 | `RECEIPT_VERIFICATION_FAILED` | Cryptographic signature or Merkle proof verification failed |
| 41 | `SWEEP_PARTIAL_FAILURE` | One or more benchmark cases in sweep failed verification |
