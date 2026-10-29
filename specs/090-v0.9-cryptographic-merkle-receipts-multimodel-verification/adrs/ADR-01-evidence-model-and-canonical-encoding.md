# ADR-01: Evidence Model and Canonical Encoding

## Status
Accepted

## Context
ReplayFix v0.8.2 outputs disaggregated diagnostic artifacts in `.replayfix/artifacts/` (`candidate.patch`, `01_candidate.diff`, `02_preflight.json`, `differential_diagnosis.json`, `regression_parity.json`, `04_audit_receipt.json`, `approval_audit.json`, `issue_ingestion.json`, `playwright.trace.zip`). However, these artifacts lacked a unified, deterministic cryptographic commitment model that binds all evidence into an immutable, verifiable proof of execution.

To establish verifiable provenance:
1. Every evidence artifact must be deterministically serialized into canonical bytes.
2. Distinct artifact types must have unambiguous domain separation so that different structures cannot collide or produce identical digests.
3. Cryptographic hashing must be stable across platforms, endianness, compilers, and JSON key ordering.

## Decision
1. **RFC 8785 Canonical JSON**:
   All structured JSON evidence is serialized using the repository's RFC 8785 implementation (`replayfix_core::digest::canonical_json`), ensuring:
   - Recursive sorting of object keys in UTF-16 code unit order.
   - Compact encoding without extraneous whitespace.
   - Strict rejection of floating-point numbers (`f32`, `f64`) to prevent cross-platform IEEE 754 precision discrepancies.
   - UTF-8 string encoding.

2. **Evidence Leaf Model (`MerkleLeaf`)**:
   Each disaggregated artifact is represented by a structured evidence leaf:
   ```rust
   pub struct EvidenceLeaf {
       pub artifact_type: String,     // e.g. "CANDIDATE_DIFF", "AUDIT_RECEIPT", "SWEEP_CASE"
       pub file_name: String,         // e.g. "01_candidate.diff", "04_audit_receipt.json"
       pub content_sha256: String,    // SHA-256 of canonical bytes
       pub size_bytes: usize,         // Exact byte length of payload
       pub timestamp: String,         // RFC 3339 timestamp
   }
   ```

3. **Canonical Leaf Byte Format**:
   The canonical representation of a leaf for Merkle inclusion is defined as:
   `canonical_json(&serde_json::to_value(&leaf)?)`

4. **Schema Versioning**:
   The protocol and evidence schema version for this vertical milestone is strictly `"0.9"`.

## Consequences
- **Positive**: Byte-level deterministic evidence encoding that guarantees identical hashes across all platforms.
- **Security**: Disallows floats, preventing precision-based hash divergence attacks.
- **Limitations**: Non-JSON binary artifacts (such as `playwright.trace.zip`) are committed via their raw byte SHA-256 digest within the structured leaf.
