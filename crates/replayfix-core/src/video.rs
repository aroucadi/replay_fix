//! Video ingestion: probing, geometric calibration, ROI differencing, and keyframes.
//!
//! A bug recording is only ever a source of *candidate* coordinates. Every candidate is
//! validated against the live DOM by the locator (Law I); this module never decides that
//! a bug is reproduced. Anything ambiguous or missing fails closed with
//! [`CoreError::Grounding`] (`AMBIGUOUS_GROUNDING`, exit code 22).

use crate::error::{CoreError, Result};
use crate::manifest::TaskManifest;
use crate::platform::Platform;
use crate::process::run_bounded;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Largest recording accepted for ingestion.
pub const MAX_VIDEO_BYTES: u64 = 512 * 1024 * 1024;
/// Longest recording analysed; longer videos are rejected rather than silently truncated.
pub const MAX_VIDEO_DURATION_MS: u64 = 90_000;
/// Hard cap on persisted keyframes (token and latency budget, Spec 050 section 2).
pub const MAX_KEYFRAMES: usize = 10;
/// Side length of the cursor-bounded region of interest, in source-video pixels.
pub const ROI_SIDE_PX: u32 = 120;
/// Frame sampling rate used for differencing.
const SAMPLE_FPS: u32 = 10;
/// Widest frame analysed; wider videos are downscaled and coordinates mapped back.
const MAX_ANALYSIS_WIDTH: u32 = 640;
/// Two events whose score margin is below this (basis points) are ambiguous.
pub const AMBIGUITY_MARGIN_BPS: u32 = 2_500;
/// Per-pixel luma difference below this is treated as codec noise.
const NOISE_FLOOR: u8 = 16;
/// Minimum share of changed pixels inside the ROI (basis points) to accept an event.
/// 20 bps (0.2%) keeps sub-1% micro-interactions visible while rejecting codec noise.
pub const DEFAULT_TAU_BPS: u32 = 20;
/// Minimum absolute number of changed analysis pixels for an event.
const MIN_CHANGED_PIXELS: u64 = 4;

const PROBE_TIMEOUT: Duration = Duration::from_secs(15);
const FRAMES_TIMEOUT: Duration = Duration::from_secs(60);
const STILL_TIMEOUT: Duration = Duration::from_secs(20);
const AUDIO_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_PROBE_BYTES: usize = 1024 * 1024;
const MAX_STILL_BYTES: usize = 16 * 1024 * 1024;
const MAX_WAV_BYTES: usize = 32 * 1024 * 1024;

/// Container, geometry and track facts reported by `ffprobe`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VideoMetadata {
    pub width: u32,
    pub height: u32,
    pub duration_ms: u64,
    /// Frames per second multiplied by 1000 (integers keep receipts deterministic).
    pub fps_milli: u32,
    pub has_audio: bool,
    pub container: String,
}

#[derive(Debug, Deserialize)]
struct ProbeOutput {
    #[serde(default)]
    streams: Vec<ProbeStream>,
    format: Option<ProbeFormat>,
}

#[derive(Debug, Deserialize)]
struct ProbeStream {
    codec_type: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    avg_frame_rate: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ProbeFormat {
    format_name: Option<String>,
    duration: Option<String>,
}

/// Parse `"30000/1001"` (or `"30"`) into frames per second times 1000.
fn parse_fps_milli(rate: &str) -> Option<u32> {
    let (num, den) = match rate.split_once('/') {
        Some((n, d)) => (n.parse::<u64>().ok()?, d.parse::<u64>().ok()?),
        None => (rate.parse::<u64>().ok()?, 1),
    };
    if den == 0 {
        return None;
    }
    u32::try_from(num * 1000 / den).ok()
}

/// Parse a decimal seconds string such as `"2.000000"` into milliseconds.
fn parse_duration_ms(seconds: &str) -> Option<u64> {
    let (whole, frac) = seconds.split_once('.').unwrap_or((seconds, ""));
    let whole = whole.parse::<u64>().ok()?;
    let mut millis = frac.chars().take(3).collect::<String>();
    while millis.len() < 3 {
        millis.push('0');
    }
    Some(whole * 1000 + millis.parse::<u64>().ok()?)
}

/// Frames decoded to grayscale at the analysis resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameSet {
    pub width: u32,
    pub height: u32,
    pub source_width: u32,
    pub source_height: u32,
    pub sample_fps: u32,
    pub frames: Vec<Vec<u8>>,
}

