# Implementation Plan: 050-v0.5-video-telemetry-ingestion

**Branch**: `release/v0.5.0-video-telemetry` | **Date**: 2026-10-27 | **Spec**: [spec.md](spec.md)

## Summary

Delivers Spec 050 (`v0.5.0-video-telemetry-ingestion`): bounded FFmpeg/FFprobe subprocess execution (`replayfix_core::process`), video container validation and metadata probing (`VideoPreprocessor`), geometric calibration across resolution/letterbox/DPR (`GeometricCalibration`), cursor-bounded $120 \times 120\text{px}$ ROI $L_1$ optical-flow frame differencing (`RoiDiffer`), Meta Superpower #2 (`muse-voice-transcribe-1.0`) 24 kHz mono PCM narration alignment (`AsrTranscriber`), Meta Superpower #1 (`sam-3.1`) `<|box;...|><|mask;...|>` wire-token centroid snapping (`SamSegmenter`), and end-to-end live Playwright CDP grounding in `MockHealer::run_live_browser_in_repo`.

## Architecture & Module Layout

1. **`crates/replayfix-core/src/process.rs`**:
   - `run_bounded(program, args, stdin_bytes, timeout, max_stdout_bytes)`: Spawns child subprocess with `Stdio::piped()`, background reader threads for `stdout` (capped at `max_stdout_bytes`) and `stderr` (tail ring buffer), `mpsc::Receiver::recv_timeout` wall-clock deadline, and `Platform::terminate_process_tree(pid)` on timeout or overflow.
2. **`crates/replayfix-core/src/video.rs`**:
   - `VideoPreprocessor`: `validate_source`, `probe_video` (`ffprobe` JSON stream inspection), `extract_gray_frames`, `extract_png_frame`, `write_keyframes` (`MAX_KEYFRAMES = 10`), `extract_wav_24k_mono` (`-vn -acodec pcm_s16le -ar 24000 -ac 1`), and `synthesize_demo_video`.
   - `GeometricCalibration`: Integer-math transformation from video pixel space to CSS logical pixel space accounting for letterbox/pillarbox offsets and DPR.
   - `RoiDiffer`: $120 \times 120\text{px}$ cursor-bounded ROI $L_1$ frame differencer with noise floor $\tau$ and fail-closed ambiguity guard (`< 2_500` bps).
3. **`crates/replayfix-core/src/perception.rs`**:
   - `SamSegmenter`: Parses `sam-3.1` `<|box;x1=...;y1=...;x2=...;y2=...|><|mask;...|>` tokens, scores contour boxes against the rough ROI click, enforces `< 2_500` bps ambiguity abstention, and snaps to the calibrated CSS centroid.
   - `AsrTranscriber`: Enforces Zero-Audio Rule (forbidding `muse-spark-1.3`), validates `muse-voice-transcribe-1.0` config (`PCM_24KHZ`, `ENDPOINTING`, `DELTA`, `emit_audio_progress: true`), and aligns narration via cumulative `audioProcessedMs`.
   - `VideoTelemetry`: Orchestrates `VideoPreprocessor`, `RoiDiffer`, `SamSegmenter`, and `AsrTranscriber` into a single deterministic telemetry record for `AuditReceipt` (schema `"0.5"`).
4. **`crates/replayfix-core/src/engine.rs` & `crates/replayfix-cli/src/main.rs`**:
   - Feeds `VideoTelemetry::ingest` calibrated CSS coordinates `(calibrated_x_css, calibrated_y_css)` into `execute_browser_gate` and `LocatorResolver::resolve`, and supports `replayfix run --video assets/fixtures/rf01_demo.mp4 --repo .`.
