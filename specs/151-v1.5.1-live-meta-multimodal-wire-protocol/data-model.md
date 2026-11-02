# Data Model: Spec 151 — Live Meta Multimodal Wire Protocol & Full Muse Model Utilization (v1.5.1)

## 1. `GatewayRequest` & `GatewayResponse` Extensions (`crates/replayfix-core/src/gateway.rs`)
- `GatewayRequest`:
  - `enable_web_search: bool` (optional, default `false`; when `true` on `muse-spark-1.3` / `muse-wonder-1.0`, attaches `"tools": [{"type": "web_search"}]`)
- `GatewayResponse`:
  - `images_processed: u64` (parsed from `usage.images_processed`)
  - `video_frames_processed: u64` (parsed from `usage.video_frames_processed`)
  - `tool_calls_count: u64` (counted from `output` items where `type == "web_search_call"` or `type == "function_call"`)

## 2. `SamVideoSegmentResponse` (`crates/replayfix-core/src/gateway.rs`)
- `wire_tokens: String`
- `latency_ms: u64`
- `images_processed: u64`
- `video_frames_processed: u64`

## 3. `VideoTelemetry` Extensions (`crates/replayfix-core/src/perception.rs`)
- `images_segmented: Option<u64>`
- `video_frames_segmented: Option<u64>`
- `verification_badge_path: Option<String>`
