# Quickstart & Validation Guide: Spec 095

## 1. Prerequisites
- Modern web browser (Chrome, Edge, Firefox, Safari)
- Rust toolchain $\ge 1.80$
- Existing ReplayFix workspace with v0.9.1 baseline

## 2. Opening the Judge Proof Portal Offline
1. Locate `proof_viewer.html` in the repository root.
2. Open directly via your browser:
   - On Windows: `start proof_viewer.html`
   - Or double click the file in Windows Explorer.
3. Observe that the portal loads instantly with:
   - Header: **ReplayFix — Judge Proof Portal & Cryptographic Verification**
   - Verification Status: **NO RECEIPT LOADED**
   - Drag & Drop zone for `audit_receipt.json` or `sweep_receipt.json`.

## 3. Validating an Audit Receipt
1. Drag and drop `.replayfix/audit_receipt.json` (or any benchmark receipt from `.replayfix/RF-01/audit_receipt.json`).
2. Verify that:
   - Status badge transitions to `VERIFIED GENUINE` (or `SIGNATURE VALID — SIGNER UNTRUSTED` if custom key).
   - Merkle Tree tab renders the leaf hierarchy and displays live cryptographic recomputation on click.
   - Unified Diff tab shows syntax-highlighted code repairs.
   - Regression Parity tab confirms $F_{\text{patched}} \setminus F_{\text{baseline}} = \emptyset$.

## 4. Validating a Five-Bug Sweep Receipt
1. Run a fresh sweep if needed:
   ```bash
   cargo run --bin replayfix -- sweep --mock-heal
   ```
2. Drag and drop `.replayfix/sweep_receipt.json` into `proof_viewer.html`.
3. Verify that the Sweep View displays 5 benchmark bug cards (`RF-01` to `RF-05`) with green status indicators, elapsed execution time, and the sweep Merkle root.

## 5. Validating Live Cloud Gateway & Circuit Breaker (CLI / Library)
1. Run gateway unit tests:
   ```bash
   cargo test --package replayfix-core gateway
   ```
2. Verify that:
   - Token-bucket rate limiting enforces rate ceilings.
   - Circuit breaker trips to `Open` after 3 failures and recovers to `HalfOpen`.
   - Zero-Audio Rule rejects accidental audio on code repair tasks.
