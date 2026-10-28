# Tasks: 050-v0.5-video-telemetry-ingestion — Multimodal Video Telemetry, SAM 3.1 Snapping & ASR Alignment

**Spec**: [spec.md](spec.md) | **Plan**: [plan.md](plan.md) | **Data Model**: [data-model.md](data-model.md)
**Rule**: The workspace must compile and all tests must pass after every task.

---

## Phase 1: Setup & Bounded Subprocess Supervision

- [x] T001 Bump workspace crates to `0.5.0` in `crates/replayfix-core/Cargo.toml` and `crates/replayfix-cli/Cargo.toml`
- [x] T002 Implement `BoundedOutput` and `run_bounded` (wall-clock deadline, stdout cap, stderr tail ring drain, `Platform::terminate_process_tree` on timeout/overflow) in `crates/replayfix-core/src/process.rs`
- [x] T003 [P] Add unit tests for `run_bounded` (stdout capture, stdin delivery, non-zero exit reporting, oversized stdout rejection, and hung process timeout kill) in `crates/replayfix-core/src/process.rs`

---

## Phase 2: User Story 1 — Video Probing, Geometric Calibration & Cursor-Bounded ROI Differencing (Priority: P1) 🎯 MVP

- [x] T004 [P] [US1] Implement `VideoMetadata`, `GeometricCalibration::to_css`, and `VideoPreprocessor` (`validate_source`, `probe_video`, `extract_gray_frames`, `extract_png_frame`, `write_keyframes` capped at `MAX_KEYFRAMES = 10`, `extract_wav_24k_mono`, `synthesize_demo_video`) in `crates/replayfix-core/src/video.rs`
- [x] T005 [US1] Implement `DetectionParams`, `InteractionEvidence`, and `RoiDiffer::detect_interaction` ($120 \times 120\text{px}$ ROI $L_1$ frame differencing, noise floor $\tau$, and `< 2_500` bps fail-closed ambiguity guard) in `crates/replayfix-core/src/video.rs`
- [x] T006 [US1] Add unit tests in `crates/replayfix-core/src/video.rs` covering DPR/letterbox calibration, zero-geometry rejection, ROI interaction detection, distractor suppression, ambiguity tie rejection, keyframe budget enforcement, and live FFmpeg MP4 synthesis/probing

---

## Phase 3: User Story 2 — Meta Superpower #2 (`muse-voice-transcribe-1.0`) & Zero-Audio Enforcement (Priority: P1)

- [x] T007 [P] [US2] Implement `AsrRequestConfig`, `NarrationCue`, and `AsrTranscriber` (`assert_model_permitted_for_audio` blocking `muse-spark*` with `CoreError::SecurityViolation`, `validate_config` for `PCM_24KHZ` / `ENDPOINTING` / `DELTA` / `emit_audio_progress: true`, cumulative `audioProcessedMs` parsing, and `align_wav_narration`) in `crates/replayfix-core/src/perception.rs`
- [x] T008 [US2] Add unit tests `test_asr_forbids_muse_spark_for_audio` and `test_asr_parses_cumulative_audio_processed_ms_without_summing` in `crates/replayfix-core/src/perception.rs`

---

## Phase 4: User Story 3 — Meta Superpower #1 (`sam-3.1`) Sub-Pixel Hotspot Snapping & End-to-End CDP Replay (Priority: P1)

- [x] T009 [P] [US3] Implement `SamContourBox`, `SamSnapResult`, `SamSegmenter` (`parse_wire_tokens` for `<|box;...|><|mask;...|>`, `snap_to_centroid`, `< 2_500` bps ambiguity guard, `segment_from_frame_pair`), and `VideoTelemetry::ingest` in `crates/replayfix-core/src/perception.rs`
- [x] T010 [US3] Add unit tests `test_sam_parse_wire_tokens_and_snap_to_centroid`, `test_sam_rejects_ambiguous_contour_tie_fail_closed`, `test_sam_rejects_malformed_or_degenerate_wire_tokens`, and `test_video_telemetry_ingest_end_to_end` in `crates/replayfix-core/src/perception.rs`
- [x] T011 [US3] Update `SCHEMA_VERSION` to `"0.5"` and add `video_telemetry: Option<VideoTelemetry>` to `AuditReceipt` in `crates/replayfix-core/src/receipt.rs`
- [x] T012 [US3] Wire `VideoTelemetry::ingest` into `MockHealer::run_live_browser_in_repo` in `crates/replayfix-core/src/engine.rs` and `crates/replayfix-cli/src/main.rs`, and generate deterministic fixture `assets/fixtures/rf01_demo.mp4`
- [x] T013 [US3] Add CLI integration test `live_browser_video_telemetry_ingests_keyframes_sam_and_asr` in `crates/replayfix-cli/tests/cli.rs`

---

## Phase 5: Polish, Quality Gates & Shipping

- [x] T014 Run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, and `cargo test --workspace` (138 tests passing)
- [x] T015 Complete SpecKit artifacts (`research.md`, `data-model.md`, `quickstart.md`, `checklists/requirements.md`, `contracts/`), update `CHANGELOG.md` and `docs/release-notes/v0.5.0.md`, and verify tag `v0.5.0`
