# 📋 Spec 120: Vertical Slice v1.2.0 — Realtime Multimodal Perception (Live Meta SAM 3.1, Muse Voice ASR & Multi-Action Trajectory)

**Feature Identity**: `120-v1.2.0-real-perception-sam-asr`  
**Target Release**: `v1.2.0`  
**Git Branch**: `release/v1.2.0-perception-sam-voice` / `main`  
**Status**: Approved for Implementation  

---

## 1. Problem Statement & Zero-Simulation Mandate

In `v1.0.0`–`v1.1.0`, the perception and repair layers retained local simulation fallbacks when `--mode live-ai` was requested:
1. `SamSegmenter::segment_from_frame_pair` used a local CPU grayscale difference loop (`abs_diff > 16`) instead of calling `POST https://api.meta.ai/v1/responses` with model `sam-3.1`.
2. `AsrTranscriber::align_wav_narration` validated the WAV header and formatted a synthetic SSE string instead of calling `POST https://api.meta.ai/v1/asr/transcribe` with model `muse-voice-transcribe-1.0`.
3. `BoundedRepairClient::generate` did not dispatch live HTTPS requests via `MetaGatewayClient` when `RepairClientMode::LiveEndpoint` was selected.
4. Video ingestion only captured a single isolated interaction point rather than an ordered multi-step `ActionTrajectory`.

**Release `v1.2.0` eliminates all simulation in `--mode live-ai` and delivers the multi-action trajectory engine.**

---

## 2. User Stories & Priorities

- **User Story 1 (P1 — Live SAM 3.1 Wire Snapping)**: As a visual reproduction engine, I dispatch extracted click keyframe PNG crops to `POST https://api.meta.ai/v1/responses` with model `sam-3.1`, parse native wire tokens (`<|box;x1=...;y1=...;x2=...;y2=...|>` and `<|mask;...|>`), calculate the exact mask centroid, and project coordinates into live CDP logical CSS space via `GeometricCalibration`.
- **User Story 2 (P2 — Live Muse Voice 24kHz ASR Alignment)**: As a multimodal intake pipeline, I extract 24 kHz mono 16-bit PCM WAV audio from the input MP4 via FFmpeg, dispatch to `POST https://api.meta.ai/v1/asr/transcribe` with model `muse-voice-transcribe-1.0`, and anchor spoken transcripts using `audioProcessedMs` ($t = \frac{	ext{audioProcessedMs}}{1000}$).
- **User Story 3 (P3 — Multi-Step Interaction Trajectory Engine)**: As a test generation feeder, I capture an ordered sequence of interaction frames into an `ActionTrajectory` struct (`Vec<UserAction>` with timestamp `t_ms`, coordinates, `SamSnapResult`, `ActionType`, and optional `NarrationCue`), enabling downstream multi-step Playwright test synthesis.
- **User Story 4 (P4 — Fail-Closed Ambiguity Guard)**: When SAM 3.1 returns zero contours, degenerate bounding boxes, or contour score margin ties ($< 2500	ext{ bps}$), or when CDP `document.elementFromPoint` finds no unambiguous interactive element, ReplayFix fails closed immediately with Exit Code `22` (`AMBIGUOUS_GROUNDING`).
- **User Story 5 (P5 — Telemetry & Cryptographic Receipt)**: `audit_receipt.json` records actual wire latency (`sam_latency_ms`), `ActionTrajectory`, bounding boxes, and transcript metadata under `video_telemetry`, and only stamps `provenance = "live-ai"` when live HTTPS inference succeeded.

---

## 3. Normative Functional Requirements

- **REQ-120-01 (Live `sam-3.1` HTTPS Dispatch)**: `MetaGatewayClient::segment_keyframe_sam31` and `SamSegmenter::segment_live` MUST base64-encode keyframe PNG crops, dispatch to `POST https://api.meta.ai/v1/responses` (`model: "sam-3.1"`), parse `<|box;x1=...;y1=...;x2=...;y2=...|><|mask;...|>` tokens, and snap to the calibrated CSS centroid.
- **REQ-120-02 (Live `muse-voice-transcribe-1.0` HTTPS Dispatch)**: `MetaGatewayClient::transcribe_audio_wav` and `AsrTranscriber::transcribe_live` MUST validate the 24 kHz mono 16-bit PCM WAV header (`validate_wav_audio`), enforce the Zero-Audio Rule (Law VI), dispatch to `POST https://api.meta.ai/v1/asr/transcribe`, and parse cumulative `audioProcessedMs` markers.
- **REQ-120-03 (Multi-Step `ActionTrajectory`)**: `RoiDiffer::detect_trajectory` and `VideoTelemetry` MUST extract ordered multi-step interactions (`Vec<UserAction>`) across the video timeline with `ActionType` (`Click`, `Input`, `Navigate`), `SamSnapResult`, and aligned `NarrationCue`.
- **REQ-120-04 (End-to-End Live Repair Wiring)**: `BoundedRepairClient::generate` in `RepairClientMode::LiveEndpoint` MUST call `MetaGatewayClient::send_live_https` so `--mode live-ai` executes real HTTPS inference end-to-end and fails closed if unreachable.
- **REQ-120-05 (CDP `elementFromPoint` Centroid Verification)**: `LocatorResolver` and `PlaywrightBridge` MUST verify that the SAM-snapped CSS centroid lands on an unambiguous interactive DOM element or fail closed with `AMBIGUOUS_GROUNDING` (Exit Code `22`).
