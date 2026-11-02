# Feature Specification: Spec 151 — Live Meta Multimodal Wire Protocol & Full Muse Model Utilization (Release 1.5.1)

**Feature Branch**: `release/v1.5.1`  
**Created**: 2026-11-01  
**Status**: Ready for Planning & Implementation  
**Input**: Align ReplayFix's `--mode live-ai` cloud gateway and perception/repair engines with the exact production wire protocols of `https://api.meta.ai/v1/` so every live run exercises the full Meta Muse & SAM model suite across all five dashboard telemetry dimensions (`Video frames segmented`, `Images segmented`, `Audio transcription seconds`, `Tool calls`, and `Images` + token usage).

---

## User Scenarios & Testing *(mandatory)*

### User Story 1 — Live SAM 3.1 Image & Video Frame Segmentation (Priority: P1)

As a ReplayFix engineer running `--mode live-ai` against a bug recording MP4, I want ReplayFix to send both extracted keyframe PNG crops (`input_image`) and the recording video stream (`input_video`) to `POST https://api.meta.ai/v1/responses` with model `sam-3.1` using the production multimodal content array schema, and parse the full `<|box;x1=...;y1=...;x2=...;y2=...;w=...;h=...|><|mask;...|>` wire token grammar, so that both **Images segmented** (`images_processed`) and **Video frames segmented** (`video_frames_processed`) are genuinely executed and recorded in `audit_receipt.json`.

**Why this priority**: Real visual grounding via SAM 3.1 is the foundational spatial perception layer of ReplayFix.

**Independent Test**: Run `replayfix repro --bug RF-01 --mode live-ai --live-browser` with a valid `META_API_KEY` and verify that `audit_receipt.json` records `video_telemetry.sam_snap.model == "sam-3.1"`, `images_processed >= 1`, `video_frames_processed >= 1`, and calibrated CSS coordinates.

**Acceptance Scenarios**:

1. **Given** a keyframe PNG and source MP4 in `--mode live-ai`, **When** `MetaGatewayClient` calls `POST https://api.meta.ai/v1/responses` with `model: "sam-3.1"`, **Then** the request MUST format `input` as a multimodal message array containing `{"type": "input_image", "image_url": "data:image/png;base64,..."}` (and `{"type": "input_video", "video_url": "data:video/mp4;base64,..."}` for video segmentation) and `{"type": "input_text", "text": "..."}` without legacy top-level `image_base64` fields.
2. **Given** `sam-3.1` returns wire tokens with optional width/height attributes (`<|box;x1=1172;y1=596;x2=1238;y2=622;w=1280;h=720|><|mask;x=0;y=0;data=...|>`), **When** `SamSegmenter::parse_wire_tokens` parses the output, **Then** it MUST extract all bounding boxes and masks accurately and snap to the nearest target contour around the interaction hotspot.

---

### User Story 2 — Live Muse Voice Transcribe 1.0 Multipart WAV ASR (Priority: P1)

As a ReplayFix user running `--mode live-ai`, I want 24 kHz mono 16-bit PCM WAV audio extracted from the bug recording to be transmitted to `POST https://api.meta.ai/v1/asr/transcribe` as a standard RFC 7578 `multipart/form-data` payload containing both the `request` JSON part (`{"model":"muse-voice-transcribe-1.0","audioEncoding":"WAV"}`) and the `audio` WAV binary part, so that spoken narration and `audioDurationMs` are transcribed live by Meta's ASR engine (`Audio transcription seconds`).

**Why this priority**: Ensures live voice narration alignment works against the production Voyager/Muse ASR endpoint without `HTTP 415 Unsupported Media Type` or `HTTP 400 Malformed request` errors.

**Independent Test**: Invoke `MetaGatewayClient::transcribe_audio_wav` against `https://api.meta.ai/v1/asr/transcribe` with a valid 24 kHz mono WAV buffer and verify HTTP 200 and a parsed `NarrationCue` with `model == "muse-voice-transcribe-1.0"` and `audio_processed_ms > 0`.

**Acceptance Scenarios**:

1. **Given** a 24 kHz mono WAV buffer in `--mode live-ai`, **When** `transcribe_audio_wav` dispatches to `/v1/asr/transcribe`, **Then** it MUST send `multipart/form-data` with field `request` (`{"model":"muse-voice-transcribe-1.0","audioEncoding":"WAV"}`) and file part `audio` (`audio.wav`, `audio/wav`).
2. **Given** the live endpoint returns `{"sessionId":"...","transcript":"...","audioDurationMs":1040,"turns":[...]}`, **When** `AsrTranscriber::parse_asr_events` processes the response, **Then** it MUST map `audioDurationMs` (when `audioProcessedMs` is absent) and handle silent audio tracks (`transcript: ""`) gracefully without failing the run.

---

### User Story 3 — Live Muse Spark 1.3 & Muse Wonder 1.0 Multimodal Vision + Web Search Tool Calling (Priority: P1)

