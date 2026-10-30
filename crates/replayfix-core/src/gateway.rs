//! Spec 095: Bounded Live Meta Multimodal Cloud Gateway, Token-Bucket Rate Pool,
//! Three-State Circuit Breaker, Baseten Open-Weight Fallback, Zero-Audio Rule,
//! and Verified 24 kHz WAV Temporal Alignment.

use crate::digest::{canonical_json, sha256_hex};
use crate::error::{CoreError, Result};
use crate::repair::{MuseModelSlug, TokenAccounting};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Official Meta Model API responses endpoint.
pub const META_RESPONSES_ENDPOINT: &str = "https://api.meta.ai/v1/responses";

/// Official Baseten hosted fallback endpoint for Muse Spark Open.
pub const BASETEN_FALLBACK_ENDPOINT: &str = "https://inference.baseten.co/v1";

/// Canonical model slug for Baseten Muse Spark Open fallback.
pub const BASETEN_MUSE_SPARK_OPEN_SLUG: &str = "meta-models/muse-spark-open";

/// Default per-request timeout in milliseconds (15 seconds).
pub const DEFAULT_GATEWAY_TIMEOUT_MS: u64 = 15_000;

/// Default maximum retry count for transient HTTP 429 / 5xx errors.
pub const DEFAULT_MAX_RETRIES: u32 = 2;

/// Default consecutive failure threshold to trip the circuit breaker.
pub const DEFAULT_CIRCUIT_FAILURE_THRESHOLD: u32 = 3;

/// Default circuit breaker open cooldown in milliseconds (30 seconds).
pub const DEFAULT_CIRCUIT_COOLDOWN_MS: u64 = 30_000;

/// Default audio-visual temporal alignment tolerance in milliseconds (+/- 250ms).
pub const DEFAULT_ALIGNMENT_TOLERANCE_MS: u64 = 250;

/// Maximum permitted audio duration in milliseconds (60 seconds).
pub const MAX_AUDIO_DURATION_MS: u64 = 60_000;

/// Upstream provider serving a gateway request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GatewayProvider {
    MetaAi,
    BasetenFallback,
    MockOffline,
}

impl GatewayProvider {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MetaAi => "meta_ai",
            Self::BasetenFallback => "baseten_fallback",
            Self::MockOffline => "mock_offline",
        }
    }
}

/// Functional role of a model in the ReplayFix multimodal architecture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelRole {
    /// Complex multi-file reasoning and causal synthesis (`muse-wonder-1.0`).
    DeepReasoning,
    /// Fast candidate patch synthesis (`muse-spark-1.3`, `meta-models/muse-spark-open`).
    FastSynthesis,
    /// Visual prompt segmentation and bounding-box snapping (`sam-3-1` / `sam-3.1`).
    SpatialSegmentation,
    /// Speech transcription and timestamp alignment (`muse-voice-transcribe-1.0`).
    SpeechTranscription,
}

/// Explicit provenance distinguishing live cloud responses from deterministic offline fixtures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GatewayProvenance {
    LiveProvider,
    DeterministicFixture,
}

/// Configuration for the Multimodal Cloud Gateway with automatic secret redaction.
#[derive(Clone, PartialEq, Eq)]
pub struct GatewayConfig {
    pub meta_endpoint_url: String,
    pub baseten_fallback_url: String,
    meta_api_key: Option<String>,
    baseten_api_key: Option<String>,
    pub timeout_ms: u64,
    pub max_retries: u32,
    pub rpm_limit: u32,
    pub tpm_limit: u64,
    pub max_in_flight: usize,
    pub circuit_failure_threshold: u32,
    pub circuit_cooldown_ms: u64,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            meta_endpoint_url: META_RESPONSES_ENDPOINT.to_string(),
            baseten_fallback_url: BASETEN_FALLBACK_ENDPOINT.to_string(),
            meta_api_key: std::env::var("META_API_KEY")
                .ok()
                .filter(|s| !s.trim().is_empty()),
            baseten_api_key: std::env::var("BASETEN_API_KEY")
                .ok()
                .filter(|s| !s.trim().is_empty()),
            timeout_ms: DEFAULT_GATEWAY_TIMEOUT_MS,
            max_retries: DEFAULT_MAX_RETRIES,
            rpm_limit: 9_000,
            tpm_limit: 12_000_000,
            max_in_flight: 3,
            circuit_failure_threshold: DEFAULT_CIRCUIT_FAILURE_THRESHOLD,
            circuit_cooldown_ms: DEFAULT_CIRCUIT_COOLDOWN_MS,
        }
    }
}

