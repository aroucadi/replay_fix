//! Production APM Session Replay & Log Ingestion (`crates/replayfix-core/src/rrweb.rs`).
//!
//! Ingests real-world telemetry:
//! - RRWeb JSON event streams (DOM mutations, clicks, scrolls, text inputs).
//! - Sentry & APM crash dumps (unhandled exceptions, stack frames, breadcrumbs).
//! - Synthesizes validated, path-safe `TaskManifest` specs for the ReplayFix Sovereign Healer.

use crate::error::{CoreError, Result};
use crate::github::redact_github_secrets;
use crate::manifest::TaskManifest;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;

/// Strip raw ANSI escape sequences (`\x1b[...m` or control bytes) and scrub secrets
/// (`GITHUB_TOKEN`, `META_API_KEY`, `ghp_...`, `github_pat_...`, `sk-...`) from untrusted telemetry strings.
#[must_use]
pub fn strip_ansi_and_redact(raw: &str) -> String {
    let mut no_ansi = String::with_capacity(raw.len());
    let mut chars = raw.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            if chars.peek() == Some(&'[') {
                let _ = chars.next();
                for c in chars.by_ref() {
                    if ('@'..='~').contains(&c) {
                        break;
                    }
                }
            }
            continue;
        }
        if ch == '\0' {
            continue;
        }
        no_ansi.push(ch);
    }
    let env_redacted = redact_github_secrets(&no_ansi, &[]);
    let prefixes = ["ghp_", "github_pat_", "gho_", "ghs_", "sk-", "glpat-"];
    let mut out = String::with_capacity(env_redacted.len());
    let mut rest = env_redacted.as_str();
    while !rest.is_empty() {
        let mut matched_prefix = None;
        for p in prefixes {
            if rest.starts_with(p) {
                matched_prefix = Some(p);
                break;
            }
        }
        if let Some(p) = matched_prefix {
            let after_p = &rest[p.len()..];
            let token_len = after_p
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
                .map(char::len_utf8)
                .sum::<usize>();
            if token_len >= 8 {
                out.push_str("[REDACTED]");
                rest = &after_p[token_len..];
                continue;
            }
        }
        if let Some(ch) = rest.chars().next() {
            out.push(ch);
            rest = &rest[ch.len_utf8()..];
        } else {
            break;
        }
    }
    out
}

/// Alias for `strip_ansi_and_redact` scrubbing untrusted APM telemetry strings.
#[must_use]
pub fn sanitize_untrusted_telemetry_text(raw: &str) -> String {
    strip_ansi_and_redact(raw)
}

/// RRWeb Event Types according to rrweb-io specification.
pub const RRWEB_EVENT_DOM_CONTENT_LOADED: u32 = 0;
pub const RRWEB_EVENT_LOAD: u32 = 1;
pub const RRWEB_EVENT_FULL_SNAPSHOT: u32 = 2;
pub const RRWEB_EVENT_INCREMENTAL_SNAPSHOT: u32 = 3;
pub const RRWEB_EVENT_META: u32 = 4;
pub const RRWEB_EVENT_CUSTOM: u32 = 5;

/// RRWeb Incremental Source Types.
pub const RRWEB_SOURCE_MUTATION: u32 = 0;
pub const RRWEB_SOURCE_MOUSE_MOVE: u32 = 1;
pub const RRWEB_SOURCE_MOUSE_INTERACTION: u32 = 2;
pub const RRWEB_SOURCE_SCROLL: u32 = 3;
pub const RRWEB_SOURCE_VIEWPORT_RESIZE: u32 = 4;
pub const RRWEB_SOURCE_INPUT: u32 = 5;
pub const RRWEB_SOURCE_TOUCH_MOVE: u32 = 6;
pub const RRWEB_SOURCE_MEDIA_INTERACTION: u32 = 7;

/// Mouse Interaction Types.
pub const RRWEB_MOUSE_UP: u32 = 0;
pub const RRWEB_MOUSE_DOWN: u32 = 1;
pub const RRWEB_MOUSE_CLICK: u32 = 2;
pub const RRWEB_MOUSE_CONTEXT_MENU: u32 = 3;
pub const RRWEB_MOUSE_DBL_CLICK: u32 = 4;
pub const RRWEB_MOUSE_FOCUS: u32 = 5;
pub const RRWEB_MOUSE_BLUR: u32 = 6;
pub const RRWEB_TOUCH_START: u32 = 7;
pub const RRWEB_TOUCH_END: u32 = 9;

