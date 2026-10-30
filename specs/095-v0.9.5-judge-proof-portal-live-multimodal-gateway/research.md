# Research & Architectural Decisions: Spec 095 — Judge Proof Portal & Live Multimodal Gateway

## 1. Research Topic: Offline Single-File Judge Proof Portal Architecture
- **Decision**: Deliver `proof_viewer.html` as an entirely self-contained single-file HTML5/CSS3/ES2022 application with embedded styling, SVG icons, and vanilla JavaScript. Zero CDN links, zero external CSS/JS fonts, zero Web Workers loading external scripts.
- **Rationale**:
  - Hackathon judges may review submissions offline, behind strict enterprise firewalls, or on air-gapped evaluation laptops.
  - Guarantees strict Content Security Policy (`default-src 'none'; style-src 'unsafe-inline'; script-src 'unsafe-inline'; img-src data: blob:; connect-src 'none';`).
  - Eliminates supply chain dependencies and external network leakage.
- **Alternatives Considered**:
  - React/Vue/Svelte SPA bundled via Vite: rejected because external build tooling complicates single-file evaluation and can introduce multi-megabyte bundle bloat and non-transparent DOM manipulation.
  - Electron / Native desktop app: rejected because judges require instant preview via standard web browsers (`file://` or static host).

## 2. Research Topic: Client-Side Cryptographic Verification in the Browser
- **Decision**: Use the browser's native `window.crypto.subtle` (Web Crypto API) for SHA-256 hashing, and embed a minimal, audited pure-JS Ed25519 verifier (RFC 8032) for signature verification.
- **Rationale**:
  - Web Crypto natively supports SHA-256 via `crypto.subtle.digest("SHA-256", bytes)` with maximum hardware acceleration.
  - While Web Crypto supports Ed25519 in modern Chromium and Safari, Firefox support has historically varied across versions; embedding a tiny, audited pure-JS RFC 8032 curve25519 verification routine guarantees 100% cross-browser determinism offline without polyfill CDNs.
- **Alternatives Considered**:
  - WebAssembly (Wasm) compiled from Rust (`replayfix-core` via `wasm-pack`): rejected because loading `.wasm` over `file://` protocol triggers CORS blocking in Chrome/Firefox (`Failed to fetch wasm`). Single-file HTML requires self-contained JS.

## 3. Research Topic: RFC 8785 Canonical JSON & RFC 6962 Leaf Hashing in Browser JS
- **Decision**: Implement RFC 8785 Section 3.2.3 in browser JavaScript with explicit UTF-16 code-unit sorting (`entries.sort((a, b) => compareUtf16(a[0], b[0]))`), zero float toleration, compact serialization, and RFC 6962 domain prefixes (`Uint8Array([0x00, ...])` for leaves, `Uint8Array([0x01, ...])` for internal nodes).
- **Rationale**:
  - Matches `replayfix-core::digest` and `merkle.rs` bit-for-bit, including the UTF-16 surrogate pair sorting fix proven in v0.9.1.
- **Alternatives Considered**:
  - Standard `JSON.stringify(obj, Object.keys(obj).sort())`: rejected because `Object.keys().sort()` in standard JS sorts by UTF-16 code units for BMP, but naive string sorting without surrogate handling can diverge from RFC 8785 on astral planes. A dedicated comparator ensures exact parity.

## 4. Research Topic: Meta Model API Cloud Gateway & Fallback Protocol
- **Decision**: In `crates/replayfix-core/src/gateway.rs`, implement an asynchronous HTTP gateway using standard REST request framing targeting `https://api.meta.ai/v1/responses` for `muse-wonder-1.0`, `muse-spark-1.3`, `sam-3-1`, and `muse-voice-transcribe-1.0`. For offline / fallback execution, route to `https://inference.baseten.co/v1` (`meta-models/muse-spark-open`) or deterministic local mock adapters.
- **Rationale**:
  - Enables clean separation between the deterministic Rust qualification harness and live cloud inference.
  - Accommodates developer environments with and without live Meta API tokens.
- **Alternatives Considered**:
  - Proprietary gRPC / WebSocket streaming: rejected because Meta Model API responses standard specifies HTTP POST `/v1/responses`.

## 5. Research Topic: Token-Bucket Rate Limiter & Circuit Breaker Architecture
- **Decision**: Implement an in-memory token bucket limiter (`RateLimiter`) with separate request (RPM) and token (TPM) capacity tracking, paired with a 3-state Circuit Breaker (`Closed -> Open -> Half-Open`) tracking consecutive errors and cooldown periods.
- **Rationale**:
  - Prevents provider rate quota exhaustion (HTTP 429) during batch operations or sweeps.
  - Automatically trips to protect system resources and triggers fallback when the upstream provider experiences transient outages.
- **Alternatives Considered**:
  - Leaky bucket: token bucket allows burst handling while maintaining average rate bounds, ideal for prompt synthesis bursts.
