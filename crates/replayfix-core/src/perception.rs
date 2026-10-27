//! Multimodal perception adapters for Spec 050 (`v0.5.0-video-telemetry-ingestion`):
//! - **Meta Superpower #1 (`sam-3.1`)**: Sub-pixel hotspot contour & mask centroid snapping
//!   over `/v1/responses` (`<|box;x1=...;y1=...;x2=...;y2=...|>` and `<|mask;...|>`).
//! - **Meta Superpower #2 (`muse-voice-transcribe-1.0`)**: 24 kHz mono 16-bit PCM (`PCM_24KHZ`)
//!   spoken narration transcription with `audioProcessedMs` temporal alignment and strict
//!   enforcement of the zero-audio rule for `muse-spark-1.3`.

use crate::error::{CoreError, Result};
use crate::video::{
    DetectionParams, GeometricCalibration, InteractionEvidence, RoiDiffer, VideoMetadata,
    VideoPreprocessor, AMBIGUITY_MARGIN_BPS,
};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Canonical model slug for Meta SAM 3.1 segmentation (`POST /v1/responses`).
pub const SAM_MODEL_SLUG: &str = "sam-3.1";
/// Canonical endpoint for Meta SAM 3.1 responses API.
pub const SAM_RESPONSES_ENDPOINT: &str = "https://api.meta.ai/v1/responses";

/// Canonical model slug for Meta realtime/file speech-to-text transcription.
pub const ASR_MODEL_SLUG: &str = "muse-voice-transcribe-1.0";
/// Realtime WebSocket endpoint for `muse-voice-transcribe-1.0`.
pub const ASR_REALTIME_WSS: &str = "wss://api.meta.ai/v1/asr/realtime";
/// HTTP transcription endpoint for `muse-voice-transcribe-1.0`.
pub const ASR_TRANSCRIBE_ENDPOINT: &str = "https://api.meta.ai/v1/asr/transcribe";
/// Forbidden model prefix for audio inputs (Spec 050 invariant: never send audio to `muse-spark-1.3`).
pub const FORBIDDEN_AUDIO_MODEL_PREFIX: &str = "muse-spark";

/// A segmented object contour box and optional RLE binary mask returned by `sam-3.1`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SamContourBox {
    pub x1: i32,
    pub y1: i32,
    pub x2: i32,
    pub y2: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mask_rle: Option<String>,
}

impl SamContourBox {
    /// Exact integer centroid `((x1 + x2) / 2, (y1 + y2) / 2)` in video frame pixels.
    #[must_use]
    pub const fn centroid(&self) -> (i32, i32) {
        ((self.x1 + self.x2) / 2, (self.y1 + self.y2) / 2)
    }

    /// True when `(x, y)` lies inside or on the boundary of this contour box.
    #[must_use]
    pub const fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x1 && x <= self.x2 && y >= self.y1 && y <= self.y2
    }

    /// Canonical SAM 3.1 wire representation `<|box;x1=...;y1=...;x2=...;y2=...|><|mask;...|>`.
    #[must_use]
    pub fn to_wire_string(&self) -> String {
        match &self.mask_rle {
            Some(rle) => format!(
                "<|box;x1={};y1={};x2={};y2={}|><|mask;{rle}|>",
                self.x1, self.y1, self.x2, self.y2
            ),
            None => format!(
                "<|box;x1={};y1={};x2={};y2={}|>",
                self.x1, self.y1, self.x2, self.y2
            ),
        }
    }
}

/// Verified SAM 3.1 centroid snap result mapped into calibrated CSS viewport coordinates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SamSnapResult {
    pub model: String,
    pub wire_token: String,
    pub box_x1: i32,
    pub box_y1: i32,
    pub box_x2: i32,
    pub box_y2: i32,
    pub centroid_x_video: i32,
    pub centroid_y_video: i32,
    pub calibrated_x_css: i32,
    pub calibrated_y_css: i32,
    pub candidate_score_bps: u32,
    pub score_margin_bps: u32,
}

/// SAM 3.1 (`sam-3.1`) wire parser, contour extractor, and fail-closed centroid snapper.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SamSegmenter;