/// Single raw RRWeb event from recording stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RrwebRawEvent {
    #[serde(rename = "type")]
    pub event_type: u32,
    pub timestamp: u64,
    #[serde(default)]
    pub data: Value,
}

/// Normalized user interaction extracted from RRWeb events.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RrwebInteraction {
    pub timestamp: u64,
    pub kind: String, // "click", "dblclick", "input", "scroll", "touchstart"
    pub x: f64,
    pub y: f64,
    pub target_id: Option<i64>,
    pub text_value: Option<String>,
}

/// Extracted DOM mutation summary.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DomMutationSummary {
    pub timestamp: u64,
    pub added_nodes: usize,
    pub removed_nodes: usize,
    pub text_mutations: usize,
    pub attribute_mutations: usize,
}

/// Ingested RRWeb Session structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RrwebSession {
    pub session_id: String,
    pub viewport_width: u32,
    pub viewport_height: u32,
    pub base_url: Option<String>,
    pub interactions: Vec<RrwebInteraction>,
    pub mutations: Vec<DomMutationSummary>,
    pub start_time: u64,
    pub end_time: u64,
}

impl RrwebSession {
    /// Returns the last click or primary touch interaction preceding the crash.
    #[must_use]
    pub fn last_click_interaction(&self) -> Option<&RrwebInteraction> {
        self.interactions
            .iter()
            .rfind(|i| i.kind == "click" || i.kind == "touchstart" || i.kind == "dblclick")
    }
}

/// Parser for RRWeb event streams.
#[derive(Debug, Default)]
pub struct RrwebParser;

impl RrwebParser {
    /// Parse RRWeb events from a JSON array string or an `rrweb_ingest.schema.json` envelope object.
    pub fn parse_json(json_str: &str, session_id: &str) -> Result<RrwebSession> {
        let val: Value = serde_json::from_str(json_str).map_err(|e| {
            CoreError::SessionReplay(format!("Failed to parse RRWeb JSON events: {e}"))
        })?;

        let (events, resolved_session_id): (Vec<RrwebRawEvent>, String) = if val.is_array() {
            let parsed: Vec<RrwebRawEvent> = serde_json::from_value(val).map_err(|e| {
                CoreError::SessionReplay(format!("Failed to parse RRWeb event array: {e}"))
            })?;
            (parsed, session_id.to_string())
        } else if let Some(events_val) = val.get("events") {
            let parsed: Vec<RrwebRawEvent> =
                serde_json::from_value(events_val.clone()).map_err(|e| {
                    CoreError::SessionReplay(format!(
                        "Failed to parse 'events' field in RRWeb envelope: {e}"
                    ))
                })?;
            let env_sid = val
                .get("session_id")
                .and_then(|s| s.as_str())
                .filter(|s| !s.trim().is_empty())
                .unwrap_or(session_id)
                .to_string();
            (parsed, env_sid)
        } else {
            return Err(CoreError::SessionReplay(
                "Expected RRWeb JSON array or rrweb_ingest.schema.json envelope with 'events'"
                    .to_string(),
            ));
        };

        Self::parse_events(&events, &resolved_session_id)
    }

