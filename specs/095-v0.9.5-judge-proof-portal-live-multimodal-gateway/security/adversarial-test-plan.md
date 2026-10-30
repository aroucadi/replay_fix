# Adversarial Test Plan: Spec 095

## 1. Portal Cryptographic & Input Hostility Tests
1. **Bit-Flip Tampered Merkle Root & Leaf (`adv_portal_01`)**:
   - Flip 1 hex character in `content_sha256` of `01_candidate.diff` inside `merkle_receipt.json` while keeping `"status": "VERIFIED_REPAIRED"`.
   - Expected: Portal computes mismatched leaf hash, fails Merkle root comparison, and renders `INTEGRITY CHECK FAILED`.
2. **Untrusted Signer Key (`adv_portal_02`)**:
   - Sign a valid Merkle root with a fresh ephemeral Ed25519 keypair not present in the portal's pinned key set.
   - Expected: Portal verifies the Ed25519 signature mathematically, refuses `VERIFIED GENUINE`, and renders `SIGNATURE VALID — SIGNER UNTRUSTED`.
3. **XSS & Prototype Pollution Payloads (`adv_portal_03`)**:
   - Inject `<img src=x onerror=alert(1)>`, `<svg onload=alert(1)>`, and `{"__proto__": {"isAdmin": true}}` into receipt fields.
   - Expected: Safe DOM text rendering (`textContent`) neutralizes markup; `Object.prototype.isAdmin` remains `undefined`.
4. **Oversized & Deeply Nested JSON (`adv_portal_04`)**:
   - Pass input exceeding 50 MB limit or nesting depth $> 32$.
   - Expected: Rejected before full verification with `MALFORMED RECEIPT`.

## 2. Cloud Gateway & Multimodal Adversarial Tests
5. **Zero-Audio Rule Violation (`adv_gateway_01`)**:
   - Dispatch a `GatewayRequest` to `muse-spark-1.3` or `muse-wonder-1.0` with `include_audio: true` or non-empty audio bytes.
   - Expected: Immediate fail-closed `CoreError::SecurityViolation` with zero network traffic.
6. **HTTP 429 Rate Limit & Circuit Breaker Trip (`adv_gateway_02`)**:
   - Inject 3 consecutive provider failures (`HTTP 503` / `HTTP 429`).
   - Expected: `CircuitBreaker` transitions from `Closed` to `Open`, subsequent request routes to `BasetenFallback` (`meta-models/muse-spark-open`), and after cooldown transitions to `HalfOpen` -> `Closed`.
7. **API Token Redaction (`adv_gateway_03`)**:
   - Configure gateway with secret `meta_secret_token_99999` and trigger a failure and `format!("{:?}", gateway)`.
   - Expected: Output contains `[REDACTED]` and never contains `meta_secret_token_99999`.
8. **Out-of-Bounds SAM 3.1 Coordinates (`adv_multimodal_01`)**:
   - Supply SAM bounding box coordinates with negative values or values $> 1000$ milliparts / outside viewport dimensions.
   - Expected: Clamped or flagged as invalid geometry without canvas/SVG injection.