impl fmt::Debug for GatewayConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GatewayConfig")
            .field("meta_endpoint_url", &self.meta_endpoint_url)
            .field("baseten_fallback_url", &self.baseten_fallback_url)
            .field(
                "meta_api_key",
                &self.meta_api_key.as_ref().map(|_| "[REDACTED]"),
            )
            .field(
                "baseten_api_key",
                &self.baseten_api_key.as_ref().map(|_| "[REDACTED]"),
            )
            .field("timeout_ms", &self.timeout_ms)
            .field("max_retries", &self.max_retries)
            .field("rpm_limit", &self.rpm_limit)
            .field("tpm_limit", &self.tpm_limit)
            .field("max_in_flight", &self.max_in_flight)
            .field("circuit_failure_threshold", &self.circuit_failure_threshold)
            .field("circuit_cooldown_ms", &self.circuit_cooldown_ms)
            .finish()
    }
}

impl GatewayConfig {
    /// Set the Meta API key explicitly (redacted in all Debug/error output).
    #[must_use]
    pub fn with_meta_api_key(mut self, key: impl Into<String>) -> Self {
        let k = key.into();
        self.meta_api_key = if k.trim().is_empty() { None } else { Some(k) };
        self
    }

    /// Set the Baseten fallback API key explicitly (redacted in all Debug/error output).
    #[must_use]
    pub fn with_baseten_api_key(mut self, key: impl Into<String>) -> Self {
        let k = key.into();
        self.baseten_api_key = if k.trim().is_empty() { None } else { Some(k) };
        self
    }

    /// Check whether a Meta API key is configured without exposing its value.
    #[must_use]
    pub fn has_meta_api_key(&self) -> bool {
        self.meta_api_key.is_some()
    }

    /// Check whether a Baseten fallback API key is configured without exposing its value.
    #[must_use]
    pub fn has_baseten_api_key(&self) -> bool {
        self.baseten_api_key.is_some()
    }

    /// Redact any occurrence of configured credentials from an arbitrary diagnostic string.
    #[must_use]
    pub fn redact_secrets(&self, message: &str) -> String {
        let mut sanitized = message.to_string();
        if let Some(ref key) = self.meta_api_key {
            if !key.is_empty() {
                sanitized = sanitized.replace(key, "[REDACTED]");
            }
        }
        if let Some(ref key) = self.baseten_api_key {
            if !key.is_empty() {
                sanitized = sanitized.replace(key, "[REDACTED]");
            }
        }
        sanitized
    }
}

/// Bounded token-bucket rate limiter enforcing RPM, TPM, and in-flight concurrency limits.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RatePool {
    pub rpm_limit: u32,
    pub tpm_limit: u64,
    pub max_in_flight: usize,
    pub requests_used: u32,
    pub tokens_used: u64,
    pub current_in_flight: usize,
    pub throttled_count: u32,
}

impl RatePool {
    #[must_use]
    pub const fn new(rpm_limit: u32, tpm_limit: u64, max_in_flight: usize) -> Self {
        Self {
            rpm_limit,
            tpm_limit,
            max_in_flight,
            requests_used: 0,
            tokens_used: 0,
            current_in_flight: 0,
            throttled_count: 0,
        }
    }

    /// Attempt to acquire capacity for 1 request and `estimated_tokens` tokens.
    pub fn try_acquire(&mut self, estimated_tokens: u64) -> Result<()> {
        if self.current_in_flight >= self.max_in_flight {
            self.throttled_count = self.throttled_count.saturating_add(1);
            return Err(CoreError::ModelApiError(format!(
                "RATE_LIMIT_IN_FLIGHT_EXCEEDED: active in-flight requests ({}) reached max_in_flight ({})",
                self.current_in_flight, self.max_in_flight
            )));
        }
        if self.requests_used >= self.rpm_limit {
            self.throttled_count = self.throttled_count.saturating_add(1);
            return Err(CoreError::ModelApiError(format!(
                "RATE_LIMIT_RPM_EXCEEDED: requests_used ({}) reached rpm_limit ({})",
                self.requests_used, self.rpm_limit
            )));
        }
        if self.tokens_used.saturating_add(estimated_tokens) > self.tpm_limit {
            self.throttled_count = self.throttled_count.saturating_add(1);
            return Err(CoreError::ModelApiError(format!(
                "RATE_LIMIT_TPM_EXCEEDED: tokens ({} + {}) exceed tpm_limit ({})",
                self.tokens_used, estimated_tokens, self.tpm_limit
            )));
        }
        self.requests_used = self.requests_used.saturating_add(1);
        self.tokens_used = self.tokens_used.saturating_add(estimated_tokens);
        self.current_in_flight = self.current_in_flight.saturating_add(1);
        Ok(())
    }

    /// Release an in-flight slot and adjust actual token consumption.
    pub fn release(&mut self, estimated_tokens: u64, actual_tokens: u64) {
        self.current_in_flight = self.current_in_flight.saturating_sub(1);
        if actual_tokens > estimated_tokens {
            self.tokens_used = self
                .tokens_used
                .saturating_add(actual_tokens - estimated_tokens);
        } else {
            self.tokens_used = self
                .tokens_used
                .saturating_sub(estimated_tokens - actual_tokens);
        }
    }