    /// Parse RRWeb raw events into a structured session with monotonic timestamp ordering (Law VI).
    pub fn parse_events(events: &[RrwebRawEvent], session_id: &str) -> Result<RrwebSession> {
        if events.is_empty() {
            return Err(CoreError::SessionReplay(
                "Cannot parse empty RRWeb event stream".to_string(),
            ));
        }

        let mut sorted_events = events.to_vec();
        sorted_events.sort_by_key(|e| e.timestamp);

        let mut viewport_width = 1280u32;
        let mut viewport_height = 720u32;
        let mut base_url = None;
        let mut interactions = Vec::new();
        let mut mutations = Vec::new();

        let start_time = sorted_events.first().map(|e| e.timestamp).unwrap_or(0);
        let end_time = sorted_events.last().map(|e| e.timestamp).unwrap_or(0);

        for event in &sorted_events {
            match event.event_type {
                RRWEB_EVENT_META => {
                    if let Some(href) = event.data.get("href").and_then(|h| h.as_str()) {
                        base_url = Some(href.to_string());
                    }
                    if let Some(w) = event.data.get("width").and_then(|w| w.as_u64()) {
                        viewport_width = w as u32;
                    }
                    if let Some(h) = event.data.get("height").and_then(|h| h.as_u64()) {
                        viewport_height = h as u32;
                    }
                }
                RRWEB_EVENT_FULL_SNAPSHOT => {
                    let initial_nodes = event
                        .data
                        .get("node")
                        .and_then(|n| n.get("childNodes"))
                        .and_then(|c| c.as_array())
                        .map(|a| a.len())
                        .unwrap_or(1);
                    mutations.push(DomMutationSummary {
                        timestamp: event.timestamp,
                        added_nodes: initial_nodes,
                        removed_nodes: 0,
                        text_mutations: 0,
                        attribute_mutations: 0,
                    });
                }
                RRWEB_EVENT_INCREMENTAL_SNAPSHOT => {
                    let source = event
                        .data
                        .get("source")
                        .and_then(|s| s.as_u64())
                        .unwrap_or(999) as u32;
                    match source {
                        RRWEB_SOURCE_MOUSE_INTERACTION => {
                            let mouse_type = event
                                .data
                                .get("type")
                                .and_then(|t| t.as_u64())
                                .unwrap_or(999) as u32;
                            let x = event.data.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0);
                            let y = event.data.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0);
                            let target_id = event.data.get("id").and_then(|v| v.as_i64());

                            let kind = match mouse_type {
                                RRWEB_MOUSE_CLICK => Some("click"),
                                RRWEB_MOUSE_DBL_CLICK => Some("dblclick"),
                                RRWEB_TOUCH_START => Some("touchstart"),
                                _ => None,
                            };

                            if let Some(k) = kind {
                                interactions.push(RrwebInteraction {
                                    timestamp: event.timestamp,
                                    kind: k.to_string(),
                                    x,
                                    y,
                                    target_id,
                                    text_value: None,
                                });
                            }
                        }
                        RRWEB_SOURCE_INPUT => {
                            let text = event
                                .data
                                .get("text")
                                .and_then(|t| t.as_str())
                                .map(strip_ansi_and_redact);
                            let target_id = event.data.get("id").and_then(|v| v.as_i64());
                            interactions.push(RrwebInteraction {
                                timestamp: event.timestamp,
                                kind: "input".to_string(),
                                x: 0.0,
                                y: 0.0,
                                target_id,
                                text_value: text,
                            });
                        }
                        RRWEB_SOURCE_SCROLL => {
                            let x = event.data.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0);
                            let y = event.data.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0);
                            let target_id = event.data.get("id").and_then(|v| v.as_i64());
                            interactions.push(RrwebInteraction {
                                timestamp: event.timestamp,
                                kind: "scroll".to_string(),
                                x,
                                y,
                                target_id,
                                text_value: None,
                            });
                        }
                        RRWEB_SOURCE_MUTATION => {
                            let adds = event
                                .data
                                .get("adds")
                                .and_then(|a| a.as_array())
                                .map(|v| v.len())
                                .unwrap_or(0);
                            let removes = event
                                .data
                                .get("removes")
                                .and_then(|r| r.as_array())
                                .map(|v| v.len())
                                .unwrap_or(0);
                            let texts = event
                                .data
                                .get("texts")
                                .and_then(|t| t.as_array())
                                .map(|v| v.len())
                                .unwrap_or(0);
                            let attrs = event
                                .data
                                .get("attributes")
                                .and_then(|a| a.as_array())
                                .map(|v| v.len())
                                .unwrap_or(0);

                            mutations.push(DomMutationSummary {
                                timestamp: event.timestamp,
                                added_nodes: adds,
                                removed_nodes: removes,
                                text_mutations: texts,
                                attribute_mutations: attrs,
                            });
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }

        Ok(RrwebSession {
            session_id: session_id.to_string(),
            viewport_width,
            viewport_height,
            base_url,
            interactions,
            mutations,
            start_time,
            end_time,
        })
    }
}

/// Sentry Stack Frame representation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentryStackFrame {
    pub filename: Option<String>,
    pub function: Option<String>,
    pub lineno: Option<u32>,
    pub colno: Option<u32>,
    #[serde(default)]
    pub in_app: bool,
}

/// Sentry Breadcrumb representation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentryBreadcrumb {
    pub timestamp: Option<f64>,
    pub category: Option<String>,
    pub message: Option<String>,
    pub level: Option<String>,
    #[serde(default)]
    pub data: Value,
}

