# ReplayFix v3.0.0 — Implementation Plan: Universal Zero-Shot Pipeline

**Spec ID**: 300-v3.0.0-universal-zero-shot  
**Branch**: `release/v3.0.0-universal-zero-shot`  

---

## 1. Technical Architecture

### 1.1 Component Harness (`crates/replayfix-core/src/discovery.rs`)
- Upgrade `Babel.transform` configuration in `synthesize_component_harness_html` to use `presets: ['env', 'react', 'flow']`.
- Implement a self-contained React `DraggableCore` pointer-event implementation inside `dynamicRequire('react-draggable')` so `lastX` / `lastY` and `deltaX` / `deltaY` are computed relative to the handle's parent offset (matching `react-draggable` semantics).
- Update `DynamicMount` so top/left resize handles (`n`, `w`, `nw`, `sw`, `ne`) adjust `left` / `top` position alongside `width` / `height`, faithfully reproducing top-left resize handle jitter when `Resizable.js` lacks bounding-box delta compensation.
- Add `role="slider"` and `aria-label` attributes to rendered `.react-resizable-handle-*` nodes via a post-render `MutationObserver` / effect so Playwright `ariaSnapshot()` and `getByRole('slider', ...)` ground each handle uniquely without positional `.first()`.

### 1.2 Playwright Bridge & Grounding (`crates/replayfix-core/src/browser.rs`)
- Expand `PROBE_PAGE` candidate selector to include `[role="slider"]`, `[role="separator"]`, `[class*="handle"]`.
- Ensure unnamed `[class*="handle"]` nodes receive `role="slider"` and `aria-label` in the DOM before `ariaSnapshot()` is captured in `PROBE_PAGE`.
- In `DRAG_AND_OBSERVE`, track the bounding box of the grounded target element (or `.react-resizable` / `.harness-surface`) across drag steps and compute `jitter_score_bps` from sign reversals and acceleration spikes.

### 1.3 Autonomous Repair Diagnostics (`crates/replayfix-core/src/engine.rs` & `deliberation.rs`)
- Update `gate1_diagnostic` in `engine.rs` to describe drag/resize jitter and observed errors accurately rather than a static `TypeError` string when `drag_target_coords.is_some()`.
- Ensure `deliberation.rs` and `engine.rs` never consult `Fixture::for_task*` when `ExecutionMode::LiveAi` / `RepairClientMode::LiveEndpoint` is active.
