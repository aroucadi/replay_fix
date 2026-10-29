# Contract: Evidence Provenance & Cryptographic Linkage

## 1. Disaggregated Fact Hierarchy

Every verified repair run generates a structured evidence tree:

```text
.replayfix/
├── audit_receipt.json        (Canonical summary receipt + Ed25519 signature)
├── sweep_receipt.json        (When run via `replayfix sweep`)
└── artifacts/
    ├── candidate.patch       (Unified diff patch proposal)
    ├── 01_candidate.diff     (Canonical numbered diff)
    ├── 02_preflight.json     (Syntax & static analysis preflight evidence)
    ├── 03_differential_diagnosis.json (6-state causal attribution)
    ├── 04_regression_parity.json      (Regression evaluation evidence)
    ├── 04_audit_receipt.json          (Canonical audit receipt copy)
    ├── approval_audit.json            (Human confirmation gate decision)
    ├── issue_ingestion.json           (GitHub issue ingestion context, if applicable)
    ├── playwright.trace.zip           (Complete CDP execution trace, if --trace)
    └── merkle_receipt.json            (RFC 6962 tree with inclusion proofs + signature)
```

---

## 2. Merkle Leaf Commitment Order
Leaves are ordered deterministically by canonical file name (ASCII sort):
1. `01_candidate.diff`
2. `02_preflight.json`
3. `03_differential_diagnosis.json`
4. `04_audit_receipt.json`
5. `04_regression_parity.json`
6. `approval_audit.json` (if present)
7. `candidate.patch`
8. `issue_ingestion.json` (if present)
9. `playwright.trace.zip` (if present)

Every leaf is hashed using RFC 6962 prefix `0x00` and committed into the binary Merkle tree.
The Merkle root is then signed using the Ed25519 attestation key.