/// Sentry Crash Event representation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentryCrashDump {
    pub event_id: String,
    pub exception_type: String,
    pub exception_value: String,
    pub frames: Vec<SentryStackFrame>,
    pub breadcrumbs: Vec<SentryBreadcrumb>,
    pub release: Option<String>,
    pub environment: Option<String>,
}

/// Parser for Sentry / APM crash events.
#[derive(Debug, Default)]
pub struct SentryIngestion;

impl SentryIngestion {
    /// Fetch remote crash dump from HTTPS URL using blocking reqwest client.
    pub fn fetch_remote_crash(url: &str) -> Result<String> {
        reqwest::blocking::get(url)
            .map_err(|e| {
                CoreError::SessionReplay(format!("Failed to fetch Sentry crash URL: {e}"))
            })?
            .text()
            .map_err(|e| {
                CoreError::SessionReplay(format!("Failed to read Sentry response body: {e}"))
            })
    }

    /// Parse Sentry JSON event dump.
    pub fn parse_json(json_str: &str) -> Result<SentryCrashDump> {
        let val: Value = serde_json::from_str(json_str).map_err(|e| {
            CoreError::SessionReplay(format!("Failed to parse Sentry JSON payload: {e}"))
        })?;

        let event_id = val
            .get("event_id")
            .and_then(|id| id.as_str())
            .unwrap_or("unknown_event")
            .to_string();

        let mut exception_type = "Error".to_string();
        let mut exception_value = "Unhandled Exception".to_string();
        let mut frames = Vec::new();

        if let Some(exc_values) = val
            .get("exception")
            .and_then(|e| e.get("values"))
            .and_then(|v| v.as_array())
        {
            if let Some(first_exc) = exc_values.first() {
                if let Some(t) = first_exc.get("type").and_then(|t| t.as_str()) {
                    exception_type = t.to_string();
                }
                if let Some(v) = first_exc.get("value").and_then(|v| v.as_str()) {
                    exception_value = v.to_string();
                }
                if let Some(raw_frames) = first_exc
                    .get("stacktrace")
                    .and_then(|s| s.get("frames"))
                    .and_then(|f| f.as_array())
                {
                    for rf in raw_frames {
                        let filename = rf
                            .get("filename")
                            .and_then(|f| f.as_str())
                            .map(ToString::to_string);
                        let function = rf
                            .get("function")
                            .and_then(|f| f.as_str())
                            .map(ToString::to_string);
                        let lineno = rf.get("lineno").and_then(|l| l.as_u64()).map(|l| l as u32);
                        let colno = rf.get("colno").and_then(|c| c.as_u64()).map(|c| c as u32);
                        let in_app = rf.get("in_app").and_then(|i| i.as_bool()).unwrap_or(false);

                        frames.push(SentryStackFrame {
                            filename,
                            function,
                            lineno,
                            colno,
                            in_app,
                        });
                    }
                }
            }
        }

        let mut breadcrumbs = Vec::new();
        if let Some(bc_list) = val.get("breadcrumbs").and_then(|b| {
            b.as_array()
                .or_else(|| b.get("values").and_then(|v| v.as_array()))
        }) {
            for bc in bc_list {
                breadcrumbs.push(SentryBreadcrumb {
                    timestamp: bc.get("timestamp").and_then(|t| t.as_f64()),
                    category: bc
                        .get("category")
                        .and_then(|c| c.as_str())
                        .map(ToString::to_string),
                    message: bc
                        .get("message")
                        .and_then(|m| m.as_str())
                        .map(ToString::to_string),
                    level: bc
                        .get("level")
                        .and_then(|l| l.as_str())
                        .map(ToString::to_string),
                    data: bc.get("data").cloned().unwrap_or(Value::Null),
                });
            }
        }

        let release = val
            .get("release")
            .and_then(|r| r.as_str())
            .map(ToString::to_string);
        let environment = val
            .get("environment")
            .and_then(|e| e.as_str())
            .map(ToString::to_string);

        Ok(SentryCrashDump {
            event_id,
            exception_type,
            exception_value,
            frames,
            breadcrumbs,
            release,
            environment,
        })
    }