impl FrameSet {
    /// Timestamp of frame `index` in milliseconds from the start of the video.
    #[must_use]
    pub fn t_ms(&self, index: usize) -> u64 {
        index as u64 * 1000 / u64::from(self.sample_fps)
    }
}

/// Thresholds for [`RoiDiffer`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DetectionParams {
    pub noise_floor: u8,
    pub tau_bps: u32,
}

impl Default for DetectionParams {
    fn default() -> Self {
        Self {
            noise_floor: NOISE_FLOOR,
            tau_bps: DEFAULT_TAU_BPS,
        }
    }
}

/// The single interaction hotspot found in a recording, in source-video pixels.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InteractionEvidence {
    pub t_ms: u64,
    pub x_video: i32,
    pub y_video: i32,
    /// Share of changed pixels inside the ROI, in basis points.
    pub roi_changed_bps: u32,
    /// `(best - runner_up) / best` in basis points; 10000 when there is no runner-up.
    pub score_margin_bps: u32,
}

/// Cursor-bounded ROI differencing over decoded frames.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RoiDiffer;

struct PairChange {
    frame_index: usize,
    changed: u64,
    sum_x: u64,
    sum_y: u64,
}

struct Event {
    first_frame: usize,
    changed: u64,
    sum_x: u64,
    sum_y: u64,
}

impl Event {
    fn centroid(&self) -> (u32, u32) {
        let n = self.changed.max(1);
        ((self.sum_x / n) as u32, (self.sum_y / n) as u32)
    }
}

impl RoiDiffer {
    fn pair_change(prev: &[u8], cur: &[u8], width: u32, noise_floor: u8) -> PairChange {
        let (mut changed, mut sum_x, mut sum_y) = (0u64, 0u64, 0u64);
        for (i, (a, b)) in prev.iter().zip(cur).enumerate() {
            if a.abs_diff(*b) > noise_floor {
                changed += 1;
                sum_x += (i as u32 % width) as u64;
                sum_y += (i as u32 / width) as u64;
            }
        }
        PairChange {
            frame_index: 0,
            changed,
            sum_x,
            sum_y,
        }
    }

    /// Share of pixels inside the ROI around `center` that changed, in basis points.
    #[must_use]
    pub fn roi_changed_bps(
        prev: &[u8],
        cur: &[u8],
        dims: (u32, u32),
        center: (u32, u32),
        roi_side: u32,
        noise_floor: u8,
    ) -> u32 {
        let (width, height) = dims;
        let (cx, cy) = center;
        let half = roi_side / 2;
        let x0 = cx.saturating_sub(half);
        let y0 = cy.saturating_sub(half);
        let x1 = (cx + half).min(width);
        let y1 = (cy + half).min(height);
        let (mut changed, mut total) = (0u64, 0u64);
        for y in y0..y1 {
            for x in x0..x1 {
                let idx = (y * width + x) as usize;
                total += 1;
                if let (Some(a), Some(b)) = (prev.get(idx), cur.get(idx)) {
                    if a.abs_diff(*b) > noise_floor {
                        changed += 1;
                    }
                }
            }
        }
        changed
            .saturating_mul(10_000)
            .checked_div(total)
            .and_then(|v| u32::try_from(v).ok())
            .unwrap_or(0)
    }

