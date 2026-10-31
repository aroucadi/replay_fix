# Tasks: Spec 120 — Realtime Multimodal Perception (v1.2.0)

- [x] **Task 120-01**: Define `SamWirePayload`, `AsrPayload`, and `ActionTrajectory` data structures in `gateway.rs` and `perception.rs`.
- [x] **Task 120-02**: Implement live `sam-3.1` HTTPS dispatch in `MetaGatewayClient`.
- [x] **Task 120-03**: Implement live `muse-voice-transcribe-1.0` HTTPS dispatch in `MetaGatewayClient`.
- [x] **Task 120-04**: Replace CPU pixel-diff in `perception.rs` with live `SamSegmenter::segment_live` in `live-ai` mode.
- [x] **Task 120-05**: Replace simulated SSE string in `perception.rs` with live `AsrTranscriber::transcribe_live` in `live-ai` mode.
- [x] **Task 120-06**: Implement multi-step interaction trajectory aggregation in `video.rs` and `perception.rs`.
- [x] **Task 120-07**: Wire CDP `elementFromPoint` verification to SAM centroid in `browser.rs` and `locator.rs`, and wire `BoundedRepairClient` to `MetaGatewayClient`.
- [x] **Task 120-08**: Update `audit_receipt.json` serialization to record live perception and trajectory telemetry.
- [x] **Task 120-09**: Add unit and integration tests verifying SAM token parsing, WAV validation, and trajectory sequencing.
- [x] **Task 120-10**: Execute end-to-end verification against real benchmark video `RF-01_quote_repost_crash_with_cursor.mp4`.
