# ADR-06: Five-Bug Sweep and Isolation Protocol

## Status
Accepted

## Context
Milestone v0.9.0 requires a comprehensive repair sweep over five distinct MetaPulse benchmark defects (`RF-01` through `RF-05`). The sweep must guarantee strict isolation between bugs, accurate resource attribution, and a unified cryptographic sweep receipt.

## Decision
1. **Benchmark Defect Suite (`RF-01` to `RF-05`)**:
   - `RF-01`: Cart Summary null reference (`cartItemCount` in `src/cart-summary.js`).
   - `RF-02`: Stale Discount double calculation (`applyDiscount` in `src/discount.js`).
   - `RF-03`: Inventory Underflow below zero (`decrementQuantity` in `src/inventory.js`).
   - `RF-04`: Cart State Reset on tab switch (`switchTab` in `src/cart-store.js`).
   - `RF-05`: Double Submit idempotency race (`submitOrder` in `src/order.js`).

2. **Per-Bug Workspace Isolation**:
   - Every bug case $B_i$ executes in its own distinct OS temp directory via `ScratchSandbox::create(&repo, bug_id)`.
   - Temporary ports for web servers are allocated dynamically via `127.0.0.1:0`.
   - Sandbox teardown occurs immediately after the bug run finishes or fails.

3. **Sweep Execution Protocol**:
   - CLI invocation: `replayfix sweep [--bugs RF-01,RF-02,...] [--model-pool muse-spark-1.3,muse-wonder-1.0]`
   - Sequential bug execution with concurrent model generation per bug.
   - Fault Isolation: A failure in `RF-02` does NOT abort the remaining bugs; each bug generates its own individual diagnosis, and the aggregate sweep receipt records the outcome per bug.

4. **Cryptographic Sweep Receipt**:
   - The sweep emits `.replayfix/sweep_receipt.json`.
   - Contains a Merkle tree whose leaves are the individual `AuditReceipt` / `TriageReport` hashes of each bug case.
   - Signed with Ed25519, proving all 5 cases were executed within the attestation envelope.

## Consequences
- **Positive**: Complete benchmark sweep capability with zero cross-bug state contamination.