    /// Find the one interaction in `frames`, or fail closed.
    ///
    /// Frame pairs are grouped into events by spatial proximity; the strongest event must
    /// beat the runner-up by at least [`AMBIGUITY_MARGIN_BPS`] and must clear the ROI
    /// threshold `tau`.
    pub fn detect_interaction(
        frames: &FrameSet,
        params: DetectionParams,
    ) -> Result<InteractionEvidence> {
        if frames.frames.len() < 2 {
            return Err(CoreError::Grounding(
                "Recording has fewer than two analysable frames".to_string(),
            ));
        }
        let scale_num = u64::from(frames.width);
        let scale_den = u64::from(frames.source_width.max(1));
        let roi_analysis =
            u32::try_from(u64::from(ROI_SIDE_PX) * scale_num / scale_den).unwrap_or(ROI_SIDE_PX);
        let roi_analysis = roi_analysis.max(8);

        let mut events: Vec<Event> = Vec::new();
        for (i, pair) in frames.frames.windows(2).enumerate() {
            let mut change =
                Self::pair_change(&pair[0], &pair[1], frames.width, params.noise_floor);
            change.frame_index = i + 1;
            if change.changed < MIN_CHANGED_PIXELS {
                continue;
            }
            let cx = (change.sum_x / change.changed) as u32;
            let cy = (change.sum_y / change.changed) as u32;
            let near = events.iter_mut().find(|e| {
                let (ex, ey) = e.centroid();
                ex.abs_diff(cx) <= roi_analysis && ey.abs_diff(cy) <= roi_analysis
            });
            match near {
                Some(event) => {
                    event.changed += change.changed;
                    event.sum_x += change.sum_x;
                    event.sum_y += change.sum_y;
                }
                None => events.push(Event {
                    first_frame: change.frame_index,
                    changed: change.changed,
                    sum_x: change.sum_x,
                    sum_y: change.sum_y,
                }),
            }
        }

        events.sort_by(|a, b| {
            b.changed
                .cmp(&a.changed)
                .then(a.first_frame.cmp(&b.first_frame))
        });
        let Some(best) = events.first() else {
            return Err(CoreError::Grounding(
                "No visual interaction detected in the recording".to_string(),
            ));
        };
        let margin_bps = match events.get(1) {
            Some(second) => u32::try_from((best.changed - second.changed) * 10_000 / best.changed)
                .unwrap_or(10_000),
            None => 10_000,
        };
        if margin_bps < AMBIGUITY_MARGIN_BPS {
            return Err(CoreError::Grounding(format!(
                "Ambiguous interaction: top two visual events differ by {margin_bps} bps (< {AMBIGUITY_MARGIN_BPS} bps threshold)"
            )));
        }

        let (cx, cy) = best.centroid();
        // The event's first frame pair carries the cleanest before/after contrast.
        let after = &frames.frames[best.first_frame];
        let before = &frames.frames[best.first_frame - 1];
        let roi_bps = Self::roi_changed_bps(
            before,
            after,
            (frames.width, frames.height),
            (cx, cy),
            roi_analysis,
            params.noise_floor,
        );
        if roi_bps < params.tau_bps {
            return Err(CoreError::Grounding(format!(
                "Visual change around ({cx}, {cy}) covers {roi_bps} bps of the ROI (< {} bps threshold)",
                params.tau_bps
            )));
        }

        let to_source = |v: u32, analysis: u32, source: u32| -> i32 {
            i32::try_from(u64::from(v) * u64::from(source) / u64::from(analysis.max(1)))
                .unwrap_or(i32::MAX)
        };
        Ok(InteractionEvidence {
            t_ms: frames.t_ms(best.first_frame),
            x_video: to_source(cx, frames.width, frames.source_width),
            y_video: to_source(cy, frames.height, frames.source_height),
            roi_changed_bps: roi_bps,
            score_margin_bps: margin_bps,
        })
    }
}

/// Maps recording pixels to CSS pixels, accounting for letterboxing and DPR.
///
/// The browser viewport (`viewport_* x dpr` device pixels) was recorded into a
/// `video_* ` frame with a single uniform scale `s` and centred padding, so
/// `x_css = (x_v - pad_x) / (s * dpr)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GeometricCalibration {
    pub video_width: u32,
    pub video_height: u32,
    pub viewport_width: u32,
    pub viewport_height: u32,
    /// Device pixel ratio multiplied by 1000 (1000 = 1.0).
    pub dpr_milli: u32,
}