As an autonomous repair harness running in `--mode live-ai`, I want `muse-spark-1.3` and `muse-wonder-1.0` requests on `POST https://api.meta.ai/v1/responses` to attach `"tools": [{"type": "web_search"}]` (and multimodal keyframe `input_image` context when available) and parse tool-augmented responses (`output` items of type `web_search_call` / `function_call` alongside `message`), so that the models leverage Meta's native tool-calling capabilities (`Tool calls` on the Meta dashboard) and visual context during causal diagnosis and patch synthesis.

**Why this priority**: Maximizes reasoning quality and ensures tool-calling capabilities of Muse Spark 1.3 and Muse Wonder 1.0 are actively utilized.

**Independent Test**: Run a live `MetaGatewayClient` request with `enable_web_search: true` on `muse-spark-1.3` / `muse-wonder-1.0` and verify HTTP 200 and recorded `tool_calls_count`.

**Acceptance Scenarios**:

1. **Given** a live repair or diagnostic request in `--mode live-ai`, **When** `MetaGatewayClient` dispatches to `/v1/responses`, **Then** it MUST include `"tools": [{"type": "web_search"}]` when tool calling is enabled on the request and count any `web_search_call` / `function_call` items in the response.

---

### User Story 4 — Live Muse Image 1.0 Verification Attestation Badge (`muse-image-1.0`) (Priority: P2)

As a developer or judge inspecting a `--mode live-ai` verification bundle, I want ReplayFix to call `POST https://api.meta.ai/v1/images/generations` with `model: "muse-image-1.0"` to synthesize a visual attestation badge (`muse_verification_badge.webp`) stored alongside the extracted keyframes, so that the `muse-image-1.0` image generation model (`Images` on the Meta dashboard) is leveraged as part of the live multimodal proof pipeline.

**Why this priority**: Completes 100% coverage of all models in the Meta Model API catalog (`muse-wonder-1.0`, `muse-spark-1.3`, `sam-3.1`, `muse-voice-transcribe-1.0`, and `muse-image-1.0`).

**Independent Test**: Call `MetaGatewayClient::generate_verification_badge` in `--mode live-ai` and verify that `.replayfix/keyframes/muse_verification_badge.webp` is written and non-empty.

---

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001 (Multimodal `input_image` & `input_video` Wire Schema)**: `MetaGatewayClient` MUST format image and video inputs to `POST https://api.meta.ai/v1/responses` as a structured `input` message array containing `{"type": "input_image", "image_url": "data:image/png;base64,..."}` and/or `{"type": "input_video", "video_url": "data:video/mp4;base64,..."}` plus `{"type": "input_text", "text": "..."}`.
- **FR-002 (Extended SAM 3.1 Token Parser)**: `SamSegmenter::parse_wire_tokens` MUST support both 4-attribute (`x1;y1;x2;y2`) and 6-attribute (`x1;y1;x2;y2;w;h`) `<|box;...|>` tokens as well as `<|mask;...|>` tokens, and select the contour closest to the optical-flow interaction centroid when multiple UI elements are segmented.
- **FR-003 (Multipart Form-Data ASR Wire Schema)**: `MetaGatewayClient::transcribe_audio_wav` MUST send an RFC 7578 `multipart/form-data` request to `POST /v1/asr/transcribe` with part `request` (`{"model":"muse-voice-transcribe-1.0","audioEncoding":"WAV"}`) and part `audio` (`audio.wav`), and `AsrTranscriber::parse_asr_events` MUST support both `audioProcessedMs` and `audioDurationMs` (including silent audio tracks).
- **FR-004 (Native Tool Calling on Muse Spark & Muse Wonder)**: `GatewayRequest` and `MetaGatewayClient` MUST support attaching `"tools": [{"type": "web_search"}]` on `muse-spark-1.3` and `muse-wonder-1.0` calls and recording `tool_calls_invoked`, `images_processed`, and `video_frames_processed` in `GatewayResponse` and `VideoTelemetry`.
- **FR-005 (Muse Image 1.0 Attestation Badge Generation)**: `MetaGatewayClient` MUST support calling `POST https://api.meta.ai/v1/images/generations` with `model: "muse-image-1.0"` during `--mode live-ai` runs to generate `.replayfix/keyframes/muse_verification_badge.webp`.

## Success Criteria *(mandatory)*

- **SC-001 (100% Live Meta Model API Modality Coverage)**: Running `replayfix repro --bug RF-01 --mode live-ai --live-browser` against `https://api.meta.ai/v1/` succeeds end-to-end with `HTTP 200` across `sam-3.1` (`input_image` + `input_video`), `muse-voice-transcribe-1.0` (`multipart/form-data` WAV), `muse-spark-1.3` / `muse-wonder-1.0` (with `web_search` tool enabled), and `muse-image-1.0` (`/v1/images/generations`).
- **SC-002 (Zero Regression & Strict Safety)**: All existing unit and integration tests plus new `v1.5.1` wire-protocol tests pass with `0` Clippy warnings and `0` non-test panics.
