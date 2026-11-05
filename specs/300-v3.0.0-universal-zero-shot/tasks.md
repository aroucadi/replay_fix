# ReplayFix v3.0.0 — Execution Tasks: Universal Zero-Shot Pipeline

**Spec ID**: 300-v3.0.0-universal-zero-shot  
**Branch**: `release/v3.0.0-universal-zero-shot`  

---

## Task Breakdown

- [x] **Task 300-01**: Audit codebase and create branch `release/v3.0.0-universal-zero-shot` + initialize Spec-Kit artifacts (`spec.md`, `plan.md`, `tasks.md`).
- [x] **Task 300-02**: Fix `gateway.rs` `"error": null` wire check and add graceful `sam-3.1` empty-output fallback to `local_diff_heuristic` with frame-pair displacement (`RoiDiffer::pair_change`) in `perception.rs`.
- [x] **Task 300-03**: Upgrade `discovery.rs` (`synthesize_component_harness_html`) to configure Babel Standalone with `['env', 'flow', 'react']` presets, strip Flow annotations safely, provide a self-contained `DraggableCore` with `offsetParent` relative coordinates and axis-specific accessible handle roles (`Resize Handle NW`), and support multi-child iterable component surfaces.
- [x] **Task 300-04**: Update `browser.rs` `PROBE_PAGE` and `DRAG_AND_OBSERVE` to assign accessible `slider` roles/labels to unnamed handle elements before `ariaSnapshot`, discover `[role="slider"], [role="separator"], [class*="handle"]` candidates, and measure real step-by-step physical bounding-box jitter (`jitter_score_bps`).
- [x] **Task 300-05**: Ensure strict `LiveAi` / `LiveEndpoint` zero-fixture isolation across `deliberation.rs`, `engine.rs`, `heal.rs`, and `repair.rs`, and add `StageExecutionMatrix` honesty provenance to `receipt.rs` and `ui.rs`.
- [x] **Task 300-06**: Validate live zero-shot `repro` and `heal` on `benchmarks/react-resizable` (`VERIFIED_REPAIRED` on `lib/Resizable.js` attempt 1/3 via live `muse-spark-1.3`), verify tests, and commit/push `release/v3.0.0-universal-zero-shot`.