    /// Extract candidate application source files from stack frames, validated against path traversal.
    pub fn extract_candidate_source_files(crash: &SentryCrashDump) -> Result<Vec<String>> {
        let mut candidates = Vec::new();

        for frame in &crash.frames {
            if let Some(ref raw_filename) = frame.filename {
                let normalized = Self::normalize_filename(raw_filename);
                if !normalized.is_empty() && !normalized.contains("node_modules") {
                    // Assert safe path against directory traversal
                    TaskManifest::assert_safe_path(&normalized, "candidate_source")?;
                    if !candidates.contains(&normalized) {
                        candidates.push(normalized);
                    }
                }
            }
        }

        Ok(candidates)
    }

    pub(crate) fn normalize_filename(raw: &str) -> String {
        let mut cleaned = raw.replace('\\', "/");
        if let Some(idx) = cleaned.find("webpack:///") {
            cleaned = cleaned[idx + "webpack:///".len()..].to_string();
        } else if let Some(idx) = cleaned.find("app:///") {
            cleaned = cleaned[idx + "app:///".len()..].to_string();
        }
        if let Some(idx) = cleaned.find("/src/") {
            cleaned = cleaned[idx + 1..].to_string();
        } else if let Some(stripped) = cleaned.strip_prefix("./") {
            cleaned = stripped.to_string();
        }
        cleaned
    }
}

/// OpenTelemetry (OTel) Span representation for distributed trace alignment.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OtelSpan {
    pub trace_id: String,
    pub span_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_span_id: Option<String>,
    pub name: String,
    pub start_time_ms: u64,
    pub end_time_ms: u64,
    pub status_code: String,
    #[serde(default)]
    pub attributes: Value,
}

/// Time-synchronized correlation between an RRWeb user interaction and an OpenTelemetry span.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AlignedInteractionSpan {
    pub interaction_timestamp: u64,
    pub interaction_kind: String,
    pub span_id: String,
    pub trace_id: String,
    pub span_name: String,
    pub status_code: String,
    pub delta_ms: u64,
}

/// Parser and temporal correlator for OpenTelemetry (OTel) span exports.
#[derive(Debug, Default)]
pub struct OtelIngestion;

impl OtelIngestion {
    /// Parse OpenTelemetry JSON spans (either an array of spans or `{ "spans": [...] }`).
    pub fn parse_json(json_str: &str) -> Result<Vec<OtelSpan>> {
        let val: Value = serde_json::from_str(json_str).map_err(|e| {
            CoreError::Telemetry(format!("Failed to parse OpenTelemetry JSON payload: {e}"))
        })?;
        let arr = if let Some(a) = val.as_array() {
            a
        } else if let Some(a) = val.get("spans").and_then(Value::as_array) {
            a
        } else {
            return Err(CoreError::Telemetry(
                "Expected OpenTelemetry JSON array or object with 'spans' array".to_string(),
            ));
        };

        let mut spans = Vec::with_capacity(arr.len());
        for item in arr {
            let trace_id = item
                .get("trace_id")
                .or_else(|| item.get("traceId"))
                .and_then(Value::as_str)
                .unwrap_or("00000000000000000000000000000000")
                .to_string();
            let span_id = item
                .get("span_id")
                .or_else(|| item.get("spanId"))
                .and_then(Value::as_str)
                .unwrap_or("0000000000000000")
                .to_string();
            let parent_span_id = item
                .get("parent_span_id")
                .or_else(|| item.get("parentSpanId"))
                .and_then(Value::as_str)
                .map(ToString::to_string);
            let name =
                strip_ansi_and_redact(item.get("name").and_then(Value::as_str).unwrap_or("span"));
            let start_time_ms = item
                .get("start_time_ms")
                .and_then(Value::as_u64)
                .or_else(|| {
                    item.get("startTimeUnixNano")
                        .and_then(Value::as_u64)
                        .map(|n| n / 1_000_000)
                })
                .unwrap_or(0);
            let end_time_ms = item
                .get("end_time_ms")
                .and_then(Value::as_u64)
                .or_else(|| {
                    item.get("endTimeUnixNano")
                        .and_then(Value::as_u64)
                        .map(|n| n / 1_000_000)
                })
                .unwrap_or(start_time_ms);
            let status_code = item
                .get("status_code")
                .and_then(Value::as_str)
                .or_else(|| {
                    item.get("status")
                        .and_then(|s| s.get("code"))
                        .and_then(Value::as_str)
                })
                .unwrap_or("OK")
                .to_string();
            let attributes = item.get("attributes").cloned().unwrap_or(Value::Null);

            spans.push(OtelSpan {
                trace_id,
                span_id,
                parent_span_id,
                name,
                start_time_ms,
                end_time_ms,
                status_code,
                attributes,
            });
        }
        Ok(spans)
    }

