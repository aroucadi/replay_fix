# Feature Specification: 050-v0.5-video-telemetry-ingestion — Multimodal Video Telemetry, SAM 3.1 Snapping & ASR Alignment

**Feature Branch**: `release/v0.5.0-video-telemetry`
**Created**: 2026-10-27
**Status**: Implemented & Verified
**Target Release**: `v0.5.0`
**Input**: FFmpeg video container probing (`ffprobe`), cursor-bounded ROI ($120 \times 120\text{px}$) $L_1$ optical-flow frame differencing, geometric coordinate calibration across resolution/letterbox/DPR, Meta Superpower #1 (`sam-3.1`) `<|box;...|><|mask;...|>` wire-token centroid snapping, Meta Superpower #2 (`muse-voice-transcribe-1.0`) 24 kHz mono PCM `audioProcessedMs` alignment with strict Zero-Audio enforcement on `muse-spark-1.3`, and end-to-end CDP grounding integration.

---

## Scope Statement (read first)

v0.5 upgrades ReplayFix from static fixture coordinates to **multimodal video telemetry ingestion**. When `replayfix run --video assets/fixtures/rf01_demo.mp4 --repo .` (or `replayfix repro --video <path> --live-browser`) is invoked and the video file exists on disk, the harness:

1. Validates and probes the container with `ffprobe` (`VideoPreprocessor::probe_video`), verifying supported formats (`.mp4`, `.webm`, `.mov`), non-zero frame geometry, and duration $\le 120\text{s}$.
2. Decodes bounded analysis frames and runs the cursor-bounded ROI ($120 \times 120\text{px}$) $L_1$ optical-flow differencer (`RoiDiffer::detect_interaction`) to isolate the exact interaction timestamp $t_{\text{click}}$ and rough click hotspot $(x_{\text{raw}}, y_{\text{raw}})$, failing closed with `AMBIGUOUS_GROUNDING` (exit code `22`) if the video is static, noisy, or has competing interaction peaks within $|\Delta S| < 0.25$ (`< 2_500` bps).
3. Writes up to `MAX_KEYFRAMES = 10` keyframe PNGs to `.replayfix/keyframes/keyframe_<ms>ms.png`.
4. Extracts 24 kHz mono 16-bit PCM audio (`PCM_24KHZ`) and aligns developer narration using Meta's `muse-voice-transcribe-1.0` (`mode: "ENDPOINTING"`, `partial_mode: "DELTA"`, `emit_audio_progress: true`), locking timestamps to cumulative `audioProcessedMs` and strictly forbidding audio transmission to `muse-spark-1.3`.
5. Invokes Meta's `sam-3.1` segmentation wire-token parser (`<|box;x1=...;y1=...;x2=...;y2=...|><|mask;...|>`), snaps the interaction coordinate to the exact contour centroid $\left(\frac{x_1+x_2}{2}, \frac{y_1+y_2}{2}\right)$, maps the video coordinate to CSS logical pixels via `GeometricCalibration::to_css`, and feeds the calibrated `(x_css, y_css)` into the Playwright CDP `LocatorResolver` and two-gate verification loop.

---

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Video Probing, Geometric Calibration & Cursor-Bounded ROI Differencing (Priority: P1)

As a developer submitting a screen recording of a web UI bug, I want ReplayFix to validate the video container, calibrate video pixel coordinates into CSS viewport coordinates across letterbox/pillarbox padding and device-pixel-ratio (DPR), and detect the exact click timestamp and hotspot via cursor-bounded $120 \times 120\text{px}$ ROI frame differencing so that full-screen animations do not corrupt click localization.

**Independent Test**: `cargo test -p replayfix-core video::` verifies `VideoPreprocessor::probe_video`, `GeometricCalibration::to_css`, `RoiDiffer::detect_interaction`, `write_keyframes`, and 24 kHz mono WAV extraction.

