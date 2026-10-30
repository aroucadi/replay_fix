# Contract: Client-Side Cryptographic Receipt Verification

## 1. Scope & Standards
This contract governs how `proof_viewer.html` independently verifies the mathematical integrity of ReplayFix receipts (`audit_receipt.json` and `sweep_receipt.json`) without trusting client-side pre-evaluations.

## 2. Verification Pipeline & Honest State Machine

```
[Raw Receipt File]
       │
       ▼
[JSON Parsing & Schema Validation] ──(Fail)──► MALFORMED RECEIPT
       │ (Pass)
       ▼
[RFC 8785 Canonical Serialization] ──(Float/Malformed)──► CANONICALIZATION_FAILED
       │ (Pass)
       ▼
[RFC 6962 Leaf & Node Hashing] ──(Mismatch)──► INTEGRITY CHECK FAILED
       │ (Matches Root)
       ▼
[RFC 8032 Pure Ed25519 Verification] ──(Bad Sig)──► SIGNATURE_VERIFICATION_FAILED
       │ (Valid Sig)
       ▼
[Trust Policy (Pinned Key Match)]
       ├── (Pinned Key Match) ───────────────► VERIFIED GENUINE
       └── (No Pin / Unrecognized Key) ──────► SIGNATURE VALID — SIGNER UNTRUSTED
```

## 3. Byte Hashing Algorithms
- **Leaf Hashing**: $\text{SHA-256}(0x00 \parallel \text{CanonicalBytes}(\text{Leaf}))$
- **Internal Node Hashing**: $\text{SHA-256}(0x01 \parallel \text{LeftNodeBytes} \parallel \text{RightNodeBytes})$
- **Signing Payload Construction**:
  `REPLAYFIX-ATTESTATION:v0.9:<schema_version>:<merkle_root>:<tree_size>:<created_at>`
