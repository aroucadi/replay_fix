# Contract: Five-Bug Benchmark Sweep (`RF-01` to `RF-05`)

## 1. Benchmark Scenarios Specification

| Task ID | Component | Defect Mode | Target Symbol | Baseline Postcondition |
|---|---|---|---|---|
| `RF-01` | `src/cart-summary.js` | TypeError on empty cart | `cartItemCount` | Returns 0 for undefined/empty items |
| `RF-02` | `src/discount.js` | State mutation twice | `applyDiscount` | Subtotal discounted exactly once |
| `RF-03` | `src/inventory.js` | Underflow below zero | `decrementQuantity` | Quantity never drops below 0 |
| `RF-04` | `src/cart-store.js` | Store reset on tab change | `switchTab` | Cart items preserved across tab transitions |
| `RF-05` | `src/order.js` | Double submit idempotency | `submitOrder` | Second click ignored while submission pending |

---

## 2. Sweep Execution Invariants
1. **Zero Cross-Bug Contamination**: Bug $i$ executes in sandbox $S_i$ with zero access to sandbox $S_j$.
2. **Ephemeral Port Allocation**: Every sandbox server dynamically binds to port `:0`.
3. **Aggregate Receipt**: Outputs `.replayfix/sweep_receipt.json` containing:
   - `schema_version`: `"0.9"`
   - `total_bugs`: 5
   - `repaired_bugs`: Count of `VERIFIED_REPAIRED` cases
   - `failed_bugs`: Count of non-repaired cases
   - `results`: Map of bug ID to individual `AuditReceipt` summary
   - `merkle_root`: Merkle root of the 5 bug receipts
   - `ed25519_attestation`: Signature over the sweep Merkle root
   - `elapsed_wall_clock_ms`: Total execution time
4. **Performance Target**:
   - Total sweep execution time $\le 60,000$ ms ($< 60$s) under simulated / offline fixtures.