**Acceptance Scenarios**:
1. **Given** a valid `.mp4` recording $\le 120\text{s}`, **When** `VideoPreprocessor::probe_video` runs, **Then** it extracts width, height, duration in ms, frame rate (`fps_milli`), audio presence, and container format via `ffprobe` under a bounded process deadline.
2. **Given** video dimensions, CSS viewport dimensions, letterbox offsets, and DPR (`dpr_milli`), **When** `GeometricCalibration::to_css` maps a video pixel `(x_video, y_video)`, **Then** it subtracts letterbox offsets, scales to CSS logical pixels, and rejects coordinates inside letterbox bars or outside the viewport.
3. **Given** sequential grayscale frames with a localized UI state transition, **When** `RoiDiffer::detect_interaction` evaluates $120 \times 120\text{px}$ ROI $L_1$ variance, **Then** it identifies the interaction frame timestamp `t_ms` and rough hotspot `(x_video, y_video)`, or fails closed (`AMBIGUOUS_GROUNDING`, exit code `22`) if variance is below $\tau$ or two distinct regions tie within `< 2_500` bps.

---

### User Story 2 - Meta Superpower #2 (`muse-voice-transcribe-1.0`) & Zero-Audio Enforcement (Priority: P1)

As a hackathon judge evaluating Meta model integration, I want ReplayFix to align spoken bug narration using `muse-voice-transcribe-1.0` over 24 kHz mono 16-bit PCM audio (`PCM_24KHZ`) with cumulative `audioProcessedMs` locking, while strictly blocking any audio payload from ever being routed to `muse-spark-1.3`.

**Independent Test**: `cargo test -p replayfix-core perception::` verifies `test_asr_forbids_muse_spark_for_audio` and `test_asr_parses_cumulative_audio_processed_ms_without_summing`.

**Acceptance Scenarios**:
1. **Given** an attempt to pass `"muse-spark-1.3"` to `AsrTranscriber::assert_model_permitted_for_audio`, **When** validated, **Then** the harness immediately rejects it with `CoreError::SecurityViolation`.
2. **Given** a stream of ASR delta events containing `audioProcessedMs`, **When** `AsrTranscriber::parse_asr_events` processes the events, **Then** it treats `audioProcessedMs` as a cumulative counter (never summing across events) and selects the narration cue closest to the visual click timestamp `t_ms`.

---

### User Story 3 - Meta Superpower #1 (`sam-3.1`) Sub-Pixel Hotspot Snapping & End-to-End CDP Replay (Priority: P1)

As an engineer running `replayfix run --video assets/fixtures/rf01_demo.mp4 --repo .`, I want ReplayFix to snap the rough ROI hotspot to the exact interactive element contour centroid via `sam-3.1` (`<|box;x1=...;y1=...;x2=...;y2=...|><|mask;...|>`), map the centroid to CSS coordinates, execute Gate 1 (RED) and Gate 2 (GREEN) in Chromium, and record the complete `video_telemetry` block in `.replayfix/audit_receipt.json` (schema `"0.5"`).

**Independent Test**: `cargo test --test cli live_browser_video_telemetry_ingests_keyframes_sam_and_asr` and `replayfix run --video assets/fixtures/rf01_demo.mp4 --repo . --headed`.

**Acceptance Scenarios**:
1. **Given** `sam-3.1` wire tokens `<|box;x1=100;y1=100;x2=220;y2=140|><|mask;rle:1200:120x40|>` and rough click `(158, 118)`, **When** `SamSegmenter::snap_to_centroid` runs, **Then** it computes the contour centroid `(160, 120)` and calibrated CSS coordinates `(160, 120)` with score margin $\ge 2\,500$ bps.
2. **Given** two competing `sam-3.1` contours around the click whose scores differ by `< 2_500` bps ($|\Delta S| < 0.25$), **When** `SamSegmenter::snap_to_centroid` evaluates them, **Then** it fails closed with `AMBIGUOUS_GROUNDING` (exit code `22`).
3. **Given** `replayfix run --video assets/fixtures/rf01_demo.mp4 --repo .`, **When** the command completes, **Then** `.replayfix/keyframes/keyframe_000400ms.png` and `.replayfix/audit_receipt.json` (with `schema_version: "0.5"` and populated `video_telemetry`) are emitted and the command exits `0` with $\Delta \text{repo} = 0$.

---

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: `replayfix_core::process::run_bounded` MUST execute external tools (`ffmpeg`, `ffprobe`) with an explicit wall-clock deadline (`recv_timeout`), stdout size cap, stderr ring drain, and automatic process-tree termination (`Platform::terminate_process_tree`) on timeout or overflow.
- **FR-002**: `replayfix_core::video::VideoPreprocessor` MUST validate video paths via `assert_safe_path`, probe stream metadata via `ffprobe`, cap keyframes at `MAX_KEYFRAMES = 10`, and extract 24 kHz mono 16-bit PCM WAV (`pcm_s16le`, `-ar 24000`, `-ac 1`).
- **FR-003**: `replayfix_core::video::GeometricCalibration` MUST transform video pixel coordinates into CSS viewport coordinates across resolution scaling, letterbox/pillarbox offsets, and DPR using integer math.
- **FR-004**: `replayfix_core::video::RoiDiffer` MUST compute $120 \times 120\text{px}$ cursor-bounded ROI $L_1$ frame differences and fail closed with `AMBIGUOUS_GROUNDING` when signal is below $\tau$ or competing peaks differ by `< 2_500` bps.
- **FR-005**: `replayfix_core::perception::AsrTranscriber` MUST enforce the Zero-Audio Rule (blocking `muse-spark-1.3`), validate `muse-voice-transcribe-1.0` configuration (`PCM_24KHZ`, `ENDPOINTING`, `DELTA`, `emit_audio_progress: true`), and align narration using cumulative `audioProcessedMs`.
- **FR-006**: `replayfix_core::perception::SamSegmenter` MUST parse `sam-3.1` `<|box;x1=...;y1=...;x2=...;y2=...|><|mask;...|>` wire tokens, compute contour centroids, enforce the `< 2_500` bps ambiguity guard, and map centroids through `GeometricCalibration::to_css`.
- **FR-007**: `AuditReceipt` (schema version `"0.5"`) MUST include `video_telemetry: Option<VideoTelemetry>` serialized in float-free RFC 8785 canonical JSON.

---

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: `cargo run -p replayfix-cli -- run --video assets/fixtures/rf01_demo.mp4 --repo .` extracts keyframes to `.replayfix/keyframes/`, aligns ASR narration, snaps the click centroid via `sam-3.1`, executes two-gate Chromium verification, writes `.replayfix/audit_receipt.json` (schema `"0.5"`), and exits `0` with $\Delta \text{repo} = 0$.
- **SC-002**: Zero `.unwrap()` and zero `.expect()` in non-test production code across `crates/*/src/*.rs`, zero orphaned `ffmpeg`/`ffprobe`/`node`/`chrome` processes, and 100% pass rate on `cargo clippy --workspace --all-targets --all-features -- -D warnings` and `cargo test --workspace`.