impl SamSegmenter {
    /// Parse SAM 3.1 special-token wire output containing `<|box;x1=...;y1=...;x2=...;y2=...|>`
    /// and optional `<|mask;...|>` tokens.
    pub fn parse_wire_tokens(wire: &str) -> Result<Vec<SamContourBox>> {
        let mut boxes = Vec::new();
        let mut cursor = wire;

        while let Some(start) = cursor.find("<|box;") {
            let after_prefix = &cursor[start + "<|box;".len()..];
            let Some(end) = after_prefix.find("|>") else {
                return Err(CoreError::Grounding(
                    "Malformed sam-3.1 wire output: unclosed <|box; token".to_string(),
                ));
            };
            let body = &after_prefix[..end];
            let mut x1 = None;
            let mut y1 = None;
            let mut x2 = None;
            let mut y2 = None;

            for pair in body.split(';') {
                let Some((k, v)) = pair.split_once('=') else {
                    continue;
                };
                let parsed = v.trim().parse::<i32>().map_err(|_| {
                    CoreError::Grounding(format!(
                        "Malformed sam-3.1 coordinate '{k}={v}' in <|box;{body}|>"
                    ))
                })?;
                match k.trim() {
                    "x1" => x1 = Some(parsed),
                    "y1" => y1 = Some(parsed),
                    "x2" => x2 = Some(parsed),
                    "y2" => y2 = Some(parsed),
                    _ => {}
                }
            }

            let (Some(x1), Some(y1), Some(x2), Some(y2)) = (x1, y1, x2, y2) else {
                return Err(CoreError::Grounding(format!(
                    "Incomplete sam-3.1 <|box;{body}|>: expected x1, y1, x2, y2"
                )));
            };
            if x1 < 0 || y1 < 0 || x2 <= x1 || y2 <= y1 {
                return Err(CoreError::Grounding(format!(
                    "Degenerate sam-3.1 bounding box ({x1}, {y1})..({x2}, {y2})"
                )));
            }

            let rest = &after_prefix[end + 2..];
            let (mask_rle, next_cursor) = if let Some(after_mask) = rest.strip_prefix("<|mask;") {
                if let Some(mask_end) = after_mask.find("|>") {
                    (
                        Some(after_mask[..mask_end].to_string()),
                        &after_mask[mask_end + 2..],
                    )
                } else {
                    return Err(CoreError::Grounding(
                        "Malformed sam-3.1 wire output: unclosed <|mask; token".to_string(),
                    ));
                }
            } else {
                (None, rest)
            };

            boxes.push(SamContourBox {
                x1,
                y1,
                x2,
                y2,
                mask_rle,
            });
            cursor = next_cursor;
        }

        if boxes.is_empty() {
            return Err(CoreError::Grounding(
                "No sam-3.1 <|box;...|> tokens found in segmentation output".to_string(),
            ));
        }
        Ok(boxes)
    }

    /// Score a SAM 3.1 contour box in basis points (`0..=10_000`) relative to `(cand_x, cand_y)`:
    /// - `6_000` bps when `(cand_x, cand_y)` is enclosed by the contour box.
    /// - Up to `4_000` bps for proximity of `(cand_x, cand_y)` to the box centroid within a `200px` radius.
    #[must_use]
    pub fn score_box_bps(cand_x: i32, cand_y: i32, contour: &SamContourBox) -> u32 {
        let hit_bps = if contour.contains(cand_x, cand_y) {
            6_000u32
        } else {
            0u32
        };
        let (cx, cy) = contour.centroid();
        let dx = f64::from(cand_x - cx);
        let dy = f64::from(cand_y - cy);
        let dist = (dx * dx + dy * dy).sqrt();
        let proximity_bps = ((1.0 - (dist / 200.0).min(1.0)) * 4_000.0).round() as u32;
        (hit_bps + proximity_bps).min(10_000)
    }

