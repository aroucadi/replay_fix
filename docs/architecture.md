# ReplayFix v3.0.0 Architecture & Verification Substrate

This document details the systems architecture of **ReplayFix v3.0.0**, explaining how the deterministic Rust verification harness orchestrates video perception, universal zero-shot component mounting, live Playwright CDP browser verification, Meta AI model repair, and RFC 6962 cryptographic notarization.

---

## 1. High-Level System Architecture

```text
┌────────────────────────────────────────────────────────────────────────────────────────┐
│                    REPLAYFIX v3.0.0 UNIVERSAL ZERO-SHOT PIPELINE                       │
│                                                                                        │
│   ┌───────────────────────────────────┐    ┌───────────────────────────────────────┐   │
│   │   1. MULTIMODAL PERCEPTION        │    │   2. UNIVERSAL COMPONENT HARNESS      │   │
│   │   FFmpeg + SAM 3.1 + RoiDiffer    │    │   RepoSourceMapper + Babel Standalone │   │
│   │   • Dense 100ms frame extraction  │───▶│   • Source ranking by stem/AST tokens │   │
│   │   • Frame-pair optical flow       │    │   • In-browser ['env','flow','react'] │   │
│   │   • Click & drag vector extraction│    │   • offsetParent DraggableCore mount  │   │
│   └───────────────────────────────────┘    └───────────────────┬───────────────────┘   │
│                                                                │                       │
│                                                                ▼                       │
│   ┌───────────────────────────────────┐    ┌───────────────────────────────────────┐   │
│   │   4. BOUNDED AST REPAIR & NOTARY  │    │   3. LIVE CDP TWO-GATE ORACLE         │   │
│   │   Muse Spark 1.3 / Wonder 1.0     │    │   Playwright Chromium Bridge          │   │
│   │   • SHA-256 TestFreezeGuard       │◀───│   • elementFromPoint + ariaSnapshot   │   │
│   │   • Lexical symbol BlockPatcher   │───▶│   • 10-step drag jitter_score_bps     │   │
│   │   • RFC 6962 Merkle + Ed25519     │    │   • Gate 1 RED & Gate 2 GREEN (x2)    │   │
│   └───────────────────────────────────┘    └───────────────────────────────────────┘   │
└────────────────────────────────────────────────────────────────────────────────────────┘
```

---

## 2. The Two-Gate Causal Verification State Machine

ReplayFix models bug reproduction and repair as a strict finite-state machine (`Phase`: `Init -> Gate1Red -> Repair -> Gate2Green -> Verified` or `Rejected`):

```text
              [Unpatched Target Repo]
                        │
                        ▼
          [Ephemeral ScratchSandbox Clone]
          (Δrepo = 0, Kernel Port :0)
                        │
                        ▼
              Gate 1: Live Browser Probe
                        │
           ┌────────────┴────────────┐
           ▼                         ▼
    [Assertion Passes]        [Assertion Fails (RED)]
    Verdict: GREEN/INVALID    (Runtime Error OR jitter_score_bps > 500)
    Abort: GATE1_NOT_RED             │
    (Exit Code 10)                   ▼
                          [SHA-256 TestFreezeGuard]
                                     │
                                     ▼
                          [Meta Muse Bounded Repair]
                          (Up to 3 attempts, 24h prefix cache)
                                     │
                                     ▼
                          [BlockPatcher + SyntaxPreflight]
                                     │
                                     ▼
                          Gate 2: 2x Consecutive Live Runs
                                     │
                        ┌────────────┴────────────┐
                        ▼                         ▼
                 [Run 1 & Run 2 GREEN]     [Any Run RED/Invalid]
                 (0 Errors & jitter<=500)  Backtrack with encrypted_content
                        │                  or Abort Exit Code 21/30
                        ▼
              [Differential Diagnosis & Parity]
                        │
                        ▼
              [RFC 6962 Merkle Receipt + Ed25519 Seal]
```

### Gate 1 (Counterfactual RED)
Before any model repair call is permitted, ReplayFix must prove the defect actually manifests in a live Chromium browser against the unpatched code. Gate 1 is classified as **RED** if and only if:
- `SETUP`, `TARGET`, and `ACTION` succeed (`passed: true`), and
- `ASSERT` fails (`passed: false`) due to a captured runtime exception (`pageerror` / `console.error`), HTTP `>= 500` response, or physical drag bounding-box oscillation (`jitter_score_bps > 500`).

### Gate 2 (2-Run Consecutive Stability GREEN)
After `BlockPatcher::apply` and `SyntaxPreflight::check_file` succeed in the scratch sandbox, ReplayFix reloads the live browser page and executes the exact same grounded interaction **twice consecutively** (`GATE2_REQUIRED_CONSECUTIVE_RUNS = 2`). Both runs must pass every stage (`SETUP`, `TARGET`, `ACTION`, `ASSERT`) with zero runtime errors and `jitter_score_bps <= 500`.

---

## 3. Universal Zero-Shot Component Harness (`discovery.rs`)

Real-world open-source repositories fall into two categories:
1. **Runnable Web Applications** with a self-contained dev server and `index.html`.
2. **UI Component Libraries** (e.g., `react-resizable`, `react-date-picker`) that export raw ES6, Flow (`// @flow`), or TypeScript/JSX components under `lib/` or `src/` without a pre-built HTML bundle.

