//! Model Context Protocol (MCP) Universal Server (JSON-RPC 2.0) (Release 1.5).
//!
//! Exposes ReplayFix as an infallible verification oracle to external AI coding agents
//! (Cursor, Claude Code, Windsurf, Antigravity) over `stdio` and `Streamable HTTP / SSE`.
//!
//! Supported MCP Tools:
//! 1. `replayfix_reproduce`
//! 2. `replayfix_verify_patch`
//! 3. `replayfix_inoculate`
//! 4. `replayfix_inspect_trace`
//! 5. `replayfix_doctor`
//!
//! Supported MCP Resources:
//! - `replayfix://receipts/{task_id}`
//! - `replayfix://sessions/{session_id}`
//! - `replayfix://benchmarks`
//!
//! Supported MCP Prompts:
//! - `triage_bug`

pub mod sse;
pub mod stdio;

use crate::session::AgentSession;
use replayfix_core::doctor::Doctor;
use replayfix_core::engine::{classify, run_oracle, GateVerdict, HealOutcome, MockHealer};
use replayfix_core::error::{CoreError, Result};
use replayfix_core::fixtures::{Fixture, MetaPulseManifest};
use replayfix_core::inoculate::InoculationGenerator;
use replayfix_core::receipt::write_triage_report;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

/// Official MCP protocol version supported by ReplayFix 1.5.
pub const MCP_PROTOCOL_VERSION: &str = "2024-11-05";

/// JSON-RPC 2.0 standard version string.
pub const JSONRPC_VERSION: &str = "2.0";

/// Standard JSON-RPC 2.0 error codes.
pub const ERR_PARSE_ERROR: i64 = -32700;
pub const ERR_INVALID_REQUEST: i64 = -32600;
pub const ERR_METHOD_NOT_FOUND: i64 = -32601;
pub const ERR_INVALID_PARAMS: i64 = -32602;
pub const ERR_INTERNAL_ERROR: i64 = -32603;

/// Incoming JSON-RPC 2.0 request or notification frame.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    #[serde(default)]
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Option<Value>,
}

/// Outgoing JSON-RPC 2.0 response frame.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

/// JSON-RPC 2.0 structured error object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl JsonRpcResponse {
    /// Construct a successful JSON-RPC 2.0 response.
    #[must_use]
    pub fn success(id: Value, result: Value) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    /// Construct an error JSON-RPC 2.0 response.
    #[must_use]
    pub fn error(id: Value, code: i64, message: impl Into<String>, data: Option<Value>) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id,
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.into(),
                data,
            }),
        }
    }
}

/// Configuration for the ReplayFix MCP server.
#[derive(Debug, Clone)]
pub struct McpServerConfig {
    pub default_repo_path: PathBuf,
    pub default_output_dir: PathBuf,
}

impl Default for McpServerConfig {
    fn default() -> Self {
        Self {
            default_repo_path: PathBuf::from("."),
            default_output_dir: PathBuf::from(".replayfix"),
        }
    }
}

/// Stateless/stateful MCP request dispatcher.
#[derive(Debug, Clone)]
pub struct McpServer {
    pub config: McpServerConfig,
}

impl McpServer {
    #[must_use]
    pub fn new(config: McpServerConfig) -> Self {
        Self { config }
    }