    /// Reset minute-window counters.
    pub fn reset_window(&mut self) {
        self.requests_used = 0;
        self.tokens_used = 0;
    }

    /// Compute retry delay in milliseconds, honoring `Retry-After` (in seconds) when provided
    /// or falling back to bounded exponential backoff with deterministic jitter.
    #[must_use]
    pub fn compute_retry_delay_ms(attempt: u32, retry_after_header: Option<&str>) -> u64 {
        if let Some(raw) = retry_after_header {
            if let Ok(secs) = raw.trim().parse::<u64>() {
                return secs.saturating_mul(1_000).min(30_000);
            }
        }
        let shift = attempt.min(5);
        let base_ms = 250u64.saturating_mul(1u64 << shift);
        let jitter_ms = u64::from(attempt).saturating_mul(37) % 120;
        base_ms.saturating_add(jitter_ms).min(10_000)
    }
}

/// Three-state circuit breaker state machine (`Closed -> Open -> HalfOpen`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

/// Circuit breaker protecting upstream model endpoints from cascading outages.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CircuitBreaker {
    pub state: CircuitState,
    pub consecutive_failures: u32,
    pub failure_threshold: u32,
    pub cooldown_ms: u64,
    pub opened_at_ms: Option<u64>,
}

impl CircuitBreaker {
    #[must_use]
    pub const fn new(failure_threshold: u32, cooldown_ms: u64) -> Self {
        Self {
            state: CircuitState::Closed,
            consecutive_failures: 0,
            failure_threshold: if failure_threshold == 0 {
                1
            } else {
                failure_threshold
            },
            cooldown_ms,
            opened_at_ms: None,
        }
    }

    /// Evaluate whether a request is permitted at timestamp `now_ms`, transitioning
    /// from `Open` to `HalfOpen` if the cooldown window has elapsed.
    pub fn allow_request(&mut self, now_ms: u64) -> bool {
        match self.state {
            CircuitState::Closed | CircuitState::HalfOpen => true,
            CircuitState::Open => {
                if let Some(opened_at) = self.opened_at_ms {
                    if now_ms.saturating_sub(opened_at) >= self.cooldown_ms {
                        self.state = CircuitState::HalfOpen;
                        return true;
                    }
                }
                false
            }
        }
    }

    /// Record a successful upstream call, closing the circuit and resetting failures.
    pub fn record_success(&mut self) {
        self.consecutive_failures = 0;
        self.state = CircuitState::Closed;
        self.opened_at_ms = None;
    }

    /// Record a retryable failure (`HTTP 429`, `HTTP 5xx`, timeout, or connection failure).
    pub fn record_failure(&mut self, now_ms: u64) {
        self.consecutive_failures = self.consecutive_failures.saturating_add(1);
        if self.state == CircuitState::HalfOpen
            || self.consecutive_failures >= self.failure_threshold
        {
            self.state = CircuitState::Open;
            self.opened_at_ms = Some(now_ms);
        }
    }
}

/// Validated PCM WAV header metadata for `muse-voice-transcribe-1.0`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WavAudioMetadata {
    pub sample_rate_hz: u32,
    pub channels: u16,
    pub bits_per_sample: u16,
    pub data_bytes: usize,
    pub duration_ms: u64,
    pub sha256: String,
    pub is_24khz_mono: bool,
}

/// Inspect and validate a RIFF/WAVE PCM header without relabeling source sample rates.
pub fn validate_wav_audio(bytes: &[u8]) -> Result<WavAudioMetadata> {
    if bytes.len() < 44 {
        return Err(CoreError::Verification(
            "WAV audio payload too short (< 44 bytes)".to_string(),
        ));
    }
    if &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" || &bytes[12..16] != b"fmt " {
        return Err(CoreError::Verification(
            "Invalid RIFF/WAVE header magic bytes".to_string(),
        ));
    }

    let audio_format = u16::from_le_bytes([bytes[20], bytes[21]]);
    if audio_format != 1 {
        return Err(CoreError::Verification(format!(
            "Unsupported WAV audio format {audio_format}: expected uncompressed PCM (1)"
        )));
    }

    let channels = u16::from_le_bytes([bytes[22], bytes[23]]);
    let sample_rate_hz = u32::from_le_bytes([bytes[24], bytes[25], bytes[26], bytes[27]]);
    let bits_per_sample = u16::from_le_bytes([bytes[34], bytes[35]]);
    let data_bytes = u32::from_le_bytes([bytes[40], bytes[41], bytes[42], bytes[43]]) as usize;

    if channels == 0 || sample_rate_hz == 0 || bits_per_sample == 0 {
        return Err(CoreError::Verification(
            "WAV header contains zero channels, sample_rate_hz, or bits_per_sample".to_string(),
        ));
    }

    let bytes_per_sec = u64::from(sample_rate_hz)
        .saturating_mul(u64::from(channels))
        .saturating_mul(u64::from(bits_per_sample) / 8);
    let duration_ms = (data_bytes as u64)
        .saturating_mul(1_000)
        .checked_div(bytes_per_sec)
        .unwrap_or(0);

    if duration_ms > MAX_AUDIO_DURATION_MS {
        return Err(CoreError::Verification(format!(
            "Audio duration ({duration_ms}ms) exceeds maximum permitted {MAX_AUDIO_DURATION_MS}ms"
        )));
    }

    Ok(WavAudioMetadata {
        sample_rate_hz,
        channels,
        bits_per_sample,
        data_bytes,
        duration_ms,
        sha256: sha256_hex(bytes),
        is_24khz_mono: sample_rate_hz == 24_000 && channels == 1 && bits_per_sample == 16,
    })
}