impl GeometricCalibration {
    /// Translate a video-frame point to CSS pixels or fail closed.
    pub fn to_css(&self, x_video: i32, y_video: i32) -> Result<(i32, i32)> {
        if self.video_width == 0
            || self.video_height == 0
            || self.viewport_width == 0
            || self.viewport_height == 0
            || self.dpr_milli == 0
        {
            return Err(CoreError::Grounding(
                "Calibration geometry contains a zero dimension".to_string(),
            ));
        }
        let dpr = f64::from(self.dpr_milli) / 1000.0;
        let device_w = f64::from(self.viewport_width) * dpr;
        let device_h = f64::from(self.viewport_height) * dpr;
        let scale =
            (f64::from(self.video_width) / device_w).min(f64::from(self.video_height) / device_h);
        let content_w = device_w * scale;
        let content_h = device_h * scale;
        let pad_x = (f64::from(self.video_width) - content_w) / 2.0;
        let pad_y = (f64::from(self.video_height) - content_h) / 2.0;

        let (xv, yv) = (f64::from(x_video), f64::from(y_video));
        if xv < pad_x || xv > pad_x + content_w || yv < pad_y || yv > pad_y + content_h {
            return Err(CoreError::Grounding(format!(
                "Video point ({x_video}, {y_video}) lies in the letterbox padding, outside the recorded viewport"
            )));
        }
        let x_css = ((xv - pad_x) / (scale * dpr)).round() as i32;
        let y_css = ((yv - pad_y) / (scale * dpr)).round() as i32;
        Ok((x_css, y_css))
    }
}

/// Probing and decoding entry points backed by FFmpeg.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct VideoPreprocessor;

fn ffmpeg_command(program: &str) -> std::process::Command {
    let mut cmd = Platform::create_command(program);
    cmd.arg("-hide_banner").args(["-loglevel", "error"]);
    cmd
}

fn path_str(path: &Path) -> Result<&str> {
    path.to_str()
        .ok_or_else(|| CoreError::SecurityViolation("Video path contains invalid UTF-8".into()))
}

