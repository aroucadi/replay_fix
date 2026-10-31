# Implementation Plan: Spec 120 — Realtime Multimodal Perception (v1.2.0)

## 1. Module Architecture & Changes

1. **`crates/replayfix-core/src/gateway.rs`**:
   - Define `SamWirePayload` and `AsrPayload`.
   - Implement `MetaGatewayClient::segment_keyframe_sam31(&self, png_bytes: &[u8], cand_x: i32, cand_y: i32) -> Result<(String, u64)>` dispatching base64 PNG images to `POST https://api.meta.ai/v1/responses` with `model = "sam-3.1"`.
   - Implement `MetaGatewayClient::transcribe_audio_wav(&self, wav_bytes: &[u8]) -> Result<NarrationCue>` validating 24 kHz mono 16-bit PCM WAV headers and dispatching to `POST https://api.meta.ai/v1/asr/transcribe` with `model = "muse-voice-transcribe-1.0"`.
2. **`crates/replayfix-core/src/perception.rs` & `video.rs`**:
   - Define `ActionType` (`Click`, `Input`, `Navigate`), `UserAction` (`t_ms`, `action_type`, `sam_snap`, `narration_cue`, `latency_ms`), and `ActionTrajectory` (`actions: Vec<UserAction>`).
   - Add `RoiDiffer::detect_trajectory` in `video.rs` to detect ordered multi-step interaction events across the recording timeline.
   - Implement `SamSegmenter::segment_live` and `AsrTranscriber::transcribe_live` in `perception.rs`, plus `VideoTelemetry::ingest_with_mode` supporting both `live-ai` (via `MetaGatewayClient`) and `deterministic-fixture`.
3. **`crates/replayfix-core/src/repair.rs` & `engine.rs`**:
   - Wire `RepairClientMode::LiveEndpoint` in `BoundedRepairClient::generate` to `MetaGatewayClient::send_live_https` so `--mode live-ai` never falls back to static in-memory diffs.
4. **`crates/replayfix-core/src/locator.rs` & `browser.rs`**:
   - Enforce CDP `document.elementFromPoint` hit-testing on the SAM-snapped CSS centroid, rejecting missing or ambiguous hits with `AMBIGUOUS_GROUNDING` (Exit Code `22`).
