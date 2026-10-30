# ADR-02: Receipt Parsing and Cryptographic Verification in the Browser

## Status
Accepted

## Context
The offline portal (`proof_viewer.html`) must never trust pre-computed status strings (e.g., `"status": "VERIFIED_REPAIRED"`) inside a JSON file. It must independently recompute canonical bytes, RFC 6962 Merkle hashes, inclusion proofs, and RFC 8032 Ed25519 signatures.

## Decision
1. **RFC 8785 Canonical JSON (UTF-16 Code-Unit Sorting)**:
   - Implements recursive canonical JSON serialization in JS sorting object keys by UTF-16 code units (`a.charCodeAt(i) - b.charCodeAt(i)`), rejecting non-integer floats (`!Number.isSafeInteger`), matching `replayfix_core::digest::canonical_json` bit-for-bit.
2. **RFC 6962 Merkle Hashing & Inclusion Proofs**:
   - Uses Web Crypto `crypto.subtle.digest('SHA-256', ...)` (with a pure-JS SHA-256 fallback for non-secure `file://` contexts where `crypto.subtle` may be restricted in older browsers) with explicit `0x00` leaf prefix and `0x01` internal node prefix.
   - Recomputes the full tree root and verifies each leaf's inclusion proof (`leaf_index`, `tree_size`, largest power of two split $k < n$, and empty trailing path check at `size == 1`).
3. **Pure Ed25519 Verification & 4-Layer Honest Status Taxonomy**:
   - Reconstructs `REPLAYFIX-ATTESTATION:v0.9:<schema_version>:<merkle_root>:<tree_size>:<created_at>` and verifies the 64-byte Ed25519 signature over the 32-byte public key using an embedded, audited RFC 8032 Ed25519 verifier (plus Web Crypto Ed25519 where available).
   -Separates signature validity from signer trust (`PinnedKeys` trust store vs. unpinned key):
     - `VERIFIED GENUINE`: Schema + Canonical JSON + Merkle Root & Proofs + Ed25519 Signature + Trusted Pinned Key + Gate/Regression Policy all pass.
     - `SIGNATURE VALID — SIGNER UNTRUSTED`: Cryptographically valid Merkle tree and Ed25519 signature, but public key is not in the trusted key store.
     - `INTEGRITY CHECK FAILED`: Tampered leaf, mismatched Merkle root, invalid inclusion proof, or invalid Ed25519 signature.
     - `UNSIGNED RECEIPT`: Valid structure/Merkle hashes, but no `attestation` object present.
     - `MALFORMED RECEIPT` / `UNSUPPORTED PROTOCOL VERSION`: Parse error, oversized input, or `schema_version != "0.9"`.

## Consequences
- **Positive**: Complete mathematical parity with `replayfix verify-receipt` and zero possibility of badge forgery.
