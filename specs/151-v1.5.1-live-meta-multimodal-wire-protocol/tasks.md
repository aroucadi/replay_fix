# Tasks: Spec 151 — Live Meta Multimodal Wire Protocol & Full Muse Model Utilization (Release 1.5.1)

**Input**: Design documents from `/specs/151-v1.5.1-live-meta-multimodal-wire-protocol/`  
**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md), [data-model.md](./data-model.md)

## Phase 1: Setup & Foundational Wire Types

- [x] T001 Extend `GatewayRequest` and `GatewayResponse` in `crates/replayfix-core/src/gateway.rs` with `enable_web_search`, `images_processed`, `video_frames_processed`, and `tool_calls_count`

## Phase 2: User Story 1 — Live SAM 3.1 Image & Video Frame Segmentation (Priority: P1)

- [x] T002 [US1] Update `MetaGatewayClient::send_live_https_with_image`, `segment_keyframe_sam31`, and add `segment_video_sam31` in `crates/replayfix-core/src/gateway.rs` to use `input_image` and `input_video` content blocks
- [x] T003 [US1] Update `SamSegmenter::parse_wire_tokens` and `VideoTelemetry::ingest_with_mode` in `crates/replayfix-core/src/perception.rs` to support 6-attribute `<|box;x1=...;y1=...;x2=...;y2=...;w=...;h=...|>` tokens, multi-box frames, and live video segmentation

## Phase 3: User Story 2 — Live Muse Voice Transcribe 1.0 Multipart WAV ASR (Priority: P1)

- [x] T004 [US2] Update `MetaGatewayClient::transcribe_audio_wav` in `crates/replayfix-core/src/gateway.rs` to build an RFC 7578 `multipart/form-data` payload (`request` JSON + `audio` WAV part)
- [x] T005 [US2] Update `AsrTranscriber::parse_asr_events` in `crates/replayfix-core/src/perception.rs` to parse live `/v1/asr/transcribe` responses (`audioDurationMs`, `turns`, and silent audio tracks)

## Phase 4: User Story 3 & 4 — Live Muse Spark/Wonder Tool Calling & Muse Image 1.0 Badge (Priority: P1/P2)

- [x] T006 [US3] Enable `"tools": [{"type": "web_search"}]` and multimodal keyframe `input_image` support in `crates/replayfix-core/src/repair.rs` and `crates/replayfix-core/src/gateway.rs`
- [x] T007 [US4] Implement `MetaGatewayClient::generate_verification_badge` (`POST /v1/images/generations` with `muse-image-1.0`) in `crates/replayfix-core/src/gateway.rs` and wire into `VideoTelemetry::ingest_with_mode`

## Phase 5: Verification, Live-AI Validation & Release 1.5.1 Packaging

- [x] T008 Add unit and integration tests for the v1.5.1 live wire schemas in `crates/replayfix-core/src/gateway.rs` and `crates/replayfix-core/src/perception.rs`
- [x] T009 Execute live `--mode live-ai` end-to-end validation against `https://api.meta.ai/v1/` and verify all 5 Meta models succeed with HTTP 200
- [x] T010 Bump workspace version to `1.5.1`, update `CHANGELOG.md`, `README.md`, `docs/release-notes/v1.5.1.md`, build and sign `release/v1.5.1/`, merge to `main`, tag `v1.5.1`, and push to `origin`