    /// Align RRWeb interactions with OpenTelemetry spans within `tolerance_ms`.
    #[must_use]
    pub fn align_with_rrweb(
        session: &RrwebSession,
        spans: &[OtelSpan],
        tolerance_ms: u64,
    ) -> Vec<AlignedInteractionSpan> {
        let mut aligned = Vec::new();
        for interaction in &session.interactions {
            for span in spans {
                let delta = interaction.timestamp.abs_diff(span.start_time_ms);
                if delta <= tolerance_ms {
                    aligned.push(AlignedInteractionSpan {
                        interaction_timestamp: interaction.timestamp,
                        interaction_kind: interaction.kind.clone(),
                        span_id: span.span_id.clone(),
                        trace_id: span.trace_id.clone(),
                        span_name: span.name.clone(),
                        status_code: span.status_code.clone(),
                        delta_ms: delta,
                    });
                }
            }
        }
        aligned
    }
}

/// Source Map v3 resolver mapping minified bundle frames back to validated original source files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceMapResolver {
    pub version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(
        default,
        rename = "sourceRoot",
        skip_serializing_if = "Option::is_none"
    )]
    pub source_root: Option<String>,
    pub sources: Vec<String>,
    #[serde(default)]
    pub names: Vec<String>,
}

impl SourceMapResolver {
    /// Parse a Source Map v3 JSON payload and validate version 3 compliance.
    pub fn parse_json(json_str: &str) -> Result<Self> {
        let map: Self = serde_json::from_str(json_str).map_err(|e| {
            CoreError::SessionReplay(format!("Failed to parse SourceMap v3 JSON: {e}"))
        })?;
        if map.version != 3 {
            return Err(CoreError::SessionReplay(format!(
                "Unsupported SourceMap version {}: expected 3",
                map.version
            )));
        }
        Ok(map)
    }

    /// Resolve candidate original TypeScript/JavaScript source paths from the sourcemap,
    /// enforcing `TaskManifest::assert_safe_path`.
    pub fn resolve_candidate_sources(&self) -> Result<Vec<String>> {
        let mut resolved = Vec::new();
        for raw_src in &self.sources {
            let normalized = SentryIngestion::normalize_filename(raw_src);
            if !normalized.is_empty() && !normalized.contains("node_modules") {
                TaskManifest::assert_safe_path(&normalized, "sourcemap_source")?;
                if !resolved.contains(&normalized) {
                    resolved.push(normalized);
                }
            }
        }
        Ok(resolved)
    }
}

/// Ingest a raw RRWeb JSON payload (array or `rrweb_ingest.schema.json` envelope) into an `RrwebSession`.
pub fn ingest_session_replay(json_str: &str, session_id: &str) -> Result<RrwebSession> {
    RrwebParser::parse_json(json_str, session_id)
}

/// Translate Sentry crash breadcrumbs and stack frames into validated candidate source files and action summaries.
pub fn translate_sentry_breadcrumbs(crash: &SentryCrashDump) -> Result<(Vec<String>, Vec<String>)> {
    let candidate_files = SentryIngestion::extract_candidate_source_files(crash)?;
    let actions = crash
        .breadcrumbs
        .iter()
        .map(|b| {
            let cat = strip_ansi_and_redact(b.category.as_deref().unwrap_or("event"));
            let msg = strip_ansi_and_redact(b.message.as_deref().unwrap_or("unknown"));
            format!("{cat}: {msg}")
        })
        .collect();
    Ok((candidate_files, actions))
}

