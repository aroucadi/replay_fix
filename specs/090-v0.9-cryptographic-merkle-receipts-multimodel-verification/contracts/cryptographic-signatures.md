# Contract: Cryptographic Ed25519 Signatures & Attestation

## 1. Specification & Payload

### 1.1 Signature Algorithm
Pure Ed25519 conforming to RFC 8032.

### 1.2 Signing Payload Construction
The message $M$ signed by the private key is strictly canonicalized as:
$$M = \text{"REPLAYFIX-ATTESTATION:v0.9:"} \parallel \text{schema\_version} \parallel \text{":"} \parallel \text{merkle\_root} \parallel \text{":"} \parallel \text{tree\_size} \parallel \text{":"} \parallel \text{created\_at}$$

Example:
`REPLAYFIX-ATTESTATION:v0.9:0.9:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad:7:2026-10-29T16:00:00.000Z`

---

## 2. Attestation Schema (`contracts/attestation.schema.json`)

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "AttestationRecord",
  "type": "object",
  "required": [
    "schema_version",
    "algorithm",
    "public_key",
    "signature",
    "signing_payload",
    "created_at"
  ],
  "properties": {
    "schema_version": { "type": "string", "const": "0.9" },
    "algorithm": { "type": "string", "const": "Ed25519" },
    "public_key": { "type": "string", "pattern": "^[0-9a-f]{64}$" },
    "signature": { "type": "string", "pattern": "^[0-9a-f]{128}$" },
    "signing_payload": { "type": "string" },
    "created_at": { "type": "string", "format": "date-time" }
  }
}
```

---

## 3. Trust & Verification Outcomes

| Condition | Status | Exit Code | Action |
|---|---|---|---|
| Valid signature, trusted public key | `VERIFIED_TRUSTED` | 0 | Success; authorized execution authority. |
| Valid signature, untrusted public key (no `--trusted-key` pin) | `VERIFIED_UNTRUSTED_SIGNER` | 0 | Warning logged; valid signature but untrusted signer. |
| Invalid signature / corrupted payload | `SIGNATURE_VERIFICATION_FAILED` | 40 | Fail-closed; cryptographic mismatch. |
| Signer public key not present in pinned `--trusted-key` | `SIGNATURE_VERIFICATION_FAILED` | 40 | Fail-closed; unauthorized signer. |
| Tampered Merkle leaf / invalid proof | `MERKLE_PROOF_INVALID` | 40 | Fail-closed; evidence integrity violated. |
