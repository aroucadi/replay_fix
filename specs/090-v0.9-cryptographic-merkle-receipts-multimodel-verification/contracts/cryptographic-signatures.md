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

## 2. Attestation Schema

```json
{
  "title": "Ed25519Attestation",
  "type": "object",
  "required": ["public_key", "signature", "signing_payload", "algorithm"],
  "properties": {
    "public_key": { "type": "string", "pattern": "^[0-9a-f]{64}$" },
    "signature": { "type": "string", "pattern": "^[0-9a-f]{128}$" },
    "signing_payload": { "type": "string" },
    "algorithm": { "type": "string", "const": "Ed25519" }
  }
}
```

---

## 3. Trust & Verification Outcomes

| Condition | Status | Exit Code |
|---|---|---|
| Valid signature, trusted public key | `VERIFIED_TRUSTED` | 0 |
| Valid signature, untrusted public key (no `--trusted-key` pin) | `VERIFIED_UNTRUSTED_SIGNER` | 0 (Warning logged) |
| Invalid signature / corrupted payload | `SIGNATURE_VERIFICATION_FAILED` | 40 |
| Tampered Merkle leaf / invalid proof | `MERKLE_PROOF_INVALID` | 40 |
