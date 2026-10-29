# ReplayFix Trust & Cryptographic Evidence Architecture

## 1. Executive Summary & Zero-Trust Mandate
ReplayFix provides deterministic verification, autonomous repair, and verifiable evidence generation for modern web applications. In a zero-trust development environment, assertions of bug fixes cannot rely on trusting an AI agent, a developer, or execution logs. Verification outcomes must be mathematically provable, immutable, and cryptographically attributable.

This document formally defines the **4-Layer Trust Architecture** governing ReplayFix cryptographic evidence, resolving the structural distinctions between **Integrity**, **Inclusion**, **Authenticity**, and **Authorization**.

---

## 2. The 4-Layer Evidence Trust Model

```mermaid
flowchart TD
    subgraph L1["Layer 1: Content Integrity"]
        A["Raw Disaggregated Artifacts<br/>(candidate.diff, repro_spec.json, etc.)"] -->|SHA-256| B["Content Hash (content_sha256)"]
    end

    subgraph L2["Layer 2: Inclusion & Tamper-Evidence"]
        B --> C["RFC 8785 Canonical Leaf (JCS)"]
        C -->|SHA-256(0x00 || JCS)| D["RFC 6962 Merkle Tree"]
        D -->|Binary Balanced Tree| E["Merkle Tree Head Root (merkle_root)"]
        D -->|Audit Paths| F["Inclusion Proofs (inclusion_proofs)"]
    end

    subgraph L3["Layer 3: Execution Authenticity & Freshness"]
        E --> G["Canonical Signing Payload<br/>REPLAYFIX-ATTESTATION:v0.9:..."]
        H["Ed25519 Private Key<br/>(RFC 8032)"] -->|Digital Signature| I["Attestation Record (attestation.json)"]
        G --> I
    end

    subgraph L4["Layer 4: Runner Authorization & Trust Policy"]
        I --> J{"Trust Policy"}
        J -->|--trusted-key &lt;hex&gt;| K["VERIFIED_TRUSTED (Exit 0)"]
        J -->|No Key Pinning| L["VERIFIED_UNTRUSTED_SIGNER (Exit 0, Warning)"]
        J -->|Key Mismatch / Bad Sig| M["SIGNATURE_VERIFICATION_FAILED (Exit 40)"]
    end
```

### Layer 1: Content Integrity (Raw Artifact Digests)
- **Target**: Individual concrete files emitted into `<output_dir>/artifacts/` (`01_candidate.diff`, `02_repro_spec.json`, `03_differential_diagnosis.json`, `04_regression_parity.json`, `04_audit_receipt.json`).
- **Mechanism**: Raw SHA-256 hash computed over exact uncompressed file bytes on disk.
- **Guarantee**: Guarantees that an individual file has not suffered bit rot, truncation, or inadvertent modification.
- **Limitation**: Does not prove that two files belong to the same execution run, nor who created them.

### Layer 2: Inclusion & Tamper-Evidence (RFC 6962 Merkle Tree)
- **Target**: The complete ordered collection of run artifacts ($N$ leaves).
- **Mechanism**:
  - Each leaf $L_i$ is serialized into canonical JSON using **RFC 8785 (JSON Canonicalization Scheme / JCS)**, with recursive UTF-16 code unit key sorting, compact delimiters, and float rejection.
  - Domain separation per RFC 6962:
    - Leaf hash: $H(L_i) = \text{SHA-256}(0x00 \parallel \text{JCS}(L_i))$
    - Internal node hash: $H(N) = \text{SHA-256}(0x01 \parallel \text{Left} \parallel \text{Right})$
    - Empty tree root: $\text{MTH}(\emptyset) = \text{SHA-256}("") = \text{e3b0c44...}$
- **Guarantee**: Binds all evidence into a single 32-byte Merkle root hash. Any addition, deletion, reordering, or alteration of any artifact produces an invalid Merkle root. Enables $O(\log N)$ inclusion proofs without transmitting the entire artifact payload.
- **Limitation**: Merkle trees provide integrity and inclusion, but **zero authenticity**. Anyone can compute a valid Merkle tree over fraudulent data.

