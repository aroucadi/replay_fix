# Quickstart Validation Guide: 060-v0.6-meta-model-bounded-repair

**Feature**: `060-v0.6-meta-model-bounded-repair`  
**Date**: 2026-10-28

---

## 1. Prerequisites

Verify all dependencies (`Git`, `Node.js`, `Playwright`, `FFmpeg`) via `replayfix doctor`:

```powershell
cargo run -p replayfix-cli -- doctor
```

---

## 2. Autonomous Bounded Repair in Live Chromium (`--model muse-spark-1.3`)

Run the full closed-loop autonomous repair workflow on `RF-01`:

```powershell
cargo run -p replayfix-cli -- run --bug RF-01 --live-browser --headed --model muse-spark-1.3 --output-dir target/qa-v06
```

**Expected Outcome**:
1. **Test Freeze**: `TestFreezeGuard` hashes protected verification files and contract before repair.
2. **Gate 1 (RED)**: Chromium reproduces `TypeError: Cannot read properties of undefined (reading 'length')`.
3. **Bounded Repair (`muse-spark-1.3`, `effort: "xhigh"`)**: Builds an `<UNTRUSTED_CONTEXT>`-fenced prompt with static prefix caching (`replayfix-v0.6-harness-prefix-v1`), decodes the strict JSON Schema `StructuredRepairPatch` (`search_block` -> `replace_block`), verifies unique block occurrence in `src/cart-summary.js`, and writes the patched file inside `ScratchSandbox`.
4. **Syntax Preflight**: Runs `node --check src/cart-summary.js` inside the scratch sandbox before launching Chromium.
5. **Gate 2 Stability (2x GREEN)**: Executes 2 consecutive Chromium verification runs (`gate2_runs_completed: 2`, `gate2_stability_passed: true`), verifies $\Delta \text{repo} = 0$ and `test_immutability_verified: true`, and writes `target/qa-v06/audit_receipt.json` (`schema_version: "0.6"`, exit code `0`).

---

## 3. Quarantine After 3 Failed Repair Attempts (`--mock-failing-repair`)

Verify bounded retry exhaustion (`MAX_REPAIR_ATTEMPTS = 3`, $\le 180\text{s}$ ceiling) and `QUARANTINE_REPAIR_FAILED` (exit code `21`):

```powershell
cargo run -p replayfix-cli -- run --bug RF-01 --mock-failing-repair --output-dir target/qa-v06-quarantine
```

**Expected Outcome**:
- Attempt 1 requests `include: ["reasoning.encrypted_content"]`; Attempts 2 and 3 chain `previous_response_id` and preserve `encrypted_content`.
- After 3 failed attempts, the harness exits with code `21` (`QUARANTINE_REPAIR_FAILED`) and writes `target/qa-v06-quarantine/triage_report.json` with the 3-attempt lineage and `token_accounting`.
