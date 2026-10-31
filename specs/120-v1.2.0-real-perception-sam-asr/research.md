# Research: Spec 120 — Realtime Multimodal Perception (v1.2.0)

## 1. SAM 3.1 Wire Protocol & Centroid Snapping
- **Endpoint**: `POST https://api.meta.ai/v1/responses`
- **Model**: `sam-3.1`
- **Payload Schema**: Keyframe image crop encoded in base64 within an XML-delimited `<keyframe_crop>` tag along with prompt coordinates $(x, y)$.
- **Wire Token Grammar**:
  - Box tokens: `<|box;x1=(\d+);y1=(\d+);x2=(\d+);y2=(\d+)|>`
  - Mask tokens: `<|mask;(\d+),(\d+),(\d+),(\d+)|>`
- **Centroid Projection**:
  $$\bar{x} = \frac{x_1 + x_2}{2}, \quad \bar{y} = \frac{y_1 + y_2}{2}$$
  Mapped to CSS viewport coordinates via `Calibration`:
  $$x_{\text{css}} = \frac{\bar{x} - \text{offset}_x}{\text{dpr} \cdot \text{scale}_x}$$
- **Fail-Closed Thresholds**:
  - Empty contours, out-of-bounds bounding boxes, or margin between candidate contours $< 2500$ bps immediately yield Exit Code `22` (`AMBIGUOUS_GROUNDING`).

## 2. Meta Muse Voice Transcribe 1.0 (ASR)
- **Endpoint**: `POST https://api.meta.ai/v1/asr/transcribe`
- **Audio Specification**:
  - Sample Rate: $24\,000$ Hz
  - Channels: 1 (mono)
  - Bit Depth: 16-bit signed PCM
  - Format: WAV (RIFF/WAVE header verified before dispatch)
- **Temporal Alignment**:
  - Server emits streaming events or direct JSON containing `audioProcessedMs`.
  - Timestamp locked to video timeline:
    $$t_{\text{video}} = \frac{\text{audioProcessedMs}}{1000.0}$$
- **Law VI (Zero-Audio Rule Quarantine)**:
  - Audio bytes are quarantined exclusively within `AsrTranscriber`.
  - Audio payloads are NEVER passed to `muse-spark-1.3`, `muse-wonder-1.0`, or text repair models.

## 3. Multi-Step Trajectory Schema
- Tracks sequential interaction frames across video duration.
- Each `UserAction` records:
  - `action_id`: UUIDv4
  - `timestamp_ms`: Temporal offset from video start
  - `action_type`: `Click`, `Input`, `Navigate`, or `Scroll`
  - `coordinates`: CSS viewport $(x, y)$ snapped via SAM 3.1
  - `bounding_box`: Exact bounding box from SAM mask
  - `confidence_bps`: Confidence score in basis points
  - `target_element`: Optional DOM candidate resolved via CDP `elementFromPoint`
