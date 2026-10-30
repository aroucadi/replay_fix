# Contract: Offline Judge Proof Portal (`proof_viewer.html`)

## 1. Specification & Packaging
- **File Name**: `proof_viewer.html`
- **Delivery Mode**: Standalone single-file HTML document containing all inline `<style>` and `<script>` definitions.
- **Network Invariant**: Zero external network requests (`fetch`, `XMLHttpRequest`, `WebSocket`, `WebRTC`, external `<script>`/`<link>`/`<img>` tags are strictly forbidden).
- **Protocol Support**: Must function seamlessly when opened directly via the `file://` URI scheme in standard modern web browsers (Chrome $\ge 110$, Firefox $\ge 115$, Safari $\ge 16.4$, Edge $\ge 110$).

## 2. Input Security & Sanitization
- **Max Upload Size**: 50 MB (`52,428,800` bytes).
- **Encoding**: UTF-8 exclusively.
- **XSS Defense**: All user-supplied and receipt-supplied strings (including file names, commit hashes, model explanations, unified diffs, task descriptions) MUST be inserted via `textContent` or escaped through safe DOM node construction. `innerHTML` is forbidden on untrusted data.
- **DOM Evaluation**: No `eval()`, `new Function()`, `setTimeout(string)`, or external script loaders.

## 3. UI Layout & Viewports
- **Header**: Application identity, active receipt filename, overall verification status banner, and theme/view selector.
- **Overview Card**: Receipt metadata, schema version, task ID, substrate, git base tree digest, wall-clock execution time.
- **Tabbed Panels**:
  1. **Merkle Proof Explorer**: Interactive visual tree with node derivation, leaf canonical bytes, and live hash recomputation.
  2. **Attestation & Keys**: Pure Ed25519 signature payload, public key, algorithm, and trust policy match.
  3. **Visual & Multimodal Evidence**: Timeline scrubber, video keyframes, SAM 3.1 segmentation overlays, and click centroid coordinates.
  4. **Unified Diff & Regression**: Line-by-line syntax-highlighted diff (+green, -red) and regression parity status.
  5. **Sweep View**: (Active when inspecting `sweep_receipt.json`) 5-card benchmark grid for `RF-01` to `RF-05`.
  6. **Raw Receipt JSON**: Collapsible syntax-highlighted canonical JSON tree.
