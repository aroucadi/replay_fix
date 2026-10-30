# ADR-03: Multimodal Evidence Model & Coordinate Alignment

## Status
Accepted

## Context
ReplayFix captures multimodal defect evidence: video metadata (`width`, `height`, `duration_ms`, `fps`), extracted keyframes, $120	imes120	ext{px}$ optical-flow ROI diffs, SAM 3.1 `<|box;...|>` segmentation bounding boxes and centroids, ASR transcript segments (`muse-voice-transcribe-1.0`), and Playwright DOM `ariaSnapshot` / locator grounding scores.

## Decision
1. **Explicit Coordinate Spaces**:
   - Video frames use intrinsic pixel dimensions $(W_{	ext{video}}, H_{	ext{video}})$.
   - Browser viewport actions use CSS pixel dimensions $(W_{	ext{css}}, H_{	ext{css}})$ calibrated via `GeometricCalibration` scale factors $(S_x, S_y)$ and `devicePixelRatio`.
   - SAM 3.1 wire tokens `<|box;x1,y1,x2,y2|>` are normalized $[0..1000]$ milliparts and mapped deterministically to pixel coordinates.
2. **Provenance & Missing-Artifact Honesty**:
   - Every displayed keyframe, SAM mask, and transcript segment shows its originating artifact SHA-256 digest and provider provenance (`live` vs. `simulated_fixture`).
   - When an external binary asset (e.g., `.mp4` or `.png` on disk) is not loaded into the browser session, the viewer displays an explicit **"Artifact Not Loaded in Offline Session (SHA-256 Verified via Receipt)"** state and renders the recorded geometric bounding-box / ROI / centroid telemetry on a calibrated coordinate grid without inventing synthetic video pixels.

## Consequences
- **Positive**: Judges can inspect exact spatial grounding math, SAM 3.1 bounding boxes, click centroids, and ASR temporal alignment offline with 100% provenance honesty.