/// Spoken transcript segment with start/end millisecond timestamps.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TranscriptSegment {
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: String,
}

/// Result of aligning a transcript segment with a visual keyframe timestamp.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyframeAlignment {
    pub keyframe_timestamp_ms: u64,
    pub matched_segment: Option<TranscriptSegment>,
    pub delta_ms: Option<u64>,
    pub within_tolerance: bool,
}

/// Correlate visual keyframe timestamps with transcribed speech segments within `tolerance_ms`.
#[must_use]
pub fn align_transcript_with_keyframes(
    segments: &[TranscriptSegment],
    keyframe_timestamps_ms: &[u64],
    tolerance_ms: u64,
) -> Vec<KeyframeAlignment> {
    keyframe_timestamps_ms
        .iter()
        .map(|&kf_ms| {
            let mut best: Option<(&TranscriptSegment, u64)> = None;
            for seg in segments {
                let dist = if kf_ms >= seg.start_ms && kf_ms <= seg.end_ms {
                    0
                } else if kf_ms < seg.start_ms {
                    seg.start_ms - kf_ms
                } else {
                    kf_ms - seg.end_ms
                };
                if best.is_none_or(|(_, b_dist)| dist < b_dist) {
                    best = Some((seg, dist));
                }
            }
            match best {
                Some((seg, dist)) if dist <= tolerance_ms => KeyframeAlignment {
                    keyframe_timestamp_ms: kf_ms,
                    matched_segment: Some(seg.clone()),
                    delta_ms: Some(dist),
                    within_tolerance: true,
                },
                Some((seg, dist)) => KeyframeAlignment {
                    keyframe_timestamp_ms: kf_ms,
                    matched_segment: Some(seg.clone()),
                    delta_ms: Some(dist),
                    within_tolerance: false,
                },
                None => KeyframeAlignment {
                    keyframe_timestamp_ms: kf_ms,
                    matched_segment: None,
                    delta_ms: None,
                    within_tolerance: false,
                },
            }
        })
        .collect()
}

/// Structured request dispatched through the Multimodal Cloud Gateway.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GatewayRequest {
    pub role: ModelRole,
    pub requested_model: String,
    pub prompt: String,
    pub reasoning_effort: Option<String>,
    pub estimated_tokens: u64,
    pub include_audio: bool,
    pub audio_wav_bytes: Option<Vec<u8>>,
    pub image_sha256: Option<String>,
}

impl GatewayRequest {
    /// Validate model allowlist, role compatibility, and the Zero-Audio Rule.
    pub fn validate(&self) -> Result<()> {
        let is_text_model = matches!(
            self.requested_model.as_str(),
            "muse-wonder-1.0"
                | "muse-spark-1.3"
                | "muse-spark-1.3-contributor"
                | BASETEN_MUSE_SPARK_OPEN_SLUG
                | "meta-models/muse-spark-open-1026"
        );
        let is_sam_model = matches!(self.requested_model.as_str(), "sam-3-1" | "sam-3.1");
        let is_asr_model = self.requested_model == "muse-voice-transcribe-1.0";

        if !is_text_model && !is_sam_model && !is_asr_model {
            return Err(CoreError::Verification(format!(
                "Disallowed or unrecognized gateway model slug: '{}'",
                self.requested_model
            )));
        }

        // Law VI: Zero-Audio Rule enforcement on all text/code and segmentation models
        if !is_asr_model && (self.include_audio || self.audio_wav_bytes.is_some()) {
            return Err(CoreError::SecurityViolation(format!(
                "ZERO_AUDIO_RULE_VIOLATION: Audio payload is forbidden for non-transcription model '{}'",
                self.requested_model
            )));
        }

        // Enforce strict role-to-model compatibility
        match self.role {
            ModelRole::DeepReasoning | ModelRole::FastSynthesis => {
                if !is_text_model {
                    return Err(CoreError::Verification(format!(
                        "Model '{}' cannot fulfill text/code synthesis role {:?}",
                        self.requested_model, self.role
                    )));
                }
            }
            ModelRole::SpatialSegmentation => {
                if !is_sam_model {
                    return Err(CoreError::Verification(format!(
                        "Model '{}' cannot fulfill SpatialSegmentation role",
                        self.requested_model
                    )));
                }
            }
            ModelRole::SpeechTranscription => {
                if !is_asr_model {
                    return Err(CoreError::Verification(format!(
                        "Model '{}' cannot fulfill SpeechTranscription role",
                        self.requested_model
                    )));
                }
                if let Some(ref wav) = self.audio_wav_bytes {
                    let meta = validate_wav_audio(wav)?;
                    if !meta.is_24khz_mono {
                        return Err(CoreError::Verification(format!(
                            "ASR audio must be 24000 Hz 16-bit mono PCM (found {} Hz, {} ch, {} bit)",
                            meta.sample_rate_hz, meta.channels, meta.bits_per_sample
                        )));
                    }
                }
            }
        }

        Ok(())
    }