impl VideoPreprocessor {
    /// Validate the file on disk (safe path, extension, size) and return its path.
    pub fn validate_source(path: &str) -> Result<PathBuf> {
        let safe = TaskManifest::assert_safe_path(path, "video")?;
        let extension = safe
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase);
        if !matches!(extension.as_deref(), Some("mp4" | "webm")) {
            return Err(CoreError::Manifest(format!(
                "Unsupported video container for '{path}': expected .mp4 or .webm"
            )));
        }
        let meta = fs::metadata(&safe)
            .map_err(|e| CoreError::Manifest(format!("Cannot read video '{path}': {e}")))?;
        if !meta.is_file() {
            return Err(CoreError::Manifest(format!("'{path}' is not a file")));
        }
        if meta.len() > MAX_VIDEO_BYTES {
            return Err(CoreError::Manifest(format!(
                "Video '{path}' is {} bytes; the limit is {MAX_VIDEO_BYTES} bytes",
                meta.len()
            )));
        }
        Ok(safe)
    }

    /// Probe container, dimensions, duration, frame rate, and audio presence via `ffprobe`.
    pub fn probe_video(path: &Path) -> Result<VideoMetadata> {
        let mut cmd = Platform::create_command("ffprobe");
        cmd.args(["-v", "error", "-print_format", "json"]).args([
            "-show_streams",
            "-show_format",
            "-i",
            path_str(path)?,
        ]);
        let out = run_bounded(cmd, None, PROBE_TIMEOUT, MAX_PROBE_BYTES)?;
        if !out.success() {
            return Err(CoreError::Execution(format!(
                "ffprobe rejected '{}': {}",
                path.display(),
                out.stderr.trim()
            )));
        }
        let parsed: ProbeOutput = serde_json::from_slice(&out.stdout)?;
        Self::metadata_from_probe(parsed)
    }

    fn metadata_from_probe(parsed: ProbeOutput) -> Result<VideoMetadata> {
        let video = parsed
            .streams
            .iter()
            .find(|s| s.codec_type.as_deref() == Some("video"))
            .ok_or_else(|| CoreError::Manifest("Recording has no video stream".to_string()))?;
        let (width, height) = match (video.width, video.height) {
            (Some(w), Some(h)) if (16..=7680).contains(&w) && (16..=7680).contains(&h) => (w, h),
            _ => {
                return Err(CoreError::Manifest(
                    "Video dimensions are missing or outside 16..=7680 pixels".to_string(),
                ))
            }
        };
        let fps_milli = video
            .avg_frame_rate
            .as_deref()
            .and_then(parse_fps_milli)
            .filter(|f| *f > 0)
            .ok_or_else(|| CoreError::Manifest("Video frame rate is unavailable".to_string()))?;
        let format = parsed.format;
        let duration_ms = format
            .as_ref()
            .and_then(|f| f.duration.as_deref())
            .and_then(parse_duration_ms)
            .ok_or_else(|| CoreError::Manifest("Video duration is unavailable".to_string()))?;
        if duration_ms == 0 || duration_ms > MAX_VIDEO_DURATION_MS {
            return Err(CoreError::Manifest(format!(
                "Video duration {duration_ms} ms is outside 1..={MAX_VIDEO_DURATION_MS} ms"
            )));
        }
        Ok(VideoMetadata {
            width,
            height,
            duration_ms,
            fps_milli,
            has_audio: parsed
                .streams
                .iter()
                .any(|s| s.codec_type.as_deref() == Some("audio")),
            container: format
                .and_then(|f| f.format_name)
                .unwrap_or_else(|| "unknown".to_string()),
        })
    }

    /// Analysis resolution for a video: at most [`MAX_ANALYSIS_WIDTH`] wide, even sides.
    #[must_use]
    pub fn analysis_size(meta: &VideoMetadata) -> (u32, u32) {
        let width = meta.width.min(MAX_ANALYSIS_WIDTH) & !1;
        let height = (u64::from(meta.height) * u64::from(width) / u64::from(meta.width)) as u32;
        (width.max(2), (height & !1).max(2))
    }

    /// Decode the whole recording into grayscale analysis frames.
    pub fn extract_gray_frames(path: &Path, meta: &VideoMetadata) -> Result<FrameSet> {
        let (aw, ah) = Self::analysis_size(meta);
        let frame_bytes = (aw * ah) as usize;
        let max_frames = (MAX_VIDEO_DURATION_MS / 1000 * u64::from(SAMPLE_FPS)) as usize + 1;
        let mut cmd = ffmpeg_command("ffmpeg");
        cmd.args(["-i", path_str(path)?, "-an"])
            .args([
                "-vf",
                &format!("fps={SAMPLE_FPS},scale={aw}:{ah},format=gray"),
            ])
            .args(["-f", "rawvideo", "-"]);
        let out = run_bounded(cmd, None, FRAMES_TIMEOUT, frame_bytes * max_frames)?;
        if !out.success() {
            return Err(CoreError::Execution(format!(
                "ffmpeg frame extraction failed: {}",
                out.stderr.trim()
            )));
        }
        if out.stdout.is_empty() || out.stdout.len() % frame_bytes != 0 {
            return Err(CoreError::Execution(
                "ffmpeg returned a truncated or empty frame stream".to_string(),
            ));
        }
        Ok(FrameSet {
            width: aw,
            height: ah,
            source_width: meta.width,
            source_height: meta.height,
            sample_fps: SAMPLE_FPS,
            frames: out.stdout.chunks(frame_bytes).map(<[u8]>::to_vec).collect(),
        })
    }

    /// Decode one frame at `t_ms` as PNG at the analysis resolution.
    pub fn extract_png_frame(path: &Path, meta: &VideoMetadata, t_ms: u64) -> Result<Vec<u8>> {
        let (aw, ah) = Self::analysis_size(meta);
        let seek = format!("{}.{:03}", t_ms / 1000, t_ms % 1000);
        let mut cmd = ffmpeg_command("ffmpeg");
        cmd.args(["-ss", &seek, "-i", path_str(path)?, "-an", "-frames:v", "1"])
            .args(["-vf", &format!("scale={aw}:{ah}")])
            .args(["-f", "image2pipe", "-c:v", "png", "-"]);
        let out = run_bounded(cmd, None, STILL_TIMEOUT, MAX_STILL_BYTES)?;
        if !out.success() || out.stdout.is_empty() {
            return Err(CoreError::Execution(format!(
                "ffmpeg could not extract a frame at {t_ms} ms: {}",
                out.stderr.trim()
            )));
        }
        Ok(out.stdout)
    }

    /// Write at most [`MAX_KEYFRAMES`] PNG keyframes into `dir`, returning their paths.
    pub fn write_keyframes(
        path: &Path,
        meta: &VideoMetadata,
        times_ms: &[u64],
        dir: &Path,
    ) -> Result<Vec<PathBuf>> {
        if times_ms.len() > MAX_KEYFRAMES {
            return Err(CoreError::Execution(format!(
                "Refusing to extract {} keyframes; the budget is {MAX_KEYFRAMES}",
                times_ms.len()
            )));
        }
        fs::create_dir_all(dir)?;
        let mut written = Vec::with_capacity(times_ms.len());
        for t_ms in times_ms {
            let png = Self::extract_png_frame(path, meta, *t_ms)?;
            let file = dir.join(format!("keyframe_{t_ms:06}ms.png"));
            fs::write(&file, png)?;
            written.push(file);
        }
        Ok(written)
    }

    /// Extract the audio track as 24 kHz mono 16-bit PCM WAV, or `None` without audio.
    pub fn extract_wav_24k_mono(path: &Path, meta: &VideoMetadata) -> Result<Option<Vec<u8>>> {
        if !meta.has_audio {
            return Ok(None);
        }
        let mut cmd = ffmpeg_command("ffmpeg");
        cmd.args(["-i", path_str(path)?, "-vn", "-ac", "1", "-ar", "24000"])
            .args(["-c:a", "pcm_s16le", "-f", "wav", "-"]);
        let out = run_bounded(cmd, None, AUDIO_TIMEOUT, MAX_WAV_BYTES)?;
        if !out.success() || out.stdout.is_empty() {
            return Err(CoreError::Execution(format!(
                "ffmpeg audio extraction failed: {}",
                out.stderr.trim()
            )));
        }
        Ok(Some(out.stdout))
    }

    /// Synthesize a deterministic 1.0s 1280x720 MP4 recording with a 24 kHz mono audio
    /// narration track and a visual button click highlight centered at `(160, 120)`
    /// (`x=100..220`, `y=100..140`) starting at `t = 400ms`.
    pub fn synthesize_demo_video(output_path: &Path) -> Result<()> {
        if let Some(parent) = output_path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        let mut cmd = ffmpeg_command("ffmpeg");
        cmd.arg("-y").args([
            "-f",
            "lavfi",
            "-i",
            "color=c=0x1e1e1e:s=1280x720:r=10:d=1.0",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:sample_rate=24000:d=1.0",
            "-vf",
            "drawbox=x=100:y=100:w=120:h=40:color=white@1.0:t=fill:enable='gte(t,0.4)'",
            "-pix_fmt",
            "yuv420p",
            "-shortest",
            path_str(output_path)?,
        ]);
        let out = run_bounded(cmd, None, AUDIO_TIMEOUT, MAX_PROBE_BYTES)?;
        if !out.success() {
            return Err(CoreError::Execution(format!(
                "ffmpeg demo video synthesis failed: {}",
                out.stderr.trim()
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const W: u32 = 640;
    const H: u32 = 480;

    fn blank() -> Vec<u8> {
        vec![30u8; (W * H) as usize]
    }

    fn with_block(base: &[u8], x0: u32, y0: u32, side: u32) -> Vec<u8> {
        let mut frame = base.to_vec();
        for y in y0..y0 + side {
            for x in x0..x0 + side {
                frame[(y * W + x) as usize] = 220;
            }
        }
        frame
    }

    fn set(frames: Vec<Vec<u8>>) -> FrameSet {
        FrameSet {
            width: W,
            height: H,
            source_width: W,
            source_height: H,
            sample_fps: 10,
            frames,
        }
    }

    #[test]
    fn test_parse_fps_and_duration() {
        assert_eq!(parse_fps_milli("30000/1001"), Some(29_970));
        assert_eq!(parse_fps_milli("10/1"), Some(10_000));
        assert_eq!(parse_fps_milli("25"), Some(25_000));
        assert_eq!(parse_fps_milli("0/0"), None);
        assert_eq!(parse_duration_ms("2.000000"), Some(2_000));
        assert_eq!(parse_duration_ms("0.5"), Some(500));
        assert_eq!(parse_duration_ms("abc"), None);
    }

    #[test]
    fn test_detects_single_interaction_centroid_and_time() {
        let a = blank();
        let b = with_block(&a, 200, 100, 8);
        let frames = set(vec![a.clone(), a.clone(), b.clone(), b]);
        let hit = RoiDiffer::detect_interaction(&frames, DetectionParams::default()).unwrap();
        assert_eq!(hit.t_ms, 200);
        assert_eq!((hit.x_video, hit.y_video), (203, 103));
        assert_eq!(hit.score_margin_bps, 10_000);
        assert!(hit.roi_changed_bps >= DEFAULT_TAU_BPS);
    }

    #[test]
    fn test_static_video_and_codec_noise_fail_closed() {
        let a = blank();
        let noisy: Vec<u8> = a.iter().map(|p| p + 5).collect();
        let still = set(vec![a.clone(), a.clone(), a.clone()]);
        let err = RoiDiffer::detect_interaction(&still, DetectionParams::default()).unwrap_err();
        assert!(matches!(err, CoreError::Grounding(_)));
        let noise = set(vec![a, noisy]);
        assert!(RoiDiffer::detect_interaction(&noise, DetectionParams::default()).is_err());
    }

    #[test]
    fn test_two_equal_interactions_are_ambiguous() {
        let a = blank();
        let b = with_block(&a, 20, 20, 40);
        let c = with_block(&b, 500, 400, 40);
        let frames = set(vec![a, b.clone(), b, c]);
        let err = RoiDiffer::detect_interaction(&frames, DetectionParams::default()).unwrap_err();
        assert!(
            matches!(&err, CoreError::Grounding(m) if m.contains("Ambiguous")),
            "got {err:?}"
        );
    }

    #[test]
    fn test_dominant_interaction_beats_small_distractor() {
        let a = blank();
        let small = with_block(&a, 20, 20, 20);
        let big = with_block(&small, 400, 300, 60);
        let frames = set(vec![a, small.clone(), small, big]);
        let hit = RoiDiffer::detect_interaction(&frames, DetectionParams::default()).unwrap();
        assert_eq!((hit.x_video, hit.y_video), (429, 329));
        assert!(hit.score_margin_bps >= AMBIGUITY_MARGIN_BPS);
    }

    #[test]
    fn test_too_few_frames_fail_closed() {
        assert!(
            RoiDiffer::detect_interaction(&set(vec![blank()]), DetectionParams::default()).is_err()
        );
    }

    #[test]
    fn test_calibration_identity_dpr_scale_and_letterbox() {
        let identity = GeometricCalibration {
            video_width: 1280,
            video_height: 720,
            viewport_width: 1280,
            viewport_height: 720,
            dpr_milli: 1000,
        };
        assert_eq!(identity.to_css(160, 120).unwrap(), (160, 120));

        // Retina recording: 2x device pixels in a 2560x1440 video.
        let retina = GeometricCalibration {
            video_width: 2560,
            video_height: 1440,
            viewport_width: 1280,
            viewport_height: 720,
            dpr_milli: 2000,
        };
        assert_eq!(retina.to_css(320, 240).unwrap(), (160, 120));

        // 1280x720 viewport letterboxed into a 1280x960 video: 120 px bars top and bottom.
        let boxed = GeometricCalibration {
            video_width: 1280,
            video_height: 960,
            viewport_width: 1280,
            viewport_height: 720,
            dpr_milli: 1000,
        };
        assert_eq!(boxed.to_css(160, 240).unwrap(), (160, 120));
        let err = boxed.to_css(160, 50).unwrap_err();
        assert!(matches!(err, CoreError::Grounding(m) if m.contains("letterbox")));
    }

    #[test]
    fn test_calibration_rejects_zero_geometry() {
        let bad = GeometricCalibration {
            video_width: 0,
            video_height: 720,
            viewport_width: 1280,
            viewport_height: 720,
            dpr_milli: 1000,
        };
        assert!(bad.to_css(1, 1).is_err());
    }

    #[test]
    fn test_validate_source_rejects_unsafe_and_unsupported_inputs() {
        assert!(matches!(
            VideoPreprocessor::validate_source("../../etc/passwd.mp4"),
            Err(CoreError::SecurityViolation(_))
        ));
        assert!(matches!(
            VideoPreprocessor::validate_source("clip.avi"),
            Err(CoreError::Manifest(_))
        ));
        assert!(matches!(
            VideoPreprocessor::validate_source("definitely-missing.mp4"),
            Err(CoreError::Manifest(_))
        ));
    }

    #[test]
    fn test_analysis_size_is_even_and_bounded() {
        let meta = VideoMetadata {
            width: 1920,
            height: 1080,
            duration_ms: 1000,
            fps_milli: 30_000,
            has_audio: false,
            container: "mp4".into(),
        };
        assert_eq!(VideoPreprocessor::analysis_size(&meta), (640, 360));
    }

    #[test]
    fn test_keyframe_budget_is_enforced() {
        let meta = VideoMetadata {
            width: 64,
            height: 48,
            duration_ms: 1000,
            fps_milli: 10_000,
            has_audio: false,
            container: "mp4".into(),
        };
        let err = VideoPreprocessor::write_keyframes(
            Path::new("none.mp4"),
            &meta,
            &[0; MAX_KEYFRAMES + 1],
            &std::env::temp_dir().join("replayfix-keyframe-budget"),
        )
        .unwrap_err();
        assert!(matches!(err, CoreError::Execution(_)));
    }

    #[test]
    fn test_ffmpeg_synthesize_probe_extract_frames_keyframes_and_audio() {
        let temp_dir =
            std::env::temp_dir().join(format!("replayfix-video-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();
        let mp4_path = temp_dir.join("rf01_demo.mp4");

        VideoPreprocessor::synthesize_demo_video(&mp4_path).unwrap();
        let meta = VideoPreprocessor::probe_video(&mp4_path).unwrap();
        assert_eq!((meta.width, meta.height), (1280, 720));
        assert!(meta.has_audio);
        assert!(meta.duration_ms >= 900 && meta.duration_ms <= 1200);

        let frames = VideoPreprocessor::extract_gray_frames(&mp4_path, &meta).unwrap();
        assert_eq!((frames.width, frames.height), (640, 360));
        let hit = RoiDiffer::detect_interaction(&frames, DetectionParams::default()).unwrap();
        // Button box is x=100..220 (center 160), y=100..140 (center 120)
        assert!(
            (hit.x_video - 160).abs() <= 6 && (hit.y_video - 120).abs() <= 6,
            "unexpected hotspot: ({}, {})",
            hit.x_video,
            hit.y_video
        );
        assert!(
            hit.t_ms >= 300 && hit.t_ms <= 600,
            "unexpected t_ms: {}",
            hit.t_ms
        );

        let kf_dir = temp_dir.join("keyframes");
        let kfs =
            VideoPreprocessor::write_keyframes(&mp4_path, &meta, &[hit.t_ms], &kf_dir).unwrap();
        assert_eq!(kfs.len(), 1);
        assert!(kfs[0].exists());

        let wav = VideoPreprocessor::extract_wav_24k_mono(&mp4_path, &meta)
            .unwrap()
            .expect("audio track present");
        assert!(wav.starts_with(b"RIFF"));
        assert!(wav.len() > 1000);

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
