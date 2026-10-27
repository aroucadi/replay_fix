# Quickstart Validation Guide: 040-v0.4-playwright-cdp-grounding

**Feature**: `040-v0.4-playwright-cdp-grounding`  
**Date**: 2026-10-27

---

## 1. Prerequisites

Ensure `replayfix doctor` reports `[OK]` for Git, Node.js, and Playwright:

```bash
cargo run -p replayfix-cli -- doctor
```

---

## 2. Run Live Browser Two-Gate Verification (`--live-browser`)

Run the full two-gate loop (Gate 1 RED in Chromium -> patch on disk in scratch sandbox -> Gate 2 GREEN in Chromium):

```bash
cargo run -p replayfix-cli -- repro \
  --task-id RF-01 \
  --bug-ref bug/RF-01 \
  --video recordings/RF-01.mp4 \
  --live-browser
```

**Expected Outcome**:
- Ephemeral sandbox HTTP server starts on `127.0.0.1:<port>`.
- Chromium reverse-probes the hotspot `(160, 120)` via `document.elementFromPoint` and `ariaSnapshot`, grounding `getByRole('button', { name: 'Refresh Cart' })`.
- **Gate 1 (RED)**: Clicking the button in Chromium triggers `TypeError: Cannot read properties of undefined (reading 'length')`.
- **Repair**: Applies the unified diff to `src/cart-summary.js` inside the scratch sandbox.
- **Gate 2 (GREEN)**: Reloads the page in Chromium, clicks the same grounded button, observes `"Items: 0"` with zero page errors, verifies $\Delta \text{repo} = 0$, and writes `.replayfix/audit_receipt.json` (`exit code 0`).

To watch the Chromium browser window live on your desktop, add `--headed`:

```bash
cargo run -p replayfix-cli -- repro \
  --task-id RF-01 \
  --bug-ref bug/RF-01 \
  --video recordings/RF-01.mp4 \
  --live-browser \
  --headed
```

---

## 3. Run Unit & Integration Tests

```bash
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
```