    /// Compute canonical SHA-256 fingerprint of the request configuration.
    pub fn fingerprint_sha256(&self) -> Result<String> {
        let val = serde_json::json!({
            "role": self.role,
            "requested_model": self.requested_model,
            "prompt_sha256": sha256_hex(self.prompt.as_bytes()),
            "reasoning_effort": self.reasoning_effort,
            "include_audio": self.include_audio,
            "image_sha256": self.image_sha256,
        });
        let canon = canonical_json(&val)?;
        Ok(sha256_hex(canon.as_bytes()))
    }
}

/// Structured response returned by the Multimodal Cloud Gateway.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GatewayResponse {
    pub provider: GatewayProvider,
    pub provenance: GatewayProvenance,
    pub requested_model: String,
    pub serving_model: String,
    pub output_text: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cached_tokens: u64,
    pub cost_nanos: u64,
    pub latency_ms: u64,
    pub retries_attempted: u32,
    pub fallback_triggered: bool,
    pub request_fingerprint_sha256: String,
}

/// Simulated or live transport outcome for a single upstream HTTP attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportOutcome {
    Success {
        serving_model: String,
        output_text: String,
        input_tokens: u64,
        output_tokens: u64,
        cached_tokens: u64,
        latency_ms: u64,
        provenance: GatewayProvenance,
    },
    RateLimited429 {
        retry_after: Option<String>,
    },
    ServerError5xx {
        status_code: u16,
        message: String,
    },
    Timeout,
    AuthError401 {
        message: String,
    },
}

/// Bounded Multimodal Cloud Gateway coordinating rate limits, retries, circuit breaking,
/// and fallback routing.
#[derive(Debug, Clone)]
pub struct MultimodalGateway {
    pub config: GatewayConfig,
    pub rate_pool: RatePool,
    pub circuit_breaker: CircuitBreaker,
}

impl MultimodalGateway {
    #[must_use]
    pub fn new(config: GatewayConfig) -> Self {
        let rate_pool = RatePool::new(config.rpm_limit, config.tpm_limit, config.max_in_flight);
        let circuit_breaker =
            CircuitBreaker::new(config.circuit_failure_threshold, config.circuit_cooldown_ms);
        Self {
            config,
            rate_pool,
            circuit_breaker,
        }
    }