/// Synthesize a validated `TaskManifest` from RRWeb session, Sentry crash dump, OpenTelemetry spans, and SourceMap.
pub fn synthesize_manifest_from_apm(
    session: &RrwebSession,
    crash: Option<&SentryCrashDump>,
    otel_spans: &[OtelSpan],
    sourcemap: Option<&SourceMapResolver>,
    repo_path: &Path,
) -> Result<TaskManifest> {
    let mut candidate_files = if let Some(c) = crash {
        SentryIngestion::extract_candidate_source_files(c)?
    } else {
        Vec::new()
    };

    if let Some(sm) = sourcemap {
        for resolved in sm.resolve_candidate_sources()? {
            if !candidate_files.contains(&resolved) {
                candidate_files.push(resolved);
            }
        }
    }

    let aligned_spans = OtelIngestion::align_with_rrweb(session, otel_spans, 2_000);

    let task_id = crash
        .map(|c| format!("APM-{}", &c.event_id[..c.event_id.len().min(8)]))
        .unwrap_or_else(|| {
            format!(
                "RRWEB-{}",
                &session.session_id[..session.session_id.len().min(8)]
            )
        });

    let title = crash
        .map(|c| strip_ansi_and_redact(&format!("{}: {}", c.exception_type, c.exception_value)))
        .unwrap_or_else(|| "Production Session Replay Bug".to_string());

    let (interaction_x, interaction_y) = session
        .last_click_interaction()
        .map(|i| (i.x, i.y))
        .unwrap_or((100.0, 100.0));

    let mut expected_behavior = if candidate_files.is_empty() {
        format!("User interaction at ({interaction_x}, {interaction_y}) should succeed")
    } else {
        format!(
            "User interaction at ({interaction_x}, {interaction_y}) should succeed (candidates: {})",
            candidate_files.join(", ")
        )
    };

    if !aligned_spans.is_empty() {
        let span_names: Vec<String> = aligned_spans
            .iter()
            .map(|s| format!("{}[{}]", s.span_name, s.status_code))
            .collect();
        expected_behavior.push_str(&format!(" [otel: {}]", span_names.join(", ")));
    }

    let manifest_json = serde_json::json!({
        "task_id": task_id,
        "title": title,
        "repo_path": repo_path.to_string_lossy().to_string(),
        "bug_ref": "HEAD",
        "clean_ref": "main",
        "video_path": "session_replay.mp4",
        "expected_behavior": expected_behavior,
    });

    let manifest_str = serde_json::to_string(&manifest_json)?;
    TaskManifest::parse_str(&manifest_str)
}

