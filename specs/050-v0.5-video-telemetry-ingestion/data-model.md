# Data Model: 050-v0.5-video-telemetry-ingestion

**Feature**: `050-v0.5-video-telemetry-ingestion`  
**Date**: 2026-10-27

---

## 1. `replayfix_core::video` Entities

### `VideoMetadata`
| Field | Type | Constraints | Description |
| :--- | :--- | :--- | :--- |
| `width` | `u32` | `> 0` | Source video width in pixels |
| `height` | `u32` | `> 0` | Source video height in pixels |
| `duration_ms` | `u64` | `> 0 && <= 90_000` | Video duration in milliseconds |
| `fps_milli` | `u32` | `> 0` | Frames per second $\times 1000$ |
| `has_audio` | `bool` | — | True if an audio stream is present |
| `container` | `String` | `.mp4` or `.webm` | Container format string from `ffprobe` |

### `GeometricCalibration`
| Field | Type | Constraints | Description |
| :--- | :--- | :--- | :--- |
| `video_width` | `u32` | `> 0` | Source video frame width |
| `video_height` | `u32` | `> 0` | Source video frame height |
| `viewport_width` | `u32` | `> 0` | Browser CSS viewport width |
| `viewport_height` | `u32` | `> 0` | Browser CSS viewport height |
| `dpr_milli` | `u32` | `> 0` (`1000` = `1.0`) | Browser `devicePixelRatio` $\times 1000$ |

### `InteractionEvidence`
| Field | Type | Constraints | Description |
| :--- | :--- | :--- | :--- |
| `t_ms` | `u64` | `<= duration_ms` | Detected interaction timestamp in ms |
| `x_video` | `i32` | `0..width` | Rough interaction X in video pixels |
| `y_video` | `i32` | `0..height` | Rough interaction Y in video pixels |
| `roi_changed_bps` | `u32` | `>= tau_bps` | Changed pixel share in $120 \times 120\text{px}$ ROI (bps) |
| `score_margin_bps` | `u32` | `>= 2_500` (`0.25`) | Score margin over runner-up cluster (bps) |

---

## 2. `replayfix_core::perception` Entities

### `SamSnapResult`
| Field | Type | Constraints | Description |
| :--- | :--- | :--- | :--- |
| `model` | `String` | `"sam-3.1"` | Serving segmentation model slug |
| `wire_token` | `String` | `<\|box;...\|><\|mask;...\|>` | Raw SAM 3.1 wire token |
| `box_x1`, `box_y1`, `box_x2`, `box_y2` | `i32` | `x1 < x2 && y1 < y2` | Contour bounding box in video pixels |
| `centroid_x_video`, `centroid_y_video` | `i32` | `((x1+x2)/2, (y1+y2)/2)` | Snapped contour centroid in video pixels |
| `calibrated_x_css`, `calibrated_y_css` | `i32` | Inside CSS viewport | Calibrated CSS logical coordinates |
| `candidate_score_bps` | `u32` | `0..=10_000` | Spatial containment + proximity score (bps) |
| `score_margin_bps` | `u32` | `>= 2_500` | Margin over runner-up contour (bps) |

### `NarrationCue`
| Field | Type | Constraints | Description |
| :--- | :--- | :--- | :--- |
| `model` | `String` | `"muse-voice-transcribe-1.0"` | Serving ASR model (never `muse-spark*`) |
| `transcript` | `String` | Non-empty | Transcribed developer narration |
| `audio_processed_ms` | `u64` | Cumulative marker | Cumulative `audioProcessedMs` from ASR stream |
| `aligned_video_time_ms` | `u64` | `<= duration_ms` | Video timeline lock in milliseconds |
| `audio_encoding` | `String` | `"PCM_24KHZ"` | 24 kHz mono signed 16-bit PCM |
| `mode` / `partial_mode` | `String` | `"ENDPOINTING"` / `"DELTA"` | Streaming endpointing configuration |

### `VideoTelemetry`
| Field | Type | Description |
| :--- | :--- | :--- |
| `video_metadata` | `VideoMetadata` | Container metadata probed via `ffprobe` |
| `interaction` | `InteractionEvidence` | ROI optical-flow interaction hotspot |
| `sam_snap` | `SamSnapResult` | SAM 3.1 contour centroid and calibrated CSS point |
| `narration` | `Option<NarrationCue>` | Temporally aligned spoken narration cue |
| `keyframe_paths` | `Vec<String>` | Extracted keyframe PNG paths ($\le 10$) |
