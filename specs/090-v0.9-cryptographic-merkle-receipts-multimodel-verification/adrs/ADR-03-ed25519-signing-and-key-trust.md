# ADR-03: Ed25519 Signing, Key Trust and Attestation

## Status
Accepted

## Context
Verifying the integrity of an execution trace via a Merkle root proves internal consistency, but does not prove who executed the verification or attest to its authenticity. ReplayFix requires digital signatures to seal receipts and link them to an identified verification agent or runner.

## Decision
1. **Cryptographic Algorithm**:
   Pure Ed25519 (RFC 8032) implemented via the vetted `ed25519-dalek` crate.

2. **Versioned Signing Payload**:
   To prevent cross-protocol or cross-version replay attacks, the signing payload is strictly formatted as:
   `REPLAYFIX-ATTESTATION:v0.9:<schema_version>:<merkle_root_hex>:<tree_size>:<created_at>`
   This explicitly binds the signature to the exact Merkle root and tree size.

3. **Key Management & Lifecycle**:
   - Ephemeral Key Generation: In automated CI or offline runs where no explicit key is configured, an ephemeral keypair is generated and the corresponding public key is embedded in the receipt.
   - Pinned Key Support: The harness supports reading an Ed25519 secret key from `REPLAYFIX_SIGNING_KEY` (hex or path to secret key file).
   - Private Key Hygiene: Secret keys are marked `zeroize` on drop and are strictly forbidden from being emitted in logs, receipts, or console output.

4. **Trust Policy**:
   ```rust
   pub enum TrustPolicy {
       /// Any structurally valid signature is accepted (displays untrusted signer warning)
       AllowAny,
       /// Signature must match one of the explicitly pinned public keys
       PinnedKeys(HashSet<String>),
   }
   ```

5. **Verification CLI**:
   `replayfix verify-receipt <path/to/audit_receipt.json> [--trusted-key <hex>]`
   - Reconstructs Merkle root from disaggregated artifacts.
   - Verifies inclusion proofs.
   - Verifies Ed25519 signature over signing payload.
   - Evaluates key against trust policy.

## Consequences
- **Positive**: Cryptographic non-repudiation and tamper detection.
- **Security Boundaries**: Clearly separates signature validity (cryptographic correctness) from trust (policy conformance) and from verification verdict (application truth).