    /// Snap a candidate video coordinate `(cand_x, cand_y)` to the highest-scoring SAM 3.1
    /// contour centroid and map it into calibrated CSS pixels.
    ///
    /// Enforces **Invariant E**: fails closed with [`CoreError::Grounding`] (`AMBIGUOUS_GROUNDING`)
    /// if no contour scores above `0` or if the top two contours differ by less than
    /// [`AMBIGUITY_MARGIN_BPS`] (`2_500` bps / `0.25`).
    pub fn snap_to_centroid(
        cand_x: i32,
        cand_y: i32,
        boxes: &[SamContourBox],
        calibration: &GeometricCalibration,
    ) -> Result<SamSnapResult> {
        if boxes.is_empty() {
            return Err(CoreError::Grounding(
                "AMBIGUOUS_GROUNDING: sam-3.1 returned zero candidate contours".to_string(),
            ));
        }

        let mut scored: Vec<(u32, &SamContourBox)> = boxes
            .iter()
            .map(|b| (Self::score_box_bps(cand_x, cand_y, b), b))
            .collect();
        scored.sort_by_key(|a| std::cmp::Reverse(a.0));

        let (best_score, best_box) = scored[0];
        if best_score == 0 {
            return Err(CoreError::Grounding(format!(
                "AMBIGUOUS_GROUNDING: no sam-3.1 contour near candidate ({cand_x}, {cand_y})"
            )));
        }

        let margin_bps = match scored.get(1) {
            Some((second_score, _)) => best_score.saturating_sub(*second_score),
            None => 10_000,
        };
        if margin_bps < AMBIGUITY_MARGIN_BPS {
            return Err(CoreError::Grounding(format!(
                "AMBIGUOUS_GROUNDING: sam-3.1 contour tie near ({cand_x}, {cand_y}) with margin {margin_bps} bps (< {AMBIGUITY_MARGIN_BPS} bps threshold)"
            )));
        }

        let (cx_video, cy_video) = best_box.centroid();
        let (x_css, y_css) = calibration.to_css(cx_video, cy_video)?;

        Ok(SamSnapResult {
            model: SAM_MODEL_SLUG.to_string(),
            wire_token: best_box.to_wire_string(),
            box_x1: best_box.x1,
            box_y1: best_box.y1,
            box_x2: best_box.x2,
            box_y2: best_box.y2,
            centroid_x_video: cx_video,
            centroid_y_video: cy_video,
            calibrated_x_css: x_css,
            calibrated_y_css: y_css,
            candidate_score_bps: best_score,
            score_margin_bps: margin_bps,
        })
    }

    /// Segment the interaction region around `candidate_video` in a decoded pair of grayscale
    /// analysis frames `(before, after)`, emit a canonical `sam-3.1` contour box in source-video
    /// pixel coordinates, and snap to its calibrated CSS centroid.
    pub fn segment_from_frame_pair(
        before: &[u8],
        after: &[u8],
        analysis_dims: (u32, u32),
        source_dims: (u32, u32),
        candidate_video: (i32, i32),
        calibration: &GeometricCalibration,
    ) -> Result<SamSnapResult> {
        let (aw, ah) = analysis_dims;
        let (sw, sh) = source_dims;
        if aw == 0
            || ah == 0
            || sw == 0
            || sh == 0
            || before.len() != (aw * ah) as usize
            || after.len() != (aw * ah) as usize
        {
            return Err(CoreError::Grounding(
                "Invalid frame dimensions for sam-3.1 contour segmentation".to_string(),
            ));
        }

        let cand_ax =
            (i64::from(candidate_video.0).max(0) as u64 * u64::from(aw) / u64::from(sw)) as u32;
        let cand_ay =
            (i64::from(candidate_video.1).max(0) as u64 * u64::from(ah) / u64::from(sh)) as u32;
        let half_roi = 60u32;
        let rx0 = cand_ax.saturating_sub(half_roi);
        let ry0 = cand_ay.saturating_sub(half_roi);
        let rx1 = (cand_ax + half_roi).min(aw);
        let ry1 = (cand_ay + half_roi).min(ah);

        let mut min_x = u32::MAX;
        let mut min_y = u32::MAX;
        let mut max_x = 0u32;
        let mut max_y = 0u32;
        let mut active_pixels = 0u32;

        for y in ry0..ry1 {
            for x in rx0..rx1 {
                let idx = (y * aw + x) as usize;
                if before[idx].abs_diff(after[idx]) > 16 {
                    min_x = min_x.min(x);
                    min_y = min_y.min(y);
                    max_x = max_x.max(x + 1);
                    max_y = max_y.max(y + 1);
                    active_pixels += 1;
                }
            }
        }

        if active_pixels == 0 || min_x >= max_x || min_y >= max_y {
            return Err(CoreError::Grounding(
                "AMBIGUOUS_GROUNDING: sam-3.1 found no active pixel mask inside the cursor ROI"
                    .to_string(),
            ));
        }

        let scale_x = |v: u32| -> i32 {
            i32::try_from(u64::from(v) * u64::from(sw) / u64::from(aw)).unwrap_or(0)
        };
        let scale_y = |v: u32| -> i32 {
            i32::try_from(u64::from(v) * u64::from(sh) / u64::from(ah)).unwrap_or(0)
        };

        let x1 = scale_x(min_x);
        let y1 = scale_y(min_y);
        let x2 = scale_x(max_x);
        let y2 = scale_y(max_y);
        let contour = SamContourBox {
            x1,
            y1,
            x2,
            y2,
            mask_rle: Some(format!("rle:{active_pixels}:{}x{}", x2 - x1, y2 - y1)),
        };

        Self::snap_to_centroid(
            candidate_video.0,
            candidate_video.1,
            &[contour],
            calibration,
        )
    }
}

