# ADR-01: Offline Portal Architecture (`proof_viewer.html`)

## Status
Accepted

## Context
Judges and independent security auditors need a deterministic, zero-setup way to inspect ReplayFix receipts (`audit_receipt.json`, `merkle_receipt.json`, `sweep_receipt.json`), verify RFC 6962 Merkle proofs and Ed25519 signatures, and inspect multimodal evidence (keyframes, SAM 3.1 masks, DOM snapshots, unified diffs) offline.

## Decision
1. **Single-File Self-Contained Deliverable (`proof_viewer.html`)**:
   - All HTML5 structure, CSS3 design tokens, SVG icons, embedded sample fixtures (for instant zero-click demo inspection alongside drag-and-drop file import), and ES2022 verification logic reside in `proof_viewer.html` at the repository root.
2. **Strict Offline Content Security Policy (CSP)**:
   - Enforces `<meta http-equiv="Content-Security-Policy" content="default-src 'none'; script-src 'unsafe-inline'; style-src 'unsafe-inline'; img-src data: blob:; media-src data: blob:; connect-src 'none'; base-uri 'none'; form-action 'none';">`.
   - Zero external network calls (`fetch`, `XMLHttpRequest`, `WebSocket`, external fonts/CDNs) are permitted.
3. **Bounded Untrusted Input Handling**:
   - Rejects files $> 50	ext{MB}$ (`52,428,800` bytes), enforces maximum JSON nesting depth ($\le 32$), strips `__proto__` / `constructor` / `prototype` keys during parsing, and renders all receipt strings exclusively via `textContent` / safe DOM APIs.

## Alternatives Considered
- **Multi-file SPA (Vite/React) or WebAssembly (`.wasm`)**: Rejected because browsers block `fetch('*.wasm')` over `file://` due to CORS restrictions, breaking offline local-file double-click usage.

## Consequences
- **Positive**: Works 100% offline via `file://` or static hosting (GitHub Pages) with zero supply-chain attack surface and zero secret exposure.
- **Test Strategy**: Validated via Rust structural/security tests on `proof_viewer.html` and cross-checked against `replayfix-core` cryptographic vectors.