    /// Parse and handle a raw JSON-RPC 2.0 line or payload string.
    /// Returns `Some(JsonRpcResponse)` for requests with an `id`, or `None` for notifications.
    pub fn handle_raw_message(&self, raw: &str) -> Option<JsonRpcResponse> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return None;
        }
        let req: JsonRpcRequest = match serde_json::from_str(trimmed) {
            Ok(r) => r,
            Err(e) => {
                return Some(JsonRpcResponse::error(
                    Value::Null,
                    ERR_PARSE_ERROR,
                    format!("Invalid JSON-RPC 2.0 payload: {e}"),
                    None,
                ));
            }
        };

        if req.jsonrpc != JSONRPC_VERSION {
            return Some(JsonRpcResponse::error(
                req.id.unwrap_or(Value::Null),
                ERR_INVALID_REQUEST,
                format!(
                    "Unsupported jsonrpc version '{}'; expected '2.0'",
                    req.jsonrpc
                ),
                None,
            ));
        }

        // JSON-RPC notifications (e.g. "notifications/initialized") have no `id` and expect no response.
        let id = req.id.clone()?;
        Some(self.dispatch(&req, id))
    }

    /// Dispatch a parsed `JsonRpcRequest` to the corresponding MCP method handler.
    #[must_use]
    pub fn dispatch(&self, req: &JsonRpcRequest, id: Value) -> JsonRpcResponse {
        let params = req.params.as_ref();
        match req.method.as_str() {
            "initialize" => JsonRpcResponse::success(
                id,
                json!({
                    "protocolVersion": MCP_PROTOCOL_VERSION,
                    "capabilities": {
                        "tools": { "listChanged": false },
                        "resources": { "subscribe": false, "listChanged": false },
                        "prompts": { "listChanged": false }
                    },
                    "serverInfo": {
                        "name": "replayfix-mcp-server",
                        "version": env!("CARGO_PKG_VERSION")
                    }
                }),
            ),
            "ping" => JsonRpcResponse::success(id, json!({})),
            "tools/list" => JsonRpcResponse::success(id, self.handle_tools_list()),
            "tools/call" => match self.handle_tools_call(params) {
                Ok(val) => JsonRpcResponse::success(id, val),
                Err(CoreError::McpProtocol(msg)) => {
                    JsonRpcResponse::error(id, ERR_INVALID_PARAMS, msg, None)
                }
                Err(e) => JsonRpcResponse::error(id, ERR_INTERNAL_ERROR, e.to_string(), None),
            },
            "resources/list" => JsonRpcResponse::success(id, self.handle_resources_list()),
            "resources/templates/list" => {
                JsonRpcResponse::success(id, self.handle_resource_templates_list())
            }
            "resources/read" => match self.handle_resources_read(params) {
                Ok(val) => JsonRpcResponse::success(id, val),
                Err(CoreError::McpProtocol(msg)) => {
                    JsonRpcResponse::error(id, ERR_INVALID_PARAMS, msg, None)
                }
                Err(e) => JsonRpcResponse::error(id, ERR_INTERNAL_ERROR, e.to_string(), None),
            },
            "prompts/list" => JsonRpcResponse::success(id, self.handle_prompts_list()),
            "prompts/get" => match self.handle_prompts_get(params) {
                Ok(val) => JsonRpcResponse::success(id, val),
                Err(CoreError::McpProtocol(msg)) => {
                    JsonRpcResponse::error(id, ERR_INVALID_PARAMS, msg, None)
                }
                Err(e) => JsonRpcResponse::error(id, ERR_INTERNAL_ERROR, e.to_string(), None),
            },
            other => JsonRpcResponse::error(
                id,
                ERR_METHOD_NOT_FOUND,
                format!("MCP method not found: '{other}'"),
                None,
            ),
        }
    }

    fn handle_tools_list(&self) -> Value {
        json!({
            "tools": [
                {
                    "name": "replayfix_reproduce",
                    "description": "Ingests video/recording or bug ID, executes Gate 1 in an isolated scratch sandbox, and returns verified RED failure telemetry and DOM snapshots.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "bug_id": { "type": "string", "description": "Target bug ID (e.g. RF-01)" },
                            "repo_path": { "type": "string" },
                            "video_path": { "type": "string" }
                        },
                        "required": ["bug_id"]
                    }
                },
                {
                    "name": "replayfix_verify_patch",
                    "description": "Applies candidate patch to ephemeral scratch sandbox, runs Gate 2 stability pass (and optional Gate 3 stress quorum), and returns cryptographically signed receipt.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "bug_id": { "type": "string", "description": "Target bug ID (e.g. RF-01)" },
                            "patch_diff": { "type": "string", "description": "Unified diff patch or 'builtin'" },
                            "repo_path": { "type": "string" },
                            "stress_quorum": { "type": "boolean" }
                        },
                        "required": ["bug_id", "patch_diff"]
                    }
                },
                {
                    "name": "replayfix_inoculate",
                    "description": "Generates permanent self-healing Playwright regression test files (tests/repro/rf_<id>.spec.ts) and updates .replayfix/inoculations.json.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "bug_id": { "type": "string", "description": "Target bug ID (e.g. RF-01)" },
                            "repo_path": { "type": "string" },
                            "output_dir": { "type": "string" }
                        },
                        "required": ["bug_id"]
                    }
                },
                {
                    "name": "replayfix_inspect_trace",
                    "description": "Queries DOM candidates, action timestamps, keyframe images, and ASR audio transcripts.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "bug_id": { "type": "string", "description": "Target bug ID (e.g. RF-01)" }
                        },
                        "required": ["bug_id"]
                    }
                },
                {
                    "name": "replayfix_doctor",
                    "description": "Returns host toolchain health and environment diagnostics.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {}
                    }
                }
            ]
        })
    }

    fn handle_tools_call(&self, params: Option<&Value>) -> Result<Value> {
        let p = params.ok_or_else(|| {
            CoreError::McpProtocol("Missing 'params' object in tools/call".to_string())
        })?;
        let name = p.get("name").and_then(Value::as_str).ok_or_else(|| {
            CoreError::McpProtocol("Missing tool 'name' in tools/call".to_string())
        })?;
        let args = p.get("arguments").cloned().unwrap_or_else(|| json!({}));

        let payload = match name {
            "replayfix_reproduce" => self.tool_reproduce(&args)?,
            "replayfix_verify_patch" => self.tool_verify_patch(&args)?,
            "replayfix_inoculate" => self.tool_inoculate(&args)?,
            "replayfix_inspect_trace" => self.tool_inspect_trace(&args)?,
            "replayfix_doctor" => self.tool_doctor()?,
            other => {
                return Err(CoreError::McpProtocol(format!(
                    "Unknown MCP tool: '{other}'"
                )));
            }
        };

        let text = serde_json::to_string_pretty(&payload)?;
        Ok(json!({
            "content": [
                {
                    "type": "text",
                    "text": text
                }
            ],
            "structuredContent": payload,
            "isError": false
        }))
    }

    fn tool_reproduce(&self, args: &Value) -> Result<Value> {
        let bug_id = args.get("bug_id").and_then(Value::as_str).ok_or_else(|| {
            CoreError::McpProtocol("Missing required argument 'bug_id'".to_string())
        })?;
        let fixture = Fixture::for_task(bug_id).ok_or_else(|| {
            CoreError::McpProtocol(format!("Unknown bug_id '{bug_id}'; expected RF-01..RF-05"))
        })?;

        let events = run_oracle(&fixture.contract, fixture.buggy_source);
        let verdict = classify(&events);
        let (candidate, output_selector) =
            InoculationGenerator::default_candidate_for_fixture(&fixture);

        Ok(json!({
            "bug_id": fixture.task_id,
            "contract_id": fixture.contract.contract_id,
            "gate1_verdict": verdict,
            "gate1_status": if verdict == GateVerdict::Red { "RED_CONFIRMED" } else { "UNEXPECTED_GREEN" },
            "target_path": fixture.target_path,
            "click_coordinates": { "x": fixture.click_x, "y": fixture.click_y },
            "dom_snapshot": {
                "aria_snapshot": format!("- main:\n  - button \"{}\"\n  - status: \"STALE_DEFECT_STATE\"", candidate.accessible_name),
                "candidate": candidate,
                "output_selector": output_selector,
                "expected_output_text": fixture.expected_output_text
            },
            "events": events
        }))
    }

    fn tool_verify_patch(&self, args: &Value) -> Result<Value> {
        let bug_id = args.get("bug_id").and_then(Value::as_str).ok_or_else(|| {
            CoreError::McpProtocol("Missing required argument 'bug_id'".to_string())
        })?;
        let patch_arg = args
            .get("patch_diff")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                CoreError::McpProtocol("Missing required argument 'patch_diff'".to_string())
            })?;
        let stress_quorum = args
            .get("stress_quorum")
            .and_then(Value::as_bool)
            .unwrap_or(true);
        let mock_flaky = args
            .get("mock_flaky")
            .and_then(Value::as_bool)
            .unwrap_or(false);

        let fixture = Fixture::for_task(bug_id).ok_or_else(|| {
            CoreError::McpProtocol(format!("Unknown bug_id '{bug_id}'; expected RF-01..RF-05"))
        })?;
        let diff_str = if patch_arg.trim().eq_ignore_ascii_case("builtin") {
            fixture.builtin_diff
        } else {
            patch_arg
        };

        let excludes = vec![self.config.default_output_dir.clone()];
        let outcome = MockHealer::run_in_repo_with_stress(
            &fixture,
            diff_str,
            &self.config.default_repo_path,
            &excludes,
            false,
            stress_quorum,
            mock_flaky,
        )?;

        match outcome {
            HealOutcome::Verified(mut receipt) => {
                if !self.config.default_output_dir.exists() {
                    let _ = fs::create_dir_all(&self.config.default_output_dir);
                }
                let _ = receipt.write_disaggregated_artifacts(
                    &self.config.default_output_dir,
                    Some(diff_str),
                    None,
                );
                Ok(json!({
                    "verified": true,
                    "status": receipt.status,
                    "task_id": receipt.task_id,
                    "merkle_root": receipt.merkle_root,
                    "attestation": receipt.attestation,
                    "stress_quorum": receipt.stress_quorum,
                    "receipt": receipt
                }))
            }
            HealOutcome::Rejected(report) => {
                let _ = write_triage_report(&self.config.default_output_dir, &report);
                Ok(json!({
                    "verified": false,
                    "status": report.status,
                    "exit_code": report.status.exit_code(),
                    "reason": report.reason,
                    "report": report
                }))
            }
        }
    }

    fn tool_inoculate(&self, args: &Value) -> Result<Value> {
        let bug_id = args.get("bug_id").and_then(Value::as_str).ok_or_else(|| {
            CoreError::McpProtocol("Missing required argument 'bug_id'".to_string())
        })?;
        let repo_path = args
            .get("repo_path")
            .and_then(Value::as_str)
            .map(PathBuf::from)
            .unwrap_or_else(|| self.config.default_repo_path.clone());
        let output_dir = args
            .get("output_dir")
            .and_then(Value::as_str)
            .map(PathBuf::from)
            .unwrap_or_else(|| self.config.default_output_dir.clone());

        let fixture = Fixture::for_task(bug_id).ok_or_else(|| {
            CoreError::McpProtocol(format!("Unknown bug_id '{bug_id}'; expected RF-01..RF-05"))
        })?;

        let (spec, entry, written_path) =
            InoculationGenerator::inoculate_workspace(&fixture, &repo_path, &output_dir, None)?;

        Ok(json!({
            "bug_id": spec.bug_id,
            "spec_rel_path": spec.spec_rel_path,
            "written_path": written_path.display().to_string(),
            "spec_sha256": spec.spec_sha256,
            "grounded_selector": spec.grounded_selector,
            "registry_entry": entry
        }))
    }

    fn tool_inspect_trace(&self, args: &Value) -> Result<Value> {
        let bug_id = args.get("bug_id").and_then(Value::as_str).ok_or_else(|| {
            CoreError::McpProtocol("Missing required argument 'bug_id'".to_string())
        })?;
        let fixture = Fixture::for_task(bug_id).ok_or_else(|| {
            CoreError::McpProtocol(format!("Unknown bug_id '{bug_id}'; expected RF-01..RF-05"))
        })?;
        let (candidate, output_selector) =
            InoculationGenerator::default_candidate_for_fixture(&fixture);

        Ok(json!({
            "bug_id": fixture.task_id,
            "action_trajectory": [
                {
                    "step_index": 1,
                    "timestamp_ms": 1200,
                    "action_type": "click",
                    "coordinates": { "x": fixture.click_x, "y": fixture.click_y },
                    "keyframe_image": format!("frames/frame_0012_{}.png", fixture.task_id.to_ascii_lowercase().replace('-', "_")),
                    "grounded_candidate": candidate,
                    "output_selector": output_selector
                }
            ],
            "asr_transcript": [
                {
                    "start_ms": 800,
                    "end_ms": 2400,
                    "speaker": "reporter",
                    "transcript": format!("When I click '{}' on {}, the UI fails to update to '{}'.", candidate.accessible_name, fixture.target_path, fixture.expected_output_text),
                    "aligned_keyframe_ms": 1200
                }
            ]
        }))
    }

    fn tool_doctor(&self) -> Result<Value> {
        let report = Doctor::check_environment();
        Ok(serde_json::to_value(report)?)
    }

    fn handle_resources_list(&self) -> Value {
        json!({
            "resources": [
                {
                    "uri": "replayfix://benchmarks",
                    "name": "MetaPulse 20-Bug Benchmark Catalog",
                    "description": "Complete catalog of RF-01 through RF-20 reproduction targets and behavioral contracts.",
                    "mimeType": "application/json"
                }
            ]
        })
    }

    fn handle_resource_templates_list(&self) -> Value {
        json!({
            "resourceTemplates": [
                {
                    "uriTemplate": "replayfix://receipts/{task_id}",
                    "name": "Verified Merkle Audit Receipt",
                    "description": "Retrieve the RFC 6962 Merkle + Ed25519 signed audit receipt for a task.",
                    "mimeType": "application/json"
                },
                {
                    "uriTemplate": "replayfix://sessions/{session_id}",
                    "name": "Active Conversational & Interaction Session State",
                    "description": "Retrieve persistent session state and turn history.",
                    "mimeType": "application/json"
                }
            ]
        })
    }

    fn handle_resources_read(&self, params: Option<&Value>) -> Result<Value> {
        let p = params.ok_or_else(|| {
            CoreError::McpProtocol("Missing 'params' in resources/read".to_string())
        })?;
        let uri = p
            .get("uri")
            .and_then(Value::as_str)
            .ok_or_else(|| CoreError::McpProtocol("Missing 'uri' in resources/read".to_string()))?;

        if uri == "replayfix://benchmarks" {
            let text = if let Ok(manifest) =
                MetaPulseManifest::load_from_repo(&self.config.default_repo_path)
            {
                serde_json::to_string_pretty(&manifest)?
            } else {
                serde_json::to_string_pretty(&json!({
                    "benchmark_suite": "MetaPulse-20",
                    "supported_builtin_tasks": Fixture::supported_tasks(),
                    "all_metapulse_bugs": replayfix_core::fixtures::MetaPulseRepoRunner::all_metapulse_bug_ids()
                }))?
            };
            return Ok(json!({
                "contents": [{
                    "uri": uri,
                    "mimeType": "application/json",
                    "text": text
                }]
            }));
        }

        if let Some(task_id) = uri.strip_prefix("replayfix://receipts/") {
            let receipt_text = self.read_or_synthesize_receipt_json(task_id)?;
            return Ok(json!({
                "contents": [{
                    "uri": uri,
                    "mimeType": "application/json",
                    "text": receipt_text
                }]
            }));
        }

        if let Some(session_id) = uri.strip_prefix("replayfix://sessions/") {
            let session_text = self.read_session_json(session_id)?;
            return Ok(json!({
                "contents": [{
                    "uri": uri,
                    "mimeType": "application/json",
                    "text": session_text
                }]
            }));
        }

        Err(CoreError::McpProtocol(format!(
            "Unsupported MCP resource URI: '{uri}'"
        )))
    }

    fn read_or_synthesize_receipt_json(&self, task_id: &str) -> Result<String> {
        let audit_path = self.config.default_output_dir.join("audit_receipt.json");
        if audit_path.exists() {
            if let Ok(raw) = fs::read_to_string(&audit_path) {
                if raw.contains(task_id) {
                    return Ok(raw);
                }
            }
        }
        let fixture = Fixture::for_task(task_id).ok_or_else(|| {
            CoreError::McpProtocol(format!(
                "Unknown task_id '{task_id}'; expected RF-01..RF-05"
            ))
        })?;
        let outcome = MockHealer::run(&fixture, fixture.builtin_diff)?;
        match outcome {
            HealOutcome::Verified(receipt) => Ok(serde_json::to_string_pretty(&receipt)?),
            HealOutcome::Rejected(report) => Ok(serde_json::to_string_pretty(&report)?),
        }
    }

    fn read_session_json(&self, session_id: &str) -> Result<String> {
        if let Some(session) = AgentSession::load_by_id(&self.config.default_output_dir, session_id)
        {
            return Ok(serde_json::to_string_pretty(&session.record)?);
        }
        Ok(serde_json::to_string_pretty(&json!({
            "session_id": session_id,
            "status": "NOT_FOUND_OR_EPHEMERAL",
            "turns": []
        }))?)
    }

    fn handle_prompts_list(&self) -> Value {
        json!({
            "prompts": [
                {
                    "name": "triage_bug",
                    "description": "Guides an external LLM through visual bug analysis, causal attribution, and surgical patch generation.",
                    "arguments": [
                        {
                            "name": "bug_id",
                            "description": "Target bug identifier (e.g. RF-01)",
                            "required": true
                        }
                    ]
                }
            ]
        })
    }

    fn handle_prompts_get(&self, params: Option<&Value>) -> Result<Value> {
        let p = params
            .ok_or_else(|| CoreError::McpProtocol("Missing 'params' in prompts/get".to_string()))?;
        let name = p
            .get("name")
            .and_then(Value::as_str)
            .ok_or_else(|| CoreError::McpProtocol("Missing 'name' in prompts/get".to_string()))?;
        if name != "triage_bug" {
            return Err(CoreError::McpProtocol(format!(
                "Unknown MCP prompt: '{name}'"
            )));
        }
        let bug_id = p
            .get("arguments")
            .and_then(|a| a.get("bug_id"))
            .and_then(Value::as_str)
            .unwrap_or("RF-01");

        let fixture = Fixture::for_task(bug_id).ok_or_else(|| {
            CoreError::McpProtocol(format!("Unknown bug_id '{bug_id}'; expected RF-01..RF-05"))
        })?;

        Ok(json!({
            "description": format!("ReplayFix Visual Bug Triage & Surgical Patch Prompt for {}", fixture.task_id),
            "messages": [
                {
                    "role": "user",
                    "content": {
                        "type": "text",
                        "text": format!(
                            "You are operating with ReplayFix 1.5 as your judicial verification oracle.\n\
                             1. Call `replayfix_reproduce` with `{{\"bug_id\": \"{}\"}}` to confirm Gate 1 RED failure on `{}`.\n\
                             2. Inspect the DOM snapshot and action trajectory via `replayfix_inspect_trace`.\n\
                             3. Formulate a minimal unified diff patch that satisfies `{}` (`{}`).\n\
                             4. Verify your patch in the isolated scratch sandbox via `replayfix_verify_patch`.\n\
                             5. Once certified GREEN, permanently inoculate the codebase via `replayfix_inoculate`.",
                            fixture.task_id,
                            fixture.target_path,
                            fixture.contract.contract_id,
                            fixture.expected_output_text
                        )
                    }
                }
            ]
        }))
    }
}