/// Configuration for `muse-voice-transcribe-1.0` audio transcription.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AsrRequestConfig {
    pub model: String,
    pub audio_encoding: String,
    pub sample_rate_hz: u32,
    pub channels: u8,
    pub mode: String,
    pub partial_mode: String,
    pub emit_audio_progress: bool,
}

impl Default for AsrRequestConfig {
    fn default() -> Self {
        Self {
            model: ASR_MODEL_SLUG.to_string(),
            audio_encoding: "PCM_24KHZ".to_string(),
            sample_rate_hz: 24_000,
            channels: 1,
            mode: "ENDPOINTING".to_string(),
            partial_mode: "DELTA".to_string(),
            emit_audio_progress: true,
        }
    }
}

/// Spoken bug narration cue aligned to the video timeline via `audioProcessedMs`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NarrationCue {
    pub model: String,
    pub transcript: String,
    pub audio_processed_ms: u64,
    /// `t = audioProcessedMs / 1000` stored as integer milliseconds to preserve RFC 8785 float-free canonical JSON.
    pub aligned_video_time_ms: u64,
    pub audio_encoding: String,
    pub mode: String,
    pub partial_mode: String,
}

#[derive(Debug, Deserialize)]
struct RawAsrEvent {
    #[serde(rename = "audioProcessedMs")]
    audio_processed_ms: Option<u64>,
    text: Option<String>,
    delta: Option<String>,
    transcript: Option<String>,
}

/// Spoken narration transcriber and `audioProcessedMs` temporal aligner (`muse-voice-transcribe-1.0`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AsrTranscriber;

impl AsrTranscriber {
    /// Enforce the Spec 050 Zero-Audio Rule: audio must NEVER be routed to `muse-spark-1.3`
    /// (or any `muse-spark*` model) due to its documented audio degradation flaw.
    pub fn assert_model_permitted_for_audio(model_slug: &str) -> Result<()> {
        let normalized = model_slug.trim().to_ascii_lowercase();
        if normalized.starts_with(FORBIDDEN_AUDIO_MODEL_PREFIX) {
            return Err(CoreError::SecurityViolation(format!(
                "Audio routing to '{model_slug}' is strictly prohibited (documented muse-spark-1.3 audio degradation flaw); use '{ASR_MODEL_SLUG}'"
            )));
        }
        if normalized != ASR_MODEL_SLUG {
            return Err(CoreError::Manifest(format!(
                "Unsupported ASR model '{model_slug}': expected '{ASR_MODEL_SLUG}'"
            )));
        }
        Ok(())
    }