ReplayFix v3.0.0 handles both automatically:
- **HTML Asset Validation (`html_assets_exist`)**: Scans candidate HTML files (`examples/*.html`, `demo/*.html`, `index.html`) and checks whether referenced local `<script src="...">` bundles actually exist on disk. Unbuilt templates (such as `examples/1.html` referencing a missing `bundle.js`) are automatically skipped.
- **In-Browser Babel Transpilation (`['env', 'flow', 'react']`)**: When no runnable HTML entry exists, `synthesize_component_harness_html` generates a zero-config HTML harness that loads React 16, ReactDOM 16, and `@babel/standalone` 7, strips Flow type imports/exports, and transpiles source modules in-browser via a recursive CommonJS `dynamicRequire` loader.
- **Offset-Parent Drag Physics (`SelfContainedDraggableCore`)**: Provides a self-contained `DraggableCore` implementation that computes pointer coordinates relative to `node.offsetParent.getBoundingClientRect()` and assigns unique ARIA slider roles/labels (`role="slider"`, `aria-label="Resize Handle NW"`). When an unpatched component shifts its parent `left`/`top` during top-left (`nw`/`n`/`w`) resizing without compensating `deltaX`/`deltaY` for `node.getBoundingClientRect()` shifts, the harness faithfully reproduces the physical bounding-box oscillation in Chromium.

---

## 4. Spatial Perception, Drag Trajectories & Physical Jitter (`perception.rs`, `browser.rs`)

### Multi-Frame Optical-Flow Trajectory Extraction
- FFmpeg extracts dense 100ms RGB24 frames across the video timeline.
- `RoiDiffer::pair_change` computes consecutive frame-pair pixel diffs across $120 	imes 120$ regions of interest (ROI) to identify both the initial interaction hotspot `(click_x, click_y)` and the terminal drag coordinates `(drag_end_x, drag_end_y)`.
- `sam-3.1` is queried over HTTPS (`https://api.meta.ai/v1/responses`). If the upstream endpoint returns empty segmentation text, ReplayFix transparently falls back to `segment_from_frame_pair` (`local_diff_heuristic`) and marks `sam_spatial_grounding: HEURISTIC` in the receipt's `StageExecutionMatrix`.

### Zero-Positional-Index DOM Grounding (`PROBE_PAGE`)
- At the calibrated CSS coordinates `(x, y)`, the Playwright bridge inspects `document.elementFromPoint(x, y)`, ensures unnamed `[class*="handle"]` elements carry semantic `role="slider"` and axis-derived `aria-label` attributes (`Resize Handle NW`), and captures `page.locator('body').ariaSnapshot()`.
- `LocatorResolver::resolve` scores every interactive candidate and requires a strict top-2 score margin $\ge 0.25$ (`2500 bps`). Positional `.first()` / `.nth()` disambiguation is prohibited by constitutional invariant.

### Physical Bounding-Box Jitter Oracle (`DRAG_AND_OBSERVE`)
During a drag interaction from `(from_x, from_y)` to `(to_x, to_y)`:
1. The Playwright bridge moves the mouse across 10 linear interpolation steps between `mousedown` and `mouseup`.
2. After every step, it samples `targetLocator.boundingBox()` (`{ x, y, width, height }`).
3. It computes inter-step displacement vectors $(\Delta x_i, \Delta y_i)$ and frame-to-frame acceleration spikes:
   $$	ext{accel}_i = |\Delta x_i - \Delta x_{i-1}| + |\Delta y_i - \Delta y_{i-1}|$$
4. Any step with $	ext{accel}_i > 2	ext{px}$ or sign reversal against the drag direction increments `jitter_score_bps`. A score $> 500	ext{ bps}$ fails `ASSERT` (reproducing Gate 1 RED on uncompensated resize handlers), while smooth resizing after repair yields `jitter_score_bps == 0` (passing Gate 2 GREEN).

---

## 5. Execution Authenticity & Forensic Matrix (`StageExecutionMatrix`)

To give judges and auditors complete transparency into what ran live vs. heuristically, every `AuditReceipt` embeds a 9-stage `stage_classification` matrix:

| Stage Field | `live-ai` with Video | Description |
| :--- | :---: | :--- |
| `video_ingestion` | `REAL` | Real FFmpeg probe and keyframe extraction from the input `.mp4`. |
| `sam_spatial_grounding` | `REAL` or `HEURISTIC` | `REAL` when `sam-3.1` returns wire tokens; `HEURISTIC` when `local_diff_heuristic` optical flow is used. |
| `interaction_trajectory` | `REAL` | Multi-step click/drag trajectory extracted from real video frames. |
| `deliberation_agent_loop` | `REAL` | Authenticated HTTPS reasoning via Meta AI (`muse-spark-1.3`). |
| `code_patch_synthesis` | `REAL` | Live structured JSON patch generated by `muse-spark-1.3` / `muse-wonder-1.0`. |
| `gate1_baseline` | `REAL` | Real Playwright Chromium CDP reproduction confirming Gate 1 RED. |
| `gate2_stability` | `REAL` | Real 2-run consecutive Playwright Chromium CDP verification confirming Gate 2 GREEN. |
| `gate3_stress_quorum` | `REAL` or `SKIPPED` | `REAL` when `--stress-quorum` (3G / 500ms latency perturbation) is enabled; `SKIPPED` otherwise. |
| `cryptographic_attestation` | `REAL` | RFC 6962 Merkle root signed with an RFC 8032 Ed25519 keypair. |
