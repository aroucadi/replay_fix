# Implementation Plan: Spec 151 — Live Meta Multimodal Wire Protocol & Full Muse Model Utilization (Release 1.5.1)

**Branch**: `release/v1.5.1` | **Date**: 2026-11-01 | **Spec**: [spec.md](./spec.md)  
**Input**: Feature specification from `/specs/151-v1.5.1-live-meta-multimodal-wire-protocol/spec.md`

## Summary

Release 1.5.1 updates ReplayFix's live Meta AI cloud gateway (`crates/replayfix-core/src/gateway.rs`), perception engine (`crates/replayfix-core/src/perception.rs`), and autonomous repair client (`crates/replayfix-core/src/repair.rs`) to match the exact production wire schemas of `https://api.meta.ai/v1/` across all 5 Meta models:
1. **`sam-3.1` Image & Video Segmentation**: Sends `input_image` (`data:image/png;base64,...`) and `input_video` (`data:video/mp4;base64,...`) content blocks to `POST /v1/responses`, parses 6-attribute `<|box;x1=...;y1=...;x2=...;y2=...;w=...;h=...|>` wire tokens, and records `images_processed` + `video_frames_processed`.
2. **`muse-voice-transcribe-1.0` Multipart WAV ASR**: Sends RFC 7578 `multipart/form-data` (`request={"model":"muse-voice-transcribe-1.0","audioEncoding":"WAV"}` + `audio` WAV part) to `POST /v1/asr/transcribe` and parses `audioDurationMs` / `turns` / `transcript`.
3. **`muse-spark-1.3` & `muse-wonder-1.0` Multimodal Vision + `web_search` Tool Calling**: Attaches `"tools": [{"type": "web_search"}]` and keyframe `input_image` blocks on live diagnostic/repair calls.
4. **`muse-image-1.0` Verification Badge Generation**: Calls `POST /v1/images/generations` to synthesize `.replayfix/keyframes/muse_verification_badge.webp` during `--mode live-ai` runs.

## Technical Context

**Language/Version**: Rust 2021 (`1.75+`)  
**Primary Dependencies**: `serde`, `serde_json`, `reqwest` (`rustls-tls`, `blocking`), `regex`, `sha2`  
**Testing**: `cargo test --workspace`, live end-to-end `--mode live-ai` verification against `https://api.meta.ai/v1/`  
**Constraints**: `#![forbid(unsafe_code)]`, zero `.unwrap()`/`.expect()`/`panic!()` outside tests, 0 Clippy warnings (`-D warnings`).

## Constitution Check

- **Law I (Zero Panics & Memory Safety)**: `#![forbid(unsafe_code)]` and `check-panics.ps1` enforced.
- **Law VI (Zero-Audio Rule)**: Audio bytes remain strictly quarantined to `muse-voice-transcribe-1.0` (`POST /v1/asr/transcribe`).
- **Law VII (Execution-Derived Provenance)**: `LIVE_PROVIDER` provenance and exact token/frame/image/audio counters are derived exclusively from real HTTP 200 responses.