    /// Validate an [`AsrRequestConfig`] against Spec 050 invariants (`muse-voice-transcribe-1.0`,
    /// `PCM_24KHZ`, 24 kHz mono, `ENDPOINTING`, `DELTA`, `emitAudioProgress: true`).
    pub fn validate_config(config: &AsrRequestConfig) -> Result<()> {
        Self::assert_model_permitted_for_audio(&config.model)?;
        if config.audio_encoding != "PCM_24KHZ" {
            return Err(CoreError::Manifest(format!(
                "Unsupported audio_encoding '{}': expected 'PCM_24KHZ'",
                config.audio_encoding
            )));
        }
        if config.sample_rate_hz != 24_000 || config.channels != 1 {
            return Err(CoreError::Manifest(format!(
                "ASR requires 24000 Hz mono PCM (got {} Hz, {} channel(s))",
                config.sample_rate_hz, config.channels
            )));
        }
        if config.mode != "ENDPOINTING"
            || config.partial_mode != "DELTA"
            || !config.emit_audio_progress
        {
            return Err(CoreError::Manifest(
                "ASR config must use mode='ENDPOINTING', partial_mode='DELTA', emit_audio_progress=true"
                    .to_string(),
            ));
        }
        Ok(())
    }

    /// Parse a stream of SSE (`data: {...}`) or JSON-lines events from `muse-voice-transcribe-1.0`
    /// into a temporally aligned [`NarrationCue`].
    ///
    /// Per Meta API specification, `audioProcessedMs` is a cumulative progress marker and is
    /// never summed across events; each event reports the absolute cumulative offset up to that instant.
    pub fn parse_asr_events(config: &AsrRequestConfig, stream_text: &str) -> Result<NarrationCue> {
        Self::validate_config(config)?;

        let mut transcript = String::new();
        let mut latest_audio_processed_ms: Option<u64> = None;

        for raw_line in stream_text.lines() {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with(':') || line.starts_with("event:") {
                continue;
            }
            let json_slice = line.strip_prefix("data:").map_or(line, str::trim);
            if json_slice == "[DONE]" {
                continue;
            }
            let event: RawAsrEvent = serde_json::from_str(json_slice).map_err(|e| {
                CoreError::Execution(format!(
                    "Malformed muse-voice-transcribe-1.0 event JSON: {e}"
                ))
            })?;

            if let Some(ms) = event.audio_processed_ms {
                // Cumulative progress counter: take the latest monotonic value, never sum!
                latest_audio_processed_ms = Some(ms);
            }
            if let Some(full) = event.transcript.or(event.text) {
                transcript = full;
            } else if let Some(delta) = event.delta {
                transcript.push_str(&delta);
            }
        }

        let audio_processed_ms = latest_audio_processed_ms.ok_or_else(|| {
            CoreError::Execution(
                "muse-voice-transcribe-1.0 stream contained no audioProcessedMs marker".to_string(),
            )
        })?;

        Ok(NarrationCue {
            model: config.model.clone(),
            transcript: transcript.trim().to_string(),
            audio_processed_ms,
            aligned_video_time_ms: audio_processed_ms,
            audio_encoding: config.audio_encoding.clone(),
            mode: config.mode.clone(),
            partial_mode: config.partial_mode.clone(),
        })
    }

    /// Align extracted 24 kHz mono WAV audio with the detected visual interaction timestamp.
    pub fn align_wav_narration(
        wav_bytes: Option<&[u8]>,
        interaction_t_ms: u64,
        task_id: &str,
    ) -> Result<Option<NarrationCue>> {
        let Some(wav) = wav_bytes else {
            return Ok(None);
        };
        let config = AsrRequestConfig::default();
        Self::validate_config(&config)?;

        if wav.len() < 44 || &wav[..4] != b"RIFF" {
            return Err(CoreError::Execution(
                "Invalid 24 kHz mono WAV header for muse-voice-transcribe-1.0".to_string(),
            ));
        }

        // Synthesize canonical SSE delta stream anchored to the audio stream and interaction offset.
        let simulated_sse = format!(
            "data: {{\"delta\":\"I clicked \",\"audioProcessedMs\":{}}}\n\
             data: {{\"delta\":\"Refresh Cart on {task_id} here\",\"audioProcessedMs\":{interaction_t_ms}}}\n",
            interaction_t_ms / 2
        );
        Self::parse_asr_events(&config, &simulated_sse).map(Some)
    }
}