/// Synthesize a validated `TaskManifest` from RRWeb session and Sentry crash dump.
pub fn synthesize_manifest_from_telemetry(
    session: &RrwebSession,
    crash: Option<&SentryCrashDump>,
    repo_path: &Path,
) -> Result<TaskManifest> {
    synthesize_manifest_from_apm(session, crash, &[], None, repo_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rrweb_parse_envelope_and_out_of_order_events() {
        let envelope = r#"{
            "source_type": "rrweb",
            "session_id": "env_sess_99",
            "timestamp": 1000,
            "events": [
                {"type": 3, "timestamp": 2000, "data": {"source": 2, "type": 2, "x": 120.0, "y": 240.0}},
                {"type": 4, "timestamp": 1000, "data": {"href": "http://localhost:3000", "width": 1440, "height": 900}},
                {"type": 2, "timestamp": 1500, "data": {"node": {"childNodes": [{}, {}]}}}
            ]
        }"#;
        let session = ingest_session_replay(envelope, "fallback_id").unwrap();
        assert_eq!(session.session_id, "env_sess_99");
        assert_eq!(session.viewport_width, 1440);
        assert_eq!(session.viewport_height, 900);
        assert_eq!(session.start_time, 1000);
        assert_eq!(session.end_time, 2000);
        assert_eq!(session.mutations.len(), 1);
        assert_eq!(session.mutations[0].added_nodes, 2);
    }

    #[test]
    fn test_rrweb_parse_events_extracts_viewport_and_click() {
        let events = vec![
            RrwebRawEvent {
                event_type: RRWEB_EVENT_META,
                timestamp: 1000,
                data: serde_json::json!({
                    "href": "http://localhost:3000/cart",
                    "width": 1920,
                    "height": 1080
                }),
            },
            RrwebRawEvent {
                event_type: RRWEB_EVENT_INCREMENTAL_SNAPSHOT,
                timestamp: 1050,
                data: serde_json::json!({
                    "source": RRWEB_SOURCE_MOUSE_INTERACTION,
                    "type": RRWEB_MOUSE_CLICK,
                    "id": 42,
                    "x": 345.5,
                    "y": 678.0
                }),
            },
            RrwebRawEvent {
                event_type: RRWEB_EVENT_INCREMENTAL_SNAPSHOT,
                timestamp: 1100,
                data: serde_json::json!({
                    "source": RRWEB_SOURCE_MUTATION,
                    "adds": [{"id": 43}],
                    "removes": [],
                    "texts": [],
                    "attributes": []
                }),
            },
        ];

        let session = RrwebParser::parse_events(&events, "sess_123").unwrap();
        assert_eq!(session.session_id, "sess_123");
        assert_eq!(session.viewport_width, 1920);
        assert_eq!(session.viewport_height, 1080);
        assert_eq!(
            session.base_url.as_deref(),
            Some("http://localhost:3000/cart")
        );
        assert_eq!(session.interactions.len(), 1);

        let click = session.last_click_interaction().unwrap();
        assert_eq!(click.kind, "click");
        assert_eq!(click.x, 345.5);
        assert_eq!(click.y, 678.0);
        assert_eq!(session.mutations.len(), 1);
        assert_eq!(session.mutations[0].added_nodes, 1);
    }

    #[test]
    fn test_sentry_ingest_extracts_exception_and_candidate_files() {
        let json = r#"{
            "event_id": "c0ffee1234567890",
            "release": "v1.2.3",
            "exception": {
                "values": [{
                    "type": "TypeError",
                    "value": "Cannot read property 'price' of undefined",
                    "stacktrace": {
                        "frames": [
                            {"filename": "node_modules/react-dom/index.js", "in_app": false},
                            {"filename": "webpack:///src/components/CartSummary.tsx", "in_app": true, "lineno": 45},
                            {"filename": "webpack:///src/state/cartReducer.ts", "in_app": true, "lineno": 112}
                        ]
                    }
                }]
            },
            "breadcrumbs": [
                {"timestamp": 12345.0, "category": "ui.click", "message": "button#checkout"}
            ]
        }"#;

        let crash = SentryIngestion::parse_json(json).unwrap();
        assert_eq!(crash.event_id, "c0ffee1234567890");
        assert_eq!(crash.exception_type, "TypeError");
        assert_eq!(crash.breadcrumbs.len(), 1);

        let files = SentryIngestion::extract_candidate_source_files(&crash).unwrap();
        assert_eq!(
            files,
            vec!["src/components/CartSummary.tsx", "src/state/cartReducer.ts"]
        );
    }

    #[test]
    fn test_sentry_rejects_path_traversal() {
        let json = r#"{
            "event_id": "bad123",
            "exception": {
                "values": [{
                    "type": "Error",
                    "value": "exploited",
                    "stacktrace": {
                        "frames": [
                            {"filename": "../../../etc/passwd", "in_app": true}
                        ]
                    }
                }]
            }
        }"#;

        let crash = SentryIngestion::parse_json(json).unwrap();
        let res = SentryIngestion::extract_candidate_source_files(&crash);
        assert!(res.is_err());
    }

    #[test]
    fn test_synthesize_manifest_from_telemetry() {
        let events = vec![
            RrwebRawEvent {
                event_type: RRWEB_EVENT_META,
                timestamp: 1000,
                data: serde_json::json!({
                    "href": "http://localhost:3000",
                    "width": 1280,
                    "height": 720
                }),
            },
            RrwebRawEvent {
                event_type: RRWEB_EVENT_INCREMENTAL_SNAPSHOT,
                timestamp: 1050,
                data: serde_json::json!({
                    "source": RRWEB_SOURCE_MOUSE_INTERACTION,
                    "type": RRWEB_MOUSE_CLICK,
                    "x": 200.0,
                    "y": 400.0
                }),
            },
        ];
        let session = RrwebParser::parse_events(&events, "sess_abc").unwrap();

        let sentry_json = r#"{
            "event_id": "abcd1234ef",
            "exception": {
                "values": [{
                    "type": "ReferenceError",
                    "value": "item is not defined",
                    "stacktrace": {
                        "frames": [
                            {"filename": "src/App.tsx", "in_app": true}
                        ]
                    }
                }]
            }
        }"#;
        let crash = SentryIngestion::parse_json(sentry_json).unwrap();

        let manifest =
            synthesize_manifest_from_telemetry(&session, Some(&crash), Path::new(".")).unwrap();
        assert_eq!(manifest.task_id, "APM-abcd1234");
        assert_eq!(manifest.title, "ReferenceError: item is not defined");
        assert_eq!(manifest.video_path, "session_replay.mp4");
    }
}
