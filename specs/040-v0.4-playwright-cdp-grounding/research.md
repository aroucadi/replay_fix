# Research & Technical Decisions: 040-v0.4-playwright-cdp-grounding

**Feature**: `040-v0.4-playwright-cdp-grounding`  
**Date**: 2026-10-27

---

## Decision 1: Line-Delimited JSON-Lines Stdio Bridge (`playwright_bridge.js`)

- **Decision**: Embed a compact, zero-dependency Node.js script (`PLAYWRIGHT_BRIDGE_JS` in `crates/replayfix-core/src/browser.rs`) that is materialized inside the ephemeral `ScratchSandbox` (`<sandbox>/.replayfix-bridge.js`) during live browser runs and communicates with Rust over line-delimited JSON (`stdin` / `stdout`) with explicit newline framing.
- **Rationale**:
  - Materializing the bridge script inside the scratch sandbox (outside the user's base repository) preserves $\Delta \text{repo} = 0$ and keeps the `replayfix` binary self-contained so it can be invoked from any working directory.
  - Resolving `playwright` dynamically across local `node_modules`, `NODE_PATH`, global `npm root -g`, and the npm `_npx` cache (`npm config get cache`) matches how `replayfix doctor` probes `npx --no-install playwright --version` on Windows, macOS, and Linux.
- **Alternatives Considered**:
  - *Raw WebSocket CDP client in Rust*: Requires a WebSocket crate (`tokio-tungstenite`) and re-implementing Playwright's `ariaSnapshot` YAML generator from raw `Accessibility.getFullAXTree` nodes. Using Playwright's native `page.locator('body').ariaSnapshot()` gives exact parity with Playwright locators while adding zero Rust crate dependencies.

---

## Decision 2: Spatial-Semantic Locator Scoring & DOM Grounding Invariant

- **Decision**: Score each interactive candidate returned by the page using integer basis points (`0..=10_000` bps, where `10_000` = `1.0000`) and enforce both the **DOM Grounding Invariant** and the **Fail-Closed Ambiguity Guard** ($|\Delta S| < 2500$ bps, i.e. $0.25$):
  1. **DOM Grounding Invariant** (inspired by Meta's `quote_on_page` verification pattern):
     - A candidate is grounded if and only if:
       - It has positive bounding-box dimensions (`width > 0 && height > 0`),
       - Its role and accessible name (or `data-testid`) are physically verified in the live `ariaSnapshot` / DOM query, and
       - Its synthesized selector uses `getByRole`, `getByTestId`, or `getByLabel` and NEVER contains `.first()` or `:nth-match`.
  2. **Spatial-Semantic Score Formula (`0..=10_000` bps)**:
     - **Point Containment (`4_000` bps)**: `4_000` bps if $(x_{\text{click}}, y_{\text{click}})$ falls inside the candidate's bounding box (`[x, x + width] x [y, y + height]`), `0` otherwise.
     - **Element-From-Point Hit (`2_500` bps)**: `2_500` bps if `document.elementFromPoint(x, y).closest(...)` resolved directly to this candidate.
     - **Center Proximity (`0..=2_000` bps)**: $\max\left(0, 2000 - \lfloor \text{dist} \cdot 10 \rfloor\right)$ based on Euclidean pixel distance from $(x_{\text{click}}, y_{\text{click}})$ to the bounding box center $(c_x, c_y)$.
     - **Semantic Accessibility Specificity (`0..=1_500` bps)**: `1_000` bps when both interactive `role` and non-empty `accessible_name` are confirmed in `ariaSnapshot`, plus `500` bps when a dedicated `data-testid` attribute is present.
  3. **Fail-Closed Ambiguity Guard**:
     - If zero candidates pass grounding, or if the highest-scoring candidate has `< 5_000` bps, or if a second distinct candidate has score within `2_500` bps ($|\Delta S| = S_1 - S_2 < 2500$ bps), `LocatorResolver::resolve` fails closed with `TriageStatus::AmbiguousGrounding` (`AMBIGUOUS_GROUNDING`, exit code `22`).
- **Rationale**: Integer basis points keep all domain and receipt types `Eq` and 100% compatible with RFC 8785 `canonical_json` while providing $0.0001$ precision.

---

## Decision 3: Embedded Sandbox HTTP Server on Ephemeral Port

- **Decision**: `SandboxServer` in `crates/replayfix-core/src/browser.rs` binds `std::net::TcpListener` on `127.0.0.1:0` (satisfying Constitutional Law IV: Child-Owned Ephemeral Provenance) and serves the `ScratchSandbox` HTML and JavaScript files (`index.html` and `src/cart-summary.js`) to the Chromium browser during Gate 1 and Gate 2.
- **Rationale**:
  - When `ScratchSandbox` patches `src/cart-summary.js` on disk between Gate 1 and Gate 2, Gate 2 navigates to `http://127.0.0.1:<port>/index.html` and loads the freshly patched `src/cart-summary.js` directly from the scratch sandbox.
  - Runs in a lightweight background thread with a shutdown flag and non-blocking / short-timeout accept loop, guaranteeing deterministic teardown with zero external dependencies.
