# Quickstart Validation Guide: 050-v0.5-video-telemetry-ingestion

**Feature**: `050-v0.5-video-telemetry-ingestion`  
**Date**: 2026-10-27

---

## 1. Prerequisites

Ensure `replayfix doctor` reports `[OK]` for Git, Node.js, Playwright, and FFmpeg:

```powershell
cargo run -p replayfix-cli -- doctor
```

---

## 2. Run Multimodal Video Telemetry Ingestion & Live Browser Replay

Run the `RF-01` demo recording through FFmpeg probing, ROI keyframe extraction, ASR `audioProcessedMs` alignment, SAM 3.1 centroid snapping, and live Chromium two-gate verification:

```powershell
cargo run -p replayfix-cli -- run --video assets/fixtures/rf01_demo.mp4 --live-browser --headed
```

**Expected Outcome**:
- `VideoPreprocessor::probe_video` probes `assets/fixtures/rf01_demo.mp4` (`1280x720`, `1000ms`, `10 fps`, audio present).
- `RoiDiffer` isolates the interaction frame at `400ms` and writes `.replayfix/keyframes/keyframe_000400ms.png`.
- `AsrTranscriber` aligns the 24 kHz mono PCM narration cue (`muse-voice-transcribe-1.0` @ `400ms`).
- `SamSegmenter` snaps the rough hotspot `(158, 118)` to contour `<|box;x1=100;y1=100;x2=220;y2=140|><|mask;rle:1200:120x40|>` centroid `(160, 120)` -> CSS `(160, 120)`.
- Chromium grounds `page.getByRole('button', { name: 'Refresh Cart', exact: true })`, reproduces Gate 1 (`RED`), applies the sandbox patch, verifies Gate 2 (`GREEN`), confirms $\Delta \text{repo} = 0$, and writes `.replayfix/audit_receipt.json` (`schema_version: "0.5"`).

---

## 3. Run Unit & Integration Tests

```powershell
cargo test -p replayfix-core -- video
cargo test -p replayfix-core -- perception
cargo test -p replayfix-core -- process
cargo test --workspace
```
