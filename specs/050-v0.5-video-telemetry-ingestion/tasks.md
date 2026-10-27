# Tasks: 050-v0.5-video-telemetry-ingestion

- [x] **Task 5.1: Video Prober & Metadata Validator (`VideoPreprocessor` & `GeometricCalibration`)**
  - Implemented `replayfix_core::process::run_bounded` and `replayfix_core::video::{VideoPreprocessor, VideoMetadata, GeometricCalibration}`.
  - Verified with `cargo test -p replayfix-core process::` and `cargo test -p replayfix-core video::`.
- [x] **Task 5.2: Cursor-Bounded ROI Optical Flow Differencer (`RoiDiffer`)**
  - Implemented $120 \times 120\text{px}$ ROI $L_1$ frame differencing, $\tau$ variance threshold, and `< 2_500` bps fail-closed ambiguity guard.
  - Verified with `test_detects_single_interaction_centroid_and_time`, `test_dominant_interaction_beats_small_distractor`, `test_static_video_and_codec_noise_fail_closed`, and `test_two_equal_interactions_are_ambiguous`.
- [x] **Task 5.3: Meta Superpower #2 (`muse-voice-transcribe-1.0`) Integration (`AsrTranscriber`)**
  - Implemented 24 kHz mono 16-bit PCM WAV extraction (`PCM_24KHZ`), `ENDPOINTING` + `DELTA` + `emit_audio_progress: true` validation, cumulative `audioProcessedMs` alignment, and Zero-Audio enforcement blocking `muse-spark-1.3`.
  - Verified with `test_asr_forbids_muse_spark_for_audio` and `test_asr_parses_cumulative_audio_processed_ms_without_summing`.
- [x] **Task 5.4: Meta Superpower #1 (`sam-3.1`) Sub-Pixel Hotspot Snapping (`SamSegmenter`)**
  - Implemented `<|box;x1=...;y1=...;x2=...;y2=...|><|mask;...|>` wire-token parser, contour centroid calculation, `GeometricCalibration::to_css` projection, and `< 2_500` bps ambiguity abstention.
  - Verified with `test_sam_parse_wire_tokens_and_snap_to_centroid`, `test_sam_rejects_ambiguous_contour_tie_fail_closed`, and `test_sam_rejects_malformed_or_degenerate_wire_tokens`.
- [x] **Task 5.5: Connect Grounded Coords to CDP & End-to-End Verification**
  - Wired `VideoTelemetry::ingest` into `MockHealer::run_live_browser_in_repo`, bumped `AuditReceipt` schema to `"0.5"`, added `assets/fixtures/rf01_demo.mp4`, and verified `replayfix run --video assets/fixtures/rf01_demo.mp4 --repo .`.
  - Verified with `test_video_telemetry_ingest_end_to_end` and `live_browser_video_telemetry_ingests_keyframes_sam_and_asr`.
