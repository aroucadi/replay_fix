# ReplayFix v1.0.0 — Security & Cryptographic Trust Architecture

## 1. Four-Layer Zero-Trust Cryptographic Receipt Verification
Every terminal repair verdict emits disaggregated evidence artifacts bound into a 4-layer cryptographic receipt:
1. **Layer 1 (On-Disk Artifact Integrity)**: `verify-receipt` re-reads physical files in `artifacts/` (`01_candidate.diff`, `02_preflight.json`, `03_differential_diagnosis.json`, `04_regression_parity.json`, `04_audit_receipt.json`), verifying exact `size_bytes` and `content_sha256` against RFC 8785 (UTF-16 sorted) canonical JSON leaves.
2. **Layer 2 (RFC 6962 Merkle Tree & Inclusion Proofs)**: Leaf hashes use `SHA-256(0x00 || B)` and internal nodes use `SHA-256(0x01 || L || R)`, preventing second-preimage attacks.
3. **Layer 3 (RFC 8032 Pure Ed25519 Attestation)**: Signs `REPLAYFIX-ATTESTATION:v0.9:<schema_version>:<merkle_root>:<tree_size>:<created_at>`.
4. **Layer 4 (Signer Trust Store)**: Explicitly separates cryptographic signature validity (`SIGNATURE VALID — SIGNER UNTRUSTED`) from pinned key authorization (`VERIFIED GENUINE`).

## 2. Anti-Cheat & Sandbox Isolation Invariants
- **Dual-Layer $\Delta	ext{repo}=0$ Isolation**: Candidate repairs execute strictly inside an ephemeral `ScratchSandbox`; `WorktreeManifest` verifies both SHA-256 file tree digests and `git status --porcelain`.
- **`TestFreezeGuard` & `DiffGuard`**: Any candidate patch that touches `tests/**`, `*.spec.*`, `*.test.*`, `playwright.config.*`, or `package.json` `"scripts.test*"` is rejected fail-closed with `TEST_MUTATION_PROHIBITED` (Exit Code `11`).
- **Air-Gapped Proof Portal (`proof_viewer.html`)**: Enforces `Content-Security-Policy: default-src 'none'; connect-src 'none'`, renders untrusted strings exclusively via `textContent`, strips prototype-pollution keys, and makes zero network requests.
