# Research & Technical Decisions: 050-v0.5-video-telemetry-ingestion

**Feature**: `050-v0.5-video-telemetry-ingestion`  
**Date**: 2026-10-27

---

## Decision 1: Bounded Subprocess Execution (`replayfix_core::process::run_bounded`)

- **Decision**: Wrap every `ffmpeg` and `ffprobe` invocation in `run_bounded`, which spawns background threads for `stdout` (capped at `max_stdout`) and `stderr` (capped at a 64 KiB tail ring buffer), polls child status until a wall-clock deadline, and calls `Platform::terminate_process_tree(pid)` on timeout or stdout overflow.
- **Rationale**:
  - Guarantees Constitutional Law I (Substrate Primacy) and Law IV (Child-Owned Ephemeral Provenance): no malformed media container or hung decoder can stall the harness or leak zombie `ffmpeg` processes on Windows.
- **Alternatives Considered**:
  - *Synchronous `Command::output()`*: Rejected because it has no wall-clock timeout and can allocate unbounded memory if a corrupt container emits infinite frames.

---

## Decision 2: Cursor-Bounded $120 \times 120\text{px}$ ROI $L_1$ Frame Differencing (`RoiDiffer`)

- **Decision**: Sample video frames at `SAMPLE_FPS = 10` (downscaled to `MAX_ANALYSIS_WIDTH = 640` with coordinates mapped back to source resolution), apply a per-pixel luma noise floor (`NOISE_FLOOR = 16`), cluster changed pixels into spatial-temporal events, and evaluate the changed-pixel share inside a $120 \times 120\text{px}$ Region of Interest (`ROI_SIDE_PX = 120`) in integer basis points (`DEFAULT_TAU_BPS = 20`, i.e., $0.2\%$). Fail closed (`AMBIGUOUS_GROUNDING`, exit code `22`) if two distinct interaction clusters have score margin $< 2\,500$ bps ($|\Delta S| < 0.25$).
- **Rationale**:
  - Catches micro-interactions affecting $0.2\%\text{–}0.8\%$ of the ROI while ignoring H.264/VP9 quantization noise and full-screen background animations.

---

## Decision 3: Integer-Math / Float-Free Receipt Representation for SAM 3.1 & ASR

- **Decision**: Represent frame rates (`fps_milli`), device-pixel ratios (`dpr_milli`), ROI change shares (`roi_changed_bps`), candidate scores (`candidate_score_bps`, `score_margin_bps`), and aligned narration timestamps (`audio_processed_ms`, `aligned_video_time_ms`) strictly as integers in `VideoTelemetry`.
- **Rationale**:
  - Preserves 100% compatibility with RFC 8785 `canonical_json` (which rejects floating-point numbers) while retaining millisecond and $0.01\%$ basis-point precision.