    /// Execute a request through the gateway using the provided transport closure,
    /// enforcing validation, Zero-Audio Rule, rate limits, circuit breaking, retries,
    /// and fallback eligibility.
    pub fn execute_with_transport<F>(
        &mut self,
        request: &GatewayRequest,
        now_ms: u64,
        mut transport: F,
    ) -> Result<GatewayResponse>
    where
        F: FnMut(GatewayProvider, &str, u32) -> TransportOutcome,
    {
        request.validate()?;
        let fingerprint = request.fingerprint_sha256()?;
        self.rate_pool.try_acquire(request.estimated_tokens)?;

        let primary_allowed = self.circuit_breaker.allow_request(now_ms);
        let mut retries_attempted = 0u32;

        if primary_allowed {
            let max_attempts = self.config.max_retries.saturating_add(1);
            for attempt in 0..max_attempts {
                if attempt > 0 {
                    retries_attempted = retries_attempted.saturating_add(1);
                }
                match transport(GatewayProvider::MetaAi, &request.requested_model, attempt) {
                    TransportOutcome::Success {
                        serving_model,
                        output_text,
                        input_tokens,
                        output_tokens,
                        cached_tokens,
                        latency_ms,
                        provenance,
                    } => {
                        self.circuit_breaker.record_success();
                        let total_tokens = input_tokens.saturating_add(output_tokens);
                        self.rate_pool
                            .release(request.estimated_tokens, total_tokens);
                        let slug = MuseModelSlug::parse(&serving_model)
                            .unwrap_or(MuseModelSlug::MuseSpark13);
                        let cost_nanos = TokenAccounting::usage_nanos(
                            slug,
                            input_tokens,
                            cached_tokens,
                            output_tokens,
                            0,
                        );
                        return Ok(GatewayResponse {
                            provider: if provenance == GatewayProvenance::LiveProvider {
                                GatewayProvider::MetaAi
                            } else {
                                GatewayProvider::MockOffline
                            },
                            provenance,
                            requested_model: request.requested_model.clone(),
                            serving_model,
                            output_text,
                            input_tokens,
                            output_tokens,
                            cached_tokens,
                            cost_nanos,
                            latency_ms,
                            retries_attempted,
                            fallback_triggered: false,
                            request_fingerprint_sha256: fingerprint,
                        });
                    }
                    TransportOutcome::AuthError401 { message } => {
                        // Auth failures must never trigger retries or trip the circuit breaker
                        self.rate_pool.release(request.estimated_tokens, 0);
                        let safe_msg = self.config.redact_secrets(&message);
                        return Err(CoreError::ModelApiError(format!(
                            "GATEWAY_AUTH_FAILED: {safe_msg}"
                        )));
                    }
                    TransportOutcome::RateLimited429 { retry_after } => {
                        let _delay =
                            RatePool::compute_retry_delay_ms(attempt, retry_after.as_deref());
                        self.circuit_breaker.record_failure(now_ms);
                        if self.circuit_breaker.state == CircuitState::Open {
                            break;
                        }
                    }
                    TransportOutcome::ServerError5xx { .. } | TransportOutcome::Timeout => {
                        self.circuit_breaker.record_failure(now_ms);
                        if self.circuit_breaker.state == CircuitState::Open {
                            break;
                        }
                    }
                }
            }
        }

        // Primary circuit is Open or retries exhausted: check if eligible for BasetenFallback
        let eligible_for_baseten = matches!(
            request.role,
            ModelRole::DeepReasoning | ModelRole::FastSynthesis
        );

        if eligible_for_baseten {
            match transport(
                GatewayProvider::BasetenFallback,
                BASETEN_MUSE_SPARK_OPEN_SLUG,
                retries_attempted,
            ) {
                TransportOutcome::Success {
                    serving_model,
                    output_text,
                    input_tokens,
                    output_tokens,
                    cached_tokens,
                    latency_ms,
                    provenance,
                } => {
                    let total_tokens = input_tokens.saturating_add(output_tokens);
                    self.rate_pool
                        .release(request.estimated_tokens, total_tokens);
                    let slug = MuseModelSlug::parse(&serving_model)
                        .unwrap_or(MuseModelSlug::MuseSparkOpen1026);
                    let cost_nanos = TokenAccounting::usage_nanos(
                        slug,
                        input_tokens,
                        cached_tokens,
                        output_tokens,
                        0,
                    );
                    return Ok(GatewayResponse {
                        provider: GatewayProvider::BasetenFallback,
                        provenance,
                        requested_model: request.requested_model.clone(),
                        serving_model,
                        output_text,
                        input_tokens,
                        output_tokens,
                        cached_tokens,
                        cost_nanos,
                        latency_ms,
                        retries_attempted,
                        fallback_triggered: true,
                        request_fingerprint_sha256: fingerprint,
                    });
                }
                other => {
                    self.rate_pool.release(request.estimated_tokens, 0);
                    return Err(CoreError::ModelApiError(format!(
                        "GATEWAY_FALLBACK_EXHAUSTED: Primary circuit {:?} and Baseten fallback failed ({:?})",
                        self.circuit_breaker.state, other
                    )));
                }
            }
        }

        self.rate_pool.release(request.estimated_tokens, 0);
        Err(CoreError::ModelApiError(format!(
            "GATEWAY_UNAVAILABLE: Primary provider failed for role {:?} (circuit: {:?})",
            request.role, self.circuit_breaker.state
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_valid_24khz_mono_wav(duration_ms: u64) -> Vec<u8> {
        let sample_rate: u32 = 24_000;
        let channels: u16 = 1;
        let bits_per_sample: u16 = 16;
        let byte_rate = sample_rate * u32::from(channels) * u32::from(bits_per_sample / 8);
        let data_bytes = ((u64::from(byte_rate) * duration_ms) / 1_000) as u32;
        let riff_size = 36u32.saturating_add(data_bytes);

        let mut buf = Vec::with_capacity(44);
        buf.extend_from_slice(b"RIFF");
        buf.extend_from_slice(&riff_size.to_le_bytes());
        buf.extend_from_slice(b"WAVE");
        buf.extend_from_slice(b"fmt ");
        buf.extend_from_slice(&16u32.to_le_bytes()); // PCM fmt chunk size
        buf.extend_from_slice(&1u16.to_le_bytes()); // AudioFormat = 1 (PCM)
        buf.extend_from_slice(&channels.to_le_bytes());
        buf.extend_from_slice(&sample_rate.to_le_bytes());
        buf.extend_from_slice(&byte_rate.to_le_bytes());
        buf.extend_from_slice(&2u16.to_le_bytes()); // BlockAlign
        buf.extend_from_slice(&bits_per_sample.to_le_bytes());
        buf.extend_from_slice(b"data");
        buf.extend_from_slice(&data_bytes.to_le_bytes());
        buf
    }

    #[test]
    fn test_meta_gateway_models_and_roles() {
        let mut gw = MultimodalGateway::new(GatewayConfig::default());

        // 1. muse-wonder-1.0 for DeepReasoning
        let req_wonder = GatewayRequest {
            role: ModelRole::DeepReasoning,
            requested_model: "muse-wonder-1.0".to_string(),
            prompt: "Diagnose RF-01 cart state".to_string(),
            reasoning_effort: Some("high".to_string()),
            estimated_tokens: 500,
            include_audio: false,
            audio_wav_bytes: None,
            image_sha256: None,
        };
        let resp = gw
            .execute_with_transport(&req_wonder, 1_000, |_, model, _| {
                TransportOutcome::Success {
                    serving_model: model.to_string(),
                    output_text: r#"{"status":"ok"}"#.to_string(),
                    input_tokens: 400,
                    output_tokens: 100,
                    cached_tokens: 200,
                    latency_ms: 120,
                    provenance: GatewayProvenance::DeterministicFixture,
                }
            })
            .unwrap();
        assert_eq!(resp.provider, GatewayProvider::MockOffline);
        assert_eq!(resp.provenance, GatewayProvenance::DeterministicFixture);
        assert_eq!(resp.serving_model, "muse-wonder-1.0");
        assert!(!resp.fallback_triggered);

        // 2. sam-3-1 for SpatialSegmentation (cannot be used for DeepReasoning)
        let bad_sam_role = GatewayRequest {
            role: ModelRole::DeepReasoning,
            requested_model: "sam-3-1".to_string(),
            prompt: "box".to_string(),
            reasoning_effort: None,
            estimated_tokens: 100,
            include_audio: false,
            audio_wav_bytes: None,
            image_sha256: Some("abc".to_string()),
        };
        assert!(bad_sam_role.validate().is_err());

        let good_sam_role = GatewayRequest {
            role: ModelRole::SpatialSegmentation,
            requested_model: "sam-3-1".to_string(),
            prompt: "Refresh Cart button".to_string(),
            reasoning_effort: None,
            estimated_tokens: 100,
            include_audio: false,
            audio_wav_bytes: None,
            image_sha256: Some("abc".to_string()),
        };
        assert!(good_sam_role.validate().is_ok());
    }

    #[test]
    fn test_zero_audio_rule_and_wav_validation() {
        // Text model with include_audio: true MUST fail closed with SecurityViolation
        let req_violation = GatewayRequest {
            role: ModelRole::FastSynthesis,
            requested_model: "muse-spark-1.3".to_string(),
            prompt: "Fix bug".to_string(),
            reasoning_effort: Some("xhigh".to_string()),
            estimated_tokens: 300,
            include_audio: true,
            audio_wav_bytes: None,
            image_sha256: None,
        };
        let err = req_violation.validate().unwrap_err();
        assert!(matches!(err, CoreError::SecurityViolation(_)));
        assert!(err.to_string().contains("ZERO_AUDIO_RULE_VIOLATION"));

        // Valid 24 kHz mono WAV on muse-voice-transcribe-1.0 passes
        let wav_24k = make_valid_24khz_mono_wav(5_000);
        let meta = validate_wav_audio(&wav_24k).unwrap();
        assert!(meta.is_24khz_mono);
        assert_eq!(meta.sample_rate_hz, 24_000);
        assert_eq!(meta.channels, 1);
        assert_eq!(meta.duration_ms, 5_000);

        let req_asr = GatewayRequest {
            role: ModelRole::SpeechTranscription,
            requested_model: "muse-voice-transcribe-1.0".to_string(),
            prompt: String::new(),
            reasoning_effort: None,
            estimated_tokens: 200,
            include_audio: true,
            audio_wav_bytes: Some(wav_24k),
            image_sha256: None,
        };
        assert!(req_asr.validate().is_ok());

        // Oversized duration (> 60s) fails validation
        let wav_75s = make_valid_24khz_mono_wav(75_000);
        assert!(validate_wav_audio(&wav_75s).is_err());
    }

    #[test]
    fn test_rate_pool_rpm_tpm_and_retry_after() {
        let mut pool = RatePool::new(2, 1_000, 2);
        assert!(pool.try_acquire(400).is_ok());
        assert!(pool.try_acquire(400).is_ok());
        // 3rd concurrent request exceeds max_in_flight (2) and rpm_limit (2)
        assert!(pool.try_acquire(100).is_err());

        pool.release(400, 400);
        // Still at rpm_limit == 2
        assert!(pool.try_acquire(100).is_err());

        pool.reset_window();
        // Exceeds TPM limit (1000)
        assert!(pool.try_acquire(1_200).is_err());

        // Retry-After header parsing
        assert_eq!(RatePool::compute_retry_delay_ms(0, Some("4")), 4_000);
        let backoff0 = RatePool::compute_retry_delay_ms(0, None);
        let backoff1 = RatePool::compute_retry_delay_ms(1, None);
        assert!(backoff1 > backoff0);
    }

    #[test]
    fn test_circuit_breaker_state_transitions_and_baseten_fallback() {
        let cfg = GatewayConfig {
            max_retries: 2,
            circuit_failure_threshold: 3,
            circuit_cooldown_ms: 30_000,
            ..Default::default()
        };

        let mut gw = MultimodalGateway::new(cfg);
        let req = GatewayRequest {
            role: ModelRole::FastSynthesis,
            requested_model: "muse-spark-1.3".to_string(),
            prompt: "Patch RF-01".to_string(),
            reasoning_effort: Some("xhigh".to_string()),
            estimated_tokens: 500,
            include_audio: false,
            audio_wav_bytes: None,
            image_sha256: None,
        };

        // Simulate 3 consecutive primary failures -> trips circuit to Open -> routes to BasetenFallback!
        let resp = gw
            .execute_with_transport(&req, 10_000, |provider, model, _attempt| match provider {
                GatewayProvider::MetaAi => TransportOutcome::ServerError5xx {
                    status_code: 503,
                    message: "upstream overloaded".to_string(),
                },
                GatewayProvider::BasetenFallback => TransportOutcome::Success {
                    serving_model: model.to_string(),
                    output_text: r#"{"fallback":true}"#.to_string(),
                    input_tokens: 300,
                    output_tokens: 120,
                    cached_tokens: 0,
                    latency_ms: 95,
                    provenance: GatewayProvenance::DeterministicFixture,
                },
                GatewayProvider::MockOffline => unreachable!(),
            })
            .unwrap();

        assert_eq!(gw.circuit_breaker.state, CircuitState::Open);
        assert_eq!(resp.provider, GatewayProvider::BasetenFallback);
        assert_eq!(resp.serving_model, BASETEN_MUSE_SPARK_OPEN_SLUG);
        assert!(resp.fallback_triggered);

        // Before cooldown (at t = 20_000ms), circuit remains Open
        assert!(!gw.circuit_breaker.allow_request(20_000));

        // After cooldown (at t = 45_000ms), circuit transitions to HalfOpen, then Closed on success
        assert!(gw.circuit_breaker.allow_request(45_000));
        assert_eq!(gw.circuit_breaker.state, CircuitState::HalfOpen);
        gw.circuit_breaker.record_success();
        assert_eq!(gw.circuit_breaker.state, CircuitState::Closed);
    }

    #[test]
    fn test_provenance_and_secret_redaction() {
        let secret = "meta_live_secret_token_987654321";
        let cfg = GatewayConfig::default().with_meta_api_key(secret);
        let debug_str = format!("{cfg:?}");
        assert!(!debug_str.contains(secret));
        assert!(debug_str.contains("[REDACTED]"));

        let mut gw = MultimodalGateway::new(cfg);
        let req = GatewayRequest {
            role: ModelRole::FastSynthesis,
            requested_model: "muse-spark-1.3".to_string(),
            prompt: "Fix".to_string(),
            reasoning_effort: None,
            estimated_tokens: 100,
            include_audio: false,
            audio_wav_bytes: None,
            image_sha256: None,
        };

        let err = gw
            .execute_with_transport(&req, 1_000, |_, _, _| TransportOutcome::AuthError401 {
                message: format!("Invalid bearer token {secret}"),
            })
            .unwrap_err();
        assert!(!err.to_string().contains(secret));
        assert!(err.to_string().contains("[REDACTED]"));
    }

    #[test]
    fn test_transcript_alignment_with_keyframes() {
        let segments = vec![
            TranscriptSegment {
                start_ms: 900,
                end_ms: 1_400,
                text: "Clicking Refresh Cart wipes the items".to_string(),
            },
            TranscriptSegment {
                start_ms: 3_000,
                end_ms: 3_800,
                text: "Total shows zero".to_string(),
            },
        ];
        let aligned = align_transcript_with_keyframes(
            &segments,
            &[1_550, 2_200],
            DEFAULT_ALIGNMENT_TOLERANCE_MS,
        );
        assert!(aligned[0].within_tolerance);
        assert_eq!(aligned[0].delta_ms, Some(150));
        assert!(!aligned[1].within_tolerance);
    }
}