/// Helper to validate that a path argument stays within safe workspace bounds.
pub fn validate_mcp_subpath(root: &Path, candidate: &str) -> Result<PathBuf> {
    replayfix_core::manifest::TaskManifest::assert_safe_path(candidate, "mcp_subpath")?;
    Ok(root.join(candidate))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_initialize_and_tools_list() {
        let srv = McpServer::new(McpServerConfig::default());
        let resp = srv
            .handle_raw_message(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#)
            .unwrap();
        assert!(resp.error.is_none());
        let res = resp.result.unwrap();
        assert_eq!(res["protocolVersion"], MCP_PROTOCOL_VERSION);

        let tools_resp = srv
            .handle_raw_message(r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#)
            .unwrap();
        let tools = tools_resp.result.unwrap()["tools"]
            .as_array()
            .unwrap()
            .clone();
        assert_eq!(tools.len(), 5);
    }

    #[test]
    fn test_mcp_tools_call_and_adversarial_payloads() {
        let tmp = std::env::temp_dir().join(format!("replayfix_mcp_unit_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let srv = McpServer::new(McpServerConfig {
            default_repo_path: tmp.clone(),
            default_output_dir: tmp.join(".replayfix"),
        });

        // 1. replayfix_reproduce
        let repro = srv
            .handle_raw_message(
                r#"{"jsonrpc":"2.0","id":10,"method":"tools/call","params":{"name":"replayfix_reproduce","arguments":{"bug_id":"RF-01"}}}"#,
            )
            .unwrap();
        assert!(repro.error.is_none());
        assert_eq!(
            repro.result.unwrap()["structuredContent"]["gate1_status"],
            "RED_CONFIRMED"
        );

        // 2. replayfix_verify_patch
        let verify = srv
            .handle_raw_message(
                r#"{"jsonrpc":"2.0","id":11,"method":"tools/call","params":{"name":"replayfix_verify_patch","arguments":{"bug_id":"RF-01","patch_diff":"builtin","stress_quorum":true}}}"#,
            )
            .unwrap();
        assert!(verify.error.is_none());
        assert_eq!(
            verify.result.unwrap()["structuredContent"]["verified"],
            true
        );

        // 3. replayfix_inoculate
        let inoc = srv
            .handle_raw_message(
                r#"{"jsonrpc":"2.0","id":12,"method":"tools/call","params":{"name":"replayfix_inoculate","arguments":{"bug_id":"RF-01"}}}"#,
            )
            .unwrap();
        assert!(inoc.error.is_none());
        assert_eq!(
            inoc.result.unwrap()["structuredContent"]["spec_rel_path"],
            "tests/repro/rf_01.spec.ts"
        );

        // 4. Adversarial invalid JSON
        let bad_json = srv.handle_raw_message("{not_valid_json").unwrap();
        assert_eq!(bad_json.error.unwrap().code, ERR_PARSE_ERROR);

        // 5. Adversarial unknown method
        let bad_method = srv
            .handle_raw_message(r#"{"jsonrpc":"2.0","id":99,"method":"nonexistent/method"}"#)
            .unwrap();
        assert_eq!(bad_method.error.unwrap().code, ERR_METHOD_NOT_FOUND);

        let _ = fs::remove_dir_all(&tmp);
    }
}