### Layer 3: Execution Authenticity & Freshness (RFC 8032 Ed25519 Attestation)
- **Target**: The Merkle root, tree size, schema version, and timestamp.
- **Mechanism**: Pure Ed25519 digital signature over an unambiguous, version-prefixed, domain-separated ASCII payload:
  $$\text{"REPLAYFIX-ATTESTATION:v0.9:"} \parallel \text{schema\_version} \parallel \text{":"} \parallel \text{merkle\_root} \parallel \text{":"} \parallel \text{tree\_size} \parallel \text{":"} \parallel \text{created\_at}$$
- **Guarantees**:
  - **Authenticity**: Proves possession of the private key corresponding to `public_key`.
  - **Non-repudiation**: The signer cannot claim they signed a different receipt or different files.
  - **Tree Size Binding**: Binds `tree_size` directly into the signed payload, preventing leaf truncation or extension attacks.
  - **Anti-Replay & Freshness**: RFC 3339 UTC timestamp with millisecond precision binds when the verification occurred.
  - **Domain Separation**: The versioned `REPLAYFIX-ATTESTATION:v0.9:` prefix prevents cross-protocol reuse of signatures on different cryptographic structures.

### Layer 4: Runner Authorization & Trust Policy (Public-Key Pinning)
- **Target**: The public key declared in the `AttestationRecord`.
- **Mechanism**: Verifier evaluates the public key against a configured `TrustPolicy`:
  1. `TrustPolicy::PinnedKeys(HashSet<String>)` (CLI: `--trusted-key <hex>`):
     Enforces that the signer public key exactly matches an authorized, pinned authority (e.g. Meta Hackathon CI, Enterprise Security Auditor, or Project Maintainer).
     - Match $\rightarrow$ `VERIFIED_TRUSTED` (`exit 0`).
     - Mismatch $\rightarrow$ `CoreError::SecurityViolation` / `SIGNATURE_VERIFICATION_FAILED` (`exit 40`).
  2. `TrustPolicy::AllowAny` (CLI without `--trusted-key`):
     Evaluates cryptographic signature validity.
     - Valid signature $\rightarrow$ `VERIFIED_UNTRUSTED_SIGNER` (`exit 0` with security warning).
     - Invalid signature $\rightarrow$ `SIGNATURE_VERIFICATION_FAILED` (`exit 40`).

---

## 3. Threat Matrix & Defensive Invariants

| Threat Scenario | Vector | Defensive Layer | ReplayFix Outcome |
|---|---|---|---|
| **Artifact Tampering** | Attacker edits `candidate.diff` post-run to inject backdoor. | Layer 1 & Layer 2 | `MERKLE_PROOF_INVALID`: Leaf hash mismatch against Merkle root. Exit code 40. |
| **Receipt Forgery** | Attacker fabricates an `audit_receipt.json` claiming green gates without running harness. | Layer 3 & Layer 4 | Cannot forge valid Ed25519 signature under pinned `--trusted-key`. Exit code 40. |
| **Replay Attack** | Attacker captures receipt from an old run and submits it for a new commit. | Layer 3 | Signed payload binds `created_at` timestamp and schema version; base repository tree hash is committed in receipt. |
| **Proof Key Divergence** | External verifier indexes `inclusion_proofs` by leaf index (`"0"`, `"1"`) or leaf hash instead of file name. | Layer 2 | Conforming verifier attempts canonical `file_name`, falls back to index string, leaf hash, or content SHA-256. |
| **Canonical Serialization Mismatch** | JSON serializers sort keys differently (`"a":1,"b":2` vs `"b":2,"a":1`). | Layer 2 | Mandatory RFC 8785 (JCS) enforcement guarantees byte-for-byte serialization invariance across all languages. |
| **Tree Extension / Truncation** | Attacker appends dummy leaves to receipt. | Layer 2 & Layer 3 | Tree size is signed in Ed25519 payload; modifying leaf count invalidates signature. |

---

## 4. Cryptographic Standards Summary
- **Leaf Canonicalization**: [RFC 8785](https://www.rfc-editor.org/rfc/rfc8785) (JSON Canonicalization Scheme).
- **Merkle Tree & Inclusion Proofs**: [RFC 6962 Section 2](https://www.rfc-editor.org/rfc/rfc6962#section-2) (Certificate Transparency domain separation: `0x00` leaf, `0x01` internal node).
- **Digital Signatures**: [RFC 8032](https://www.rfc-editor.org/rfc/rfc8032) (Pure Ed25519).
- **Hashing**: FIPS 180-4 SHA-256 (32-byte digests).
