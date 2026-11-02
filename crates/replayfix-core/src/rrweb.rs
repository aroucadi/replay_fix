//! Production APM Session Replay & Log Ingestion (`crates/replayfix-core/src/rrweb.rs`).
//!
//! Ingests real-world telemetry:
//! - RRWeb JSON event streams (DOM mutations, clicks, scrolls, text inputs).
//! - Sentry & APM crash dumps (unhandled exceptions, stack frames, breadcrumbs).
//! - Synthesizes validated, path-safe `TaskManifest` specs for the ReplayFix Sovereign Healer.

use crate::error::{CoreError, Result};
use crate::manifest::TaskManifest;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;

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
    /// Parse RRWeb events from a JSON array string.
    pub fn parse_json(json_str: &str, session_id: &str) -> Result<RrwebSession> {
        let events: Vec<RrwebRawEvent> = serde_json::from_str(json_str).map_err(|e| {
            CoreError::SessionReplay(format!("Failed to parse RRWeb JSON events: {e}"))
        })?;
        Self::parse_events(&events, session_id)
    }

    /// Parse RRWeb raw events into a structured session.
    pub fn parse_events(events: &[RrwebRawEvent], session_id: &str) -> Result<RrwebSession> {
        if events.is_empty() {
            return Err(CoreError::SessionReplay(
                "Cannot parse empty RRWeb event stream".to_string(),
            ));
        }

        let mut viewport_width = 1280u32;
        let mut viewport_height = 720u32;
        let mut base_url = None;
        let mut interactions = Vec::new();
        let mut mutations = Vec::new();

        let start_time = events.first().map(|e| e.timestamp).unwrap_or(0);
        let end_time = events.last().map(|e| e.timestamp).unwrap_or(0);

        for event in events {
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
                                .map(|s| s.to_string());
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

    fn normalize_filename(raw: &str) -> String {
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

/// Synthesize a validated `TaskManifest` from RRWeb session and Sentry crash dump.
pub fn synthesize_manifest_from_telemetry(
    session: &RrwebSession,
    crash: Option<&SentryCrashDump>,
    repo_path: &Path,
) -> Result<TaskManifest> {
    let task_id = crash
        .map(|c| format!("APM-{}", &c.event_id[..c.event_id.len().min(8)]))
        .unwrap_or_else(|| {
            format!(
                "RRWEB-{}",
                &session.session_id[..session.session_id.len().min(8)]
            )
        });

    let title = crash
        .map(|c| format!("{}: {}", c.exception_type, c.exception_value))
        .unwrap_or_else(|| "Production Session Replay Bug".to_string());

    let (interaction_x, interaction_y) = session
        .last_click_interaction()
        .map(|i| (i.x, i.y))
        .unwrap_or((100.0, 100.0));

    let manifest_json = serde_json::json!({
        "task_id": task_id,
        "title": title,
        "repo_path": repo_path.to_string_lossy().to_string(),
        "bug_ref": "HEAD",
        "clean_ref": "main",
        "video_path": "session_replay.mp4",
        "expected_behavior": format!("User interaction at ({interaction_x}, {interaction_y}) should succeed"),
    });

    let manifest_str = serde_json::to_string(&manifest_json)?;
    TaskManifest::parse_str(&manifest_str)
}

#[cfg(test)]
mod tests {
    use super::*;

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