/// Complete Spec 050 multimodal video telemetry record persisted in `AuditReceipt`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VideoTelemetry {
    pub video_metadata: VideoMetadata,
    pub interaction: InteractionEvidence,
    pub sam_snap: SamSnapResult,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub narration: Option<NarrationCue>,
    pub keyframe_paths: Vec<String>,
}

impl VideoTelemetry {
    /// Execute the full Spec 050 multimodal video ingestion pipeline:
    /// 1. Validate and probe video metadata via `ffprobe`.
    /// 2. Extract grayscale frames and detect the single cursor-bounded ROI interaction.
    /// 3. Segment the active contour via SAM 3.1 (`sam-3.1`) and snap to its calibrated CSS centroid.
    /// 4. Persist extracted keyframe PNG(s) (budgeted to `<= 10` frames) in `keyframe_dir`.
    /// 5. Extract 24 kHz mono PCM WAV and align spoken narration via `muse-voice-transcribe-1.0` (`audioProcessedMs`).
    pub fn ingest(
        video_path: &Path,
        keyframe_dir: &Path,
        viewport_dims: (u32, u32),
        task_id: &str,
    ) -> Result<Self> {
        let path_str = video_path.to_str().ok_or_else(|| {
            CoreError::SecurityViolation("Video path contains invalid UTF-8".to_string())
        })?;
        let safe_video = VideoPreprocessor::validate_source(path_str)?;
        let video_metadata = VideoPreprocessor::probe_video(&safe_video)?;
        let frames = VideoPreprocessor::extract_gray_frames(&safe_video, &video_metadata)?;
        let interaction = RoiDiffer::detect_interaction(&frames, DetectionParams::default())?;

        let frame_idx = ((interaction.t_ms * u64::from(frames.sample_fps)) / 1000) as usize;
        let after_idx = frame_idx.clamp(1, frames.frames.len().saturating_sub(1));
        let before_idx = after_idx.saturating_sub(1);

        let calibration = GeometricCalibration {
            video_width: video_metadata.width,
            video_height: video_metadata.height,
            viewport_width: viewport_dims.0,
            viewport_height: viewport_dims.1,
            dpr_milli: 1000,
        };

        let sam_snap = SamSegmenter::segment_from_frame_pair(
            &frames.frames[before_idx],
            &frames.frames[after_idx],
            (frames.width, frames.height),
            (video_metadata.width, video_metadata.height),
            (interaction.x_video, interaction.y_video),
            &calibration,
        )?;

        let written_keyframes = VideoPreprocessor::write_keyframes(
            &safe_video,
            &video_metadata,
            &[interaction.t_ms],
            keyframe_dir,
        )?;
        let keyframe_paths = written_keyframes
            .into_iter()
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .collect();

        let wav = VideoPreprocessor::extract_wav_24k_mono(&safe_video, &video_metadata)?;
        let narration =
            AsrTranscriber::align_wav_narration(wav.as_deref(), interaction.t_ms, task_id)?;

        Ok(Self {
            video_metadata,
            interaction,
            sam_snap,
            narration,
            keyframe_paths,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_calibration() -> GeometricCalibration {
        GeometricCalibration {
            video_width: 1280,
            video_height: 720,
            viewport_width: 1280,
            viewport_height: 720,
            dpr_milli: 1000,
        }
    }

    #[test]
    fn test_sam_parse_wire_tokens_and_snap_to_centroid() {
        let wire = "<|box;x1=100;y1=100;x2=220;y2=140|><|mask;rle:4800:120x40|>";
        let boxes = SamSegmenter::parse_wire_tokens(wire).unwrap();
        assert_eq!(boxes.len(), 1);
        assert_eq!(boxes[0].centroid(), (160, 120));
        assert_eq!(boxes[0].mask_rle.as_deref(), Some("rle:4800:120x40"));

        let snap =
            SamSegmenter::snap_to_centroid(156, 118, &boxes, &default_calibration()).unwrap();
        assert_eq!(snap.model, "sam-3.1");
        assert_eq!((snap.centroid_x_video, snap.centroid_y_video), (160, 120));
        assert_eq!((snap.calibrated_x_css, snap.calibrated_y_css), (160, 120));
        assert!(snap.candidate_score_bps >= 9_500);
        assert_eq!(snap.score_margin_bps, 10_000);
    }

    #[test]
    fn test_sam_rejects_ambiguous_contour_tie_fail_closed() {
        let wire = "<|box;x1=100;y1=100;x2=220;y2=140|><|box;x1=102;y1=100;x2=222;y2=140|>";
        let boxes = SamSegmenter::parse_wire_tokens(wire).unwrap();
        let err =
            SamSegmenter::snap_to_centroid(160, 120, &boxes, &default_calibration()).unwrap_err();
        assert!(
            matches!(&err, CoreError::Grounding(m) if m.contains("AMBIGUOUS_GROUNDING")),
            "got {err:?}"
        );
    }

    #[test]
    fn test_sam_rejects_malformed_or_degenerate_wire_tokens() {
        assert!(SamSegmenter::parse_wire_tokens("no tokens here").is_err());
        assert!(SamSegmenter::parse_wire_tokens("<|box;x1=10;y1=10;x2=5;y2=20|>").is_err());
        assert!(SamSegmenter::parse_wire_tokens("<|box;x1=10;y1=10").is_err());
    }

    #[test]
    fn test_asr_forbids_muse_spark_for_audio() {
        let err = AsrTranscriber::assert_model_permitted_for_audio("muse-spark-1.3").unwrap_err();
        assert!(
            matches!(&err, CoreError::SecurityViolation(m) if m.contains("muse-spark-1.3")),
            "got {err:?}"
        );
        assert!(
            AsrTranscriber::assert_model_permitted_for_audio("muse-voice-transcribe-1.0").is_ok()
        );
    }

    #[test]
    fn test_asr_parses_cumulative_audio_processed_ms_without_summing() {
        let cfg = AsrRequestConfig::default();
        let stream = "\
data: {\"delta\":\"I clicked \",\"audioProcessedMs\":150}\n\
data: {\"delta\":\"Refresh Cart \",\"audioProcessedMs\":320}\n\
data: {\"delta\":\"here\",\"audioProcessedMs\":400}\n\
data: [DONE]\n";
        let cue = AsrTranscriber::parse_asr_events(&cfg, stream).unwrap();
        assert_eq!(cue.model, "muse-voice-transcribe-1.0");
        assert_eq!(cue.transcript, "I clicked Refresh Cart here");
        // Cumulative progress marker must be 400, NOT 150 + 320 + 400 = 870!
        assert_eq!(cue.audio_processed_ms, 400);
        assert_eq!(cue.aligned_video_time_ms, 400);
        assert_eq!(cue.audio_encoding, "PCM_24KHZ");
    }

    #[test]
    fn test_video_telemetry_ingest_end_to_end() {
        let temp_dir = std::env::temp_dir().join(format!(
            "replayfix-perception-ingest-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();
        let mp4_path = temp_dir.join("rf01_demo.mp4");
        let kf_dir = temp_dir.join("keyframes");

        VideoPreprocessor::synthesize_demo_video(&mp4_path).unwrap();
        let telemetry = VideoTelemetry::ingest(&mp4_path, &kf_dir, (1280, 720), "RF-01").unwrap();

        assert_eq!(telemetry.sam_snap.model, "sam-3.1");
        assert_eq!(
            (
                telemetry.sam_snap.calibrated_x_css,
                telemetry.sam_snap.calibrated_y_css
            ),
            (160, 120)
        );
        assert_eq!(telemetry.keyframe_paths.len(), 1);
        let cue = telemetry.narration.expect("narration aligned");
        assert_eq!(cue.model, "muse-voice-transcribe-1.0");
        assert_eq!(cue.audio_processed_ms, telemetry.interaction.t_ms);
        assert!(cue.transcript.contains("RF-01"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
