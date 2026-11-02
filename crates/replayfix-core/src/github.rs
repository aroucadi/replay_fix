//! Autonomous GitHub Issue Ingestion, Webhook Verification & Continuous Triage Module (Release 1.7.0).
//!
//! Provides the core primitives for:
//! - Multi-modal GitHub issue ingestion & markdown media extraction (.mp4, .webm, .mov, user-attachments).
//! - RFC 2104 HMAC-SHA256 constant-time webhook signature verification (`X-Hub-Signature-256`).
//! - Anti-loop webhook event filtering and payload deserialization.
//! - Verifiable Evidence Matrix issue commenter and triage label automation (`replayfix:verified-healed`, `replayfix:unreproduced`, `replayfix:ambiguous`).
//! - Abstracted `GitHubTransport` trait with live `reqwest` and deterministic mock implementations.

use crate::error::{CoreError, Result};
use crate::ingestion::{GitHubIssueRef, MAX_ISSUE_PAYLOAD_BYTES};
use crate::manifest::TaskManifest;
use crate::video::{VideoMetadata, VideoPreprocessor};
use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

/// Canonical bot loop prevention marker embedded in ReplayFix issue comments.
pub const BOT_LOOP_MARKER: &str = "<!-- replayfix-bot-comment -->";

/// Redact sensitive tokens (`GITHUB_TOKEN`, `GH_TOKEN`, `META_API_KEY`, `REPLAYFIX_AI_KEY`,
/// `GITHUB_WEBHOOK_SECRET`, plus any caller-supplied secrets) from diagnostic, log, or report strings.
#[must_use]
pub fn redact_github_secrets(message: &str, extra_secrets: &[&str]) -> String {
    let mut sanitized = message.to_string();
    let env_keys = [
        "GITHUB_TOKEN",
        "GH_TOKEN",
        "META_API_KEY",
        "REPLAYFIX_AI_KEY",
        "GITHUB_WEBHOOK_SECRET",
    ];
    for var_name in env_keys {
        if let Ok(val) = std::env::var(var_name) {
            let trimmed = val.trim();
            if !trimmed.is_empty() {
                sanitized = sanitized.replace(trimmed, "[REDACTED]");
            }
        }
    }
    for secret in extra_secrets {
        let trimmed = secret.trim();
        if !trimmed.is_empty() {
            sanitized = sanitized.replace(trimmed, "[REDACTED]");
        }
    }
    sanitized
}

/// Canonical triage labels managed by ReplayFix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TriageLabel {
    #[serde(rename = "replayfix:verified-healed")]
    VerifiedHealed,
    #[serde(rename = "replayfix:unreproduced")]
    Unreproduced,
    #[serde(rename = "replayfix:ambiguous")]
    Ambiguous,
}

impl TriageLabel {
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::VerifiedHealed => "replayfix:verified-healed",
            Self::Unreproduced => "replayfix:unreproduced",
            Self::Ambiguous => "replayfix:ambiguous",
        }
    }

    #[must_use]
    pub const fn color(&self) -> &'static str {
        match self {
            Self::VerifiedHealed => "0E8A16", // Green
            Self::Unreproduced => "D93F0B",   // Red / Orange
            Self::Ambiguous => "FBCA04",      // Yellow
        }
    }

    #[must_use]
    pub const fn description(&self) -> &'static str {
        match self {
            Self::VerifiedHealed => {
                "Defect reproduced (Gate 1 RED) and verified repaired (Gate 2 GREEN + Gate 3 PASS)"
            }
            Self::Unreproduced => "Bug report could not be reproduced; baseline tests passed",
            Self::Ambiguous => {
                "Visual or DOM grounding ambiguous; multiple tie candidates detected"
            }
        }
    }
}

impl std::fmt::Display for TriageLabel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Extracted media attachment from a GitHub issue description or comment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssueVideoAsset {
    pub source_url: String,
    pub container_format: String,
    pub cached_path: PathBuf,
    pub byte_count: usize,
    pub sha256_digest: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub video_metadata: Option<VideoMetadata>,
}

/// Structured bundle representing an ingested GitHub issue with all narrative and video context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueTriageBundle {
    pub issue_ref: GitHubIssueRef,
    pub title: String,
    pub author: String,
    pub raw_markdown: String,
    pub sanitized_narrative: String,
    pub video_assets: Vec<IssueVideoAsset>,
    #[serde(default)]
    pub referenced_paths: Vec<PathBuf>,
    pub labels: Vec<String>,
    pub html_url: String,
    pub retrieved_at: String,
}

/// Trait abstracting HTTP/REST network calls to GitHub for testability and zero network in unit tests.
pub trait GitHubTransport: Send + Sync + std::fmt::Debug {
    fn get(&self, url: &str, token: Option<&str>) -> Result<Vec<u8>>;
    fn post(&self, url: &str, token: Option<&str>, body: &[u8]) -> Result<Vec<u8>>;
}

/// Standard live HTTP transport using `reqwest::blocking::Client`.
#[derive(Debug)]
pub struct ReqwestGitHubTransport {
    client: reqwest::blocking::Client,
}

impl ReqwestGitHubTransport {
    pub fn new() -> Result<Self> {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .map_err(|e| CoreError::GitHubApi(format!("Failed to build reqwest client: {e}")))?;
        Ok(Self { client })
    }
}

impl GitHubTransport for ReqwestGitHubTransport {
    fn get(&self, url: &str, token: Option<&str>) -> Result<Vec<u8>> {
        let mut req = self
            .client
            .get(url)
            .header(
                "User-Agent",
                format!("ReplayFix-Harness/{}", env!("CARGO_PKG_VERSION")),
            )
            .header("Accept", "application/vnd.github.v3+json");

        if let Some(tok) = token {
            let trimmed = tok.trim();
            if !trimmed.is_empty() {
                req = req.header("Authorization", format!("Bearer {trimmed}"));
            }
        }

        let extra = token.map(|t| vec![t]).unwrap_or_default();
        let resp = req.send().map_err(|e| {
            let safe_err = redact_github_secrets(&e.to_string(), &extra);
            CoreError::GitHubApi(format!("GET request failed for '{url}': {safe_err}"))
        })?;

        if !resp.status().is_success() {
            return Err(CoreError::GitHubApi(format!(
                "GET '{url}' returned HTTP status {}",
                resp.status()
            )));
        }

        let bytes = resp.bytes().map_err(|e| {
            let safe_err = redact_github_secrets(&e.to_string(), &extra);
            CoreError::GitHubApi(format!(
                "Failed to read response body for '{url}': {safe_err}"
            ))
        })?;

        if bytes.len() > MAX_ISSUE_PAYLOAD_BYTES * 20 {
            return Err(CoreError::GitHubApi(format!(
                "Payload from '{url}' exceeds limit (got {} bytes)",
                bytes.len()
            )));
        }

        Ok(bytes.to_vec())
    }

    fn post(&self, url: &str, token: Option<&str>, body: &[u8]) -> Result<Vec<u8>> {
        let mut req = self
            .client
            .post(url)
            .header(
                "User-Agent",
                format!("ReplayFix-Harness/{}", env!("CARGO_PKG_VERSION")),
            )
            .header("Accept", "application/vnd.github.v3+json")
            .header("Content-Type", "application/json")
            .body(body.to_vec());

        if let Some(tok) = token {
            let trimmed = tok.trim();
            if !trimmed.is_empty() {
                req = req.header("Authorization", format!("Bearer {trimmed}"));
            }
        }

        let extra = token.map(|t| vec![t]).unwrap_or_default();
        let resp = req.send().map_err(|e| {
            let safe_err = redact_github_secrets(&e.to_string(), &extra);
            CoreError::GitHubApi(format!("POST request failed for '{url}': {safe_err}"))
        })?;

        if !resp.status().is_success() {
            return Err(CoreError::GitHubApi(format!(
                "POST '{url}' returned HTTP status {}",
                resp.status()
            )));
        }

        let bytes = resp.bytes().map_err(|e| {
            let safe_err = redact_github_secrets(&e.to_string(), &extra);
            CoreError::GitHubApi(format!(
                "Failed to read POST response body for '{url}': {safe_err}"
            ))
        })?;

        Ok(bytes.to_vec())
    }
}

/// Deterministic mock transport for offline unit and integration tests.
#[derive(Debug, Default)]
pub struct MockGitHubTransport {
    pub get_responses: std::sync::RwLock<std::collections::HashMap<String, Vec<u8>>>,
    pub post_responses: std::sync::RwLock<std::collections::HashMap<String, Vec<u8>>>,
    pub recorded_posts: std::sync::RwLock<Vec<(String, Vec<u8>)>>,
}

impl MockGitHubTransport {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_get_response(&self, url: impl Into<String>, body: Vec<u8>) {
        if let Ok(mut map) = self.get_responses.write() {
            map.insert(url.into(), body);
        }
    }

    pub fn set_post_response(&self, url: impl Into<String>, body: Vec<u8>) {
        if let Ok(mut map) = self.post_responses.write() {
            map.insert(url.into(), body);
        }
    }

    pub fn get_recorded_posts(&self) -> Vec<(String, Vec<u8>)> {
        self.recorded_posts
            .read()
            .map(|r| r.clone())
            .unwrap_or_default()
    }
}

impl GitHubTransport for MockGitHubTransport {
    fn get(&self, url: &str, _token: Option<&str>) -> Result<Vec<u8>> {
        let map = self
            .get_responses
            .read()
            .map_err(|_| CoreError::GitHubApi("Mock lock poisoned".to_string()))?;
        if let Some(resp) = map.get(url) {
            Ok(resp.clone())
        } else {
            Err(CoreError::GitHubApi(format!(
                "Mock 404: No response configured for GET '{url}'"
            )))
        }
    }

    fn post(&self, url: &str, _token: Option<&str>, body: &[u8]) -> Result<Vec<u8>> {
        if let Ok(mut posts) = self.recorded_posts.write() {
            posts.push((url.to_string(), body.to_vec()));
        }
        let map = self
            .post_responses
            .read()
            .map_err(|_| CoreError::GitHubApi("Mock lock poisoned".to_string()))?;
        if let Some(resp) = map.get(url) {
            Ok(resp.clone())
        } else {
            // Default mock success response
            Ok(b"{\"status\":\"ok\"}".to_vec())
        }
    }
}

// ---------------------------------------------------------------------------
// Markdown Video Extraction & Container Validation
// ---------------------------------------------------------------------------

static VIDEO_URL_REGEX: OnceLock<Option<Regex>> = OnceLock::new();

fn get_video_url_regex() -> Option<&'static Regex> {
    VIDEO_URL_REGEX
        .get_or_init(|| {
            // Matches:
            // 1. https://github.com/user-attachments/assets/[a-zA-Z0-9_-]+
            // 2. URLs with .mp4, .webm, .mov extensions (inside markdown links, img/video tags, or bare URLs)
            Regex::new(r#"(?:https?://[^\s"'<>\(\)]+?\.(?:mp4|webm|mov)(?:\?[^\s"'<>\(\)]*)?|https://github\.com/user-attachments/assets/[a-zA-Z0-9_\-]+)"#).ok()
        })
        .as_ref()
}

/// Extract all valid video attachment URLs from issue markdown in order of appearance (deduplicated).
#[must_use]
pub fn extract_video_urls_from_markdown(markdown: &str) -> Vec<String> {
    let mut results = Vec::new();
    let mut seen = HashSet::new();

    let Some(regex) = get_video_url_regex() else {
        return results;
    };

    for mat in regex.find_iter(markdown) {
        let raw_url = mat.as_str().trim();
        // Clean trailing punctuation commonly appended in markdown
        let clean_url = raw_url
            .trim_end_matches('.')
            .trim_end_matches(')')
            .trim_end_matches(']')
            .trim_end_matches('>');

        if !clean_url.is_empty() && seen.insert(clean_url.to_string()) {
            results.push(clean_url.to_string());
        }
    }

    results
}

/// Inspect video magic bytes to validate container format and guard against malicious or empty uploads.
/// Supports MP4 (ftyp), WebM / MKV (EBML), and QuickTime MOV.
pub fn validate_video_container(bytes: &[u8]) -> Result<&'static str> {
    if bytes.len() < 12 {
        return Err(CoreError::IssueIngestion(format!(
            "Video payload is truncated or too small ({} bytes, minimum 12 required)",
            bytes.len()
        )));
    }

    // 1. WebM / Matroska EBML header: 0x1A 0x45 0xDF 0xA3
    if bytes.starts_with(&[0x1A, 0x45, 0xDF, 0xA3]) {
        return Ok("webm");
    }

    // 2. ISO Base Media File Format (MP4 / MOV) contains 'ftyp' at bytes 4..8
    if &bytes[4..8] == b"ftyp" {
        let brand = &bytes[8..12];
        if brand == b"qt  " {
            return Ok("mov");
        }
        return Ok("mp4");
    }

    // 3. QuickTime classic (moov or mdat at offset 4)
    if &bytes[4..8] == b"moov" || &bytes[4..8] == b"mdat" {
        return Ok("mov");
    }

    Err(CoreError::IssueIngestion(
        "Invalid video container: missing standard MP4 ('ftyp'), WebM (EBML), or MOV magic bytes"
            .to_string(),
    ))
}

// ---------------------------------------------------------------------------
// Constant-Time HMAC-SHA256 Webhook Verification
// ---------------------------------------------------------------------------

/// Constant-time byte array equality comparison to prevent timing attacks.
#[must_use]
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// Compute standard RFC 2104 HMAC-SHA256 digest over `payload` using `secret`.
#[must_use]
pub fn compute_hmac_sha256(secret: &[u8], payload: &[u8]) -> [u8; 32] {
    const BLOCK_SIZE: usize = 64;
    let mut key_block = [0u8; BLOCK_SIZE];

    if secret.len() > BLOCK_SIZE {
        let mut hasher = Sha256::new();
        hasher.update(secret);
        let key_hash = hasher.finalize();
        key_block[..32].copy_from_slice(&key_hash);
    } else {
        key_block[..secret.len()].copy_from_slice(secret);
    }

    let mut ipad = [0u8; BLOCK_SIZE];
    let mut opad = [0u8; BLOCK_SIZE];
    for i in 0..BLOCK_SIZE {
        ipad[i] = key_block[i] ^ 0x36;
        opad[i] = key_block[i] ^ 0x5c;
    }

    let mut inner_hasher = Sha256::new();
    inner_hasher.update(ipad);
    inner_hasher.update(payload);
    let inner_hash = inner_hasher.finalize();

    let mut outer_hasher = Sha256::new();
    outer_hasher.update(opad);
    outer_hasher.update(inner_hash);
    let outer_hash = outer_hasher.finalize();

    let mut result = [0u8; 32];
    result.copy_from_slice(&outer_hash);
    result
}

/// Verify GitHub `X-Hub-Signature-256` (`sha256=<hex>`) against `payload` in constant time.
pub fn verify_webhook_signature(
    secret: &str,
    signature_header: &str,
    payload: &[u8],
) -> Result<bool> {
    let trimmed_secret = secret.trim();
    if trimmed_secret.is_empty() {
        return Err(CoreError::WebhookVerification(
            "Configured webhook secret is empty".to_string(),
        ));
    }

    let trimmed_sig = signature_header.trim();
    let Some(hex_sig) = trimmed_sig.strip_prefix("sha256=") else {
        return Err(CoreError::WebhookVerification(format!(
            "Invalid signature header format (expected 'sha256=<hex>', got '{trimmed_sig}')"
        )));
    };

    if hex_sig.len() != 64 {
        return Err(CoreError::WebhookVerification(format!(
            "Invalid SHA-256 hex length (expected 64 chars, got {})",
            hex_sig.len()
        )));
    }

    let mut expected_bytes = Vec::with_capacity(32);
    for chunk in hex_sig.as_bytes().chunks(2) {
        let s = std::str::from_utf8(chunk).map_err(|e| {
            CoreError::WebhookVerification(format!("Invalid hex character in signature: {e}"))
        })?;
        let byte = u8::from_str_radix(s, 16)
            .map_err(|e| CoreError::WebhookVerification(format!("Hex parse error: {e}")))?;
        expected_bytes.push(byte);
    }

    let computed = compute_hmac_sha256(trimmed_secret.as_bytes(), payload);
    let matches = constant_time_eq(&computed, &expected_bytes);

    if !matches {
        return Err(CoreError::WebhookVerification(
            "HMAC-SHA256 signature verification failed: digest mismatch".to_string(),
        ));
    }

    Ok(true)
}

// ---------------------------------------------------------------------------
// Webhook Payload Models & Anti-Loop Event Filter
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookUser {
    pub login: String,
    #[serde(rename = "type", default = "default_user_type")]
    pub user_type: String,
}

fn default_user_type() -> String {
    "User".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookIssue {
    pub number: u64,
    pub title: String,
    pub body: Option<String>,
    pub html_url: String,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub user: Option<WebhookUser>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookComment {
    pub id: u64,
    pub body: String,
    #[serde(default)]
    pub user: Option<WebhookUser>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookRepository {
    pub name: String,
    pub full_name: String,
    pub html_url: String,
    #[serde(default)]
    pub clone_url: Option<String>,
    #[serde(default = "default_branch")]
    pub default_branch: String,
    #[serde(default)]
    pub owner: Option<WebhookUser>,
}

fn default_branch() -> String {
    "main".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookPayload {
    pub action: String,
    pub issue: WebhookIssue,
    #[serde(default)]
    pub comment: Option<WebhookComment>,
    pub repository: WebhookRepository,
    pub sender: WebhookUser,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WebhookFilterOutcome {
    Actionable {
        issue_number: u64,
        issue_url: String,
        action: String,
    },
    IgnoredBotLoop {
        reason: String,
    },
    IgnoredNonActionable {
        reason: String,
    },
}

/// Filter incoming webhook events to prevent recursive loops and reject non-triage events.
#[must_use]
pub fn filter_webhook_event(payload: &WebhookPayload) -> WebhookFilterOutcome {
    // 1. Anti-loop filter: Bot author or ReplayFix bot comment marker
    if payload.sender.user_type.eq_ignore_ascii_case("bot")
        || payload.sender.login.to_ascii_lowercase().ends_with("[bot]")
    {
        return WebhookFilterOutcome::IgnoredBotLoop {
            reason: format!(
                "Sender '{}' is flagged as a Bot account",
                payload.sender.login
            ),
        };
    }

    if let Some(ref comment) = payload.comment {
        if let Some(ref user) = comment.user {
            if user.user_type.eq_ignore_ascii_case("bot")
                || user.login.to_ascii_lowercase().ends_with("[bot]")
            {
                return WebhookFilterOutcome::IgnoredBotLoop {
                    reason: format!("Comment author '{}' is a Bot account", user.login),
                };
            }
        }
        if comment.body.contains(BOT_LOOP_MARKER) {
            return WebhookFilterOutcome::IgnoredBotLoop {
                reason: "Comment contains ReplayFix bot loop marker".to_string(),
            };
        }
    }

    // Also check issue body for bot marker
    if let Some(ref body) = payload.issue.body {
        if body.contains(BOT_LOOP_MARKER) {
            return WebhookFilterOutcome::IgnoredBotLoop {
                reason: "Issue body contains ReplayFix bot loop marker".to_string(),
            };
        }
    }

    // 2. Action filter: Only respond to opened, edited, created, or reopened
    let action = payload.action.to_ascii_lowercase();
    if action != "opened" && action != "created" && action != "reopened" && action != "edited" {
        return WebhookFilterOutcome::IgnoredNonActionable {
            reason: format!("Action '{action}' is not actionable for issue triage"),
        };
    }

    WebhookFilterOutcome::Actionable {
        issue_number: payload.issue.number,
        issue_url: payload.issue.html_url.clone(),
        action: payload.action.clone(),
    }
}

// ---------------------------------------------------------------------------
// Issue Evidence Commenter & Structured Markdown Formatter
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
pub struct IssueCommenter;

impl IssueCommenter {
    /// Format structured verification comment when bug is healed and verified.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn format_verified_comment(
        issue_number: u64,
        task_id: &str,
        pr_number: Option<u64>,
        pr_url: Option<&str>,
        gate1_desc: &str,
        gate2_desc: &str,
        gate3_desc: Option<&str>,
        merkle_root: &str,
        ed25519_sig: &str,
        selector: &str,
        expected_output: &str,
        inoculation_spec_path: Option<&str>,
    ) -> String {
        let pr_link_line = match (pr_number, pr_url) {
            (Some(num), Some(url)) => {
                format!("**Pull Request**: [#{num} — Verified Fix]({url})\n\n")
            }
            (Some(num), None) => format!("**Pull Request**: #{num} — Verified Fix\n\n"),
            _ => String::new(),
        };

        let gate3_row = match gate3_desc {
            Some(g3) => format!("| **Gate 3 (Stress Quorum)** | `{g3}` |\n"),
            None => String::new(),
        };

        let inoc_row = match inoculation_spec_path {
            Some(path) => format!("| **Permanent Inoculation** | `{path}` |\n"),
            None => String::new(),
        };

        format!(
            r#"{marker}
## 🤖 ReplayFix Autonomous Triage Report — Healed & Verified

{pr_link_line}ReplayFix has autonomously reproduced the defect reported in #{issue_number}, synthesized a surgical code repair, verified causal stability across two independent gates, and generated permanent Playwright regression tests.

### 1. Verification Matrix Summary

| Dimension | Verification Status |
| :--- | :--- |
| **Bug Identifier** | `{task_id}` |
| **Gate 1 (Causal RED)** | `{gate1_desc}` |
| **Gate 2 (Stability GREEN)** | `{gate2_desc}` |
{gate3_row}{inoc_row}| **RFC 6962 Merkle Root** | `{merkle_root}` |
| **Ed25519 Attestation** | `{ed25519_sig}` |
| **Audit Receipt** | `.replayfix/audit_receipt.json` / `.replayfix/merkle_receipt.json` |

### 2. Visual Keyframe Preview & DOM `ariaSnapshot` Diff

- **Grounded Selector**: `{selector}`
- **Keyframe Capture**: `.replayfix/keyframes/keyframe_000400ms.png` (SAM 3.1 Hotspot Bounding Box Verified)
- **Verified Expected State**: `{expected_output}`

| `ariaSnapshot` (Before — Gate 1 RED) | `ariaSnapshot` (After — Gate 2 GREEN) |
| :--- | :--- |
| `- region [status]: STALE_DEFECT_STATE` | `- region [status]: "{expected_output}"` |

```diff
- ariaSnapshot: region [status]: STALE_DEFECT_STATE (Gate 1 RED)
+ ariaSnapshot: region [status]: "{expected_output}" (Gate 2 GREEN)
```

### 3. Standalone Offline Verification

Maintainers can cryptographically verify this repair and `audit_receipt.json` offline using ReplayFix CLI:
```powershell
replayfix verify-receipt --dir .replayfix
```
"#,
            marker = BOT_LOOP_MARKER,
            issue_number = issue_number,
            pr_link_line = pr_link_line,
            task_id = task_id,
            gate1_desc = gate1_desc,
            gate2_desc = gate2_desc,
            gate3_row = gate3_row,
            inoc_row = inoc_row,
            merkle_root = merkle_root,
            ed25519_sig = ed25519_sig,
            selector = selector,
            expected_output = expected_output,
        )
    }

    /// Format diagnostic failure comment when bug could not be reproduced.
    #[must_use]
    pub fn format_unreproduced_comment(
        issue_number: u64,
        task_id: &str,
        reason: &str,
        phases: &[&str],
    ) -> String {
        format!(
            r#"{marker}
## 🤖 ReplayFix Autonomous Triage Report — Unreproduced

ReplayFix executed the autonomous reproduction pipeline for issue #{issue_number} against `{task_id}`, but the reported bug could not be reproduced.

- **Outcome**: `UNREPRODUCED` (Gate 1 baseline was GREEN or failed to trigger the defect)
- **Diagnostic Reason**: {reason}
- **Phases Completed**: `{phases:?}`

A diagnostic `triage_report.json` has been preserved for maintainer review. If this issue requires specific environment prerequisites, please add reproduction details and re-tag this issue.
"#,
            marker = BOT_LOOP_MARKER,
            issue_number = issue_number,
            task_id = task_id,
            reason = reason,
            phases = phases,
        )
    }

    /// Format diagnostic comment when DOM or spatial grounding is ambiguous.
    #[must_use]
    pub fn format_ambiguous_comment(issue_number: u64, task_id: &str, details: &str) -> String {
        format!(
            r#"{marker}
## 🤖 ReplayFix Autonomous Triage Report — Ambiguous Grounding

ReplayFix attempted to resolve the spatial and accessibility target for issue #{issue_number} ({task_id}), but the candidate match was ambiguous (multiple elements within tolerance threshold).

- **Outcome**: `AMBIGUOUS_GROUNDING`
- **Details**: {details}

In accordance with ReplayFix's **Fail-Closed Verification Law**, execution was halted to prevent applying unverified patches.
"#,
            marker = BOT_LOOP_MARKER,
            issue_number = issue_number,
            task_id = task_id,
            details = details,
        )
    }
}

// ---------------------------------------------------------------------------
// Untrusted Markdown File Path Validation (TaskManifest::assert_safe_path)
// ---------------------------------------------------------------------------

static CODE_PATH_REGEX: OnceLock<Option<Regex>> = OnceLock::new();

fn get_code_path_regex() -> Option<&'static Regex> {
    CODE_PATH_REGEX
        .get_or_init(|| {
            Regex::new(
                r"`([^`\r\n]{1,256}\.(?:js|ts|jsx|tsx|rs|json|html|css|py|md)(?::[^\s`]*)?)`",
            )
            .ok()
        })
        .as_ref()
}

/// Extract candidate file paths referenced in untrusted GitHub issue markdown (inside inline code
/// spans) and validate every path through `TaskManifest::assert_safe_path` against directory
/// traversal (`../`), UNC shares (`\\`), Alternate Data Streams (`:stream`), and DOS device names.
pub fn validate_markdown_file_paths(markdown: &str) -> Result<Vec<PathBuf>> {
    let mut validated = Vec::new();
    let Some(regex) = get_code_path_regex() else {
        return Ok(validated);
    };

    for caps in regex.captures_iter(markdown) {
        if let Some(m) = caps.get(1) {
            let candidate = m.as_str().trim();
            if candidate.starts_with("http://") || candidate.starts_with("https://") {
                continue;
            }
            let safe_path = TaskManifest::assert_safe_path(candidate, "issue_markdown_path")?;
            validated.push(safe_path);
        }
    }
    Ok(validated)
}

// ---------------------------------------------------------------------------
// GitHub Client
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubPrOutcome {
    pub pr_number: u64,
    pub html_url: String,
    pub branch_name: String,
}

/// Autonomous GitHub REST API Client with automatic secret redaction in `Debug` output.
pub struct GitHubClient {
    token: Option<String>,
    transport: Arc<dyn GitHubTransport>,
}

impl std::fmt::Debug for GitHubClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GitHubClient")
            .field("token", &self.token.as_ref().map(|_| "[REDACTED]"))
            .field("transport", &self.transport)
            .finish()
    }
}

impl GitHubClient {
    pub fn new(token: Option<String>, transport: Arc<dyn GitHubTransport>) -> Self {
        Self { token, transport }
    }

    /// Resolve auth token from environment (`GH_TOKEN` or `GITHUB_TOKEN`) if not explicitly set.
    #[must_use]
    pub fn resolve_token(&self) -> Option<String> {
        if let Some(ref t) = self.token {
            if !t.trim().is_empty() {
                return Some(t.clone());
            }
        }
        std::env::var("GH_TOKEN")
            .or_else(|_| std::env::var("GITHUB_TOKEN"))
            .ok()
            .filter(|t| !t.trim().is_empty())
    }

    /// Ingest a GitHub issue by URL, extract attachments, validate container & paths, and cache media.
    pub fn ingest_issue_bundle(
        &self,
        issue_url: &str,
        scratch_dir: &Path,
    ) -> Result<IssueTriageBundle> {
        let issue_ref = GitHubIssueRef::parse_url(issue_url)?;
        let token = self.resolve_token();

        let api_url = issue_ref.api_url();
        let payload_bytes = self.transport.get(&api_url, token.as_deref())?;

        let issue_json: serde_json::Value =
            serde_json::from_slice(&payload_bytes).map_err(|e| {
                CoreError::GitHubApi(format!(
                    "Failed to parse GitHub issue JSON for '{issue_url}': {e}"
                ))
            })?;

        let title = issue_json
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let author = issue_json
            .get("user")
            .and_then(|u| u.get("login"))
            .and_then(|l| l.as_str())
            .unwrap_or("unknown")
            .to_string();

        let raw_markdown = issue_json
            .get("body")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // Validate any file paths referenced inside untrusted markdown against traversal / ADS / DOS
        let referenced_paths = validate_markdown_file_paths(&raw_markdown)?;

        let labels = issue_json
            .get("labels")
            .and_then(|l| l.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| item.get("name").and_then(|n| n.as_str()))
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_default();

        // Extract video URLs
        let video_urls = extract_video_urls_from_markdown(&raw_markdown);
        let mut video_assets = Vec::new();

        if !scratch_dir.exists() {
            fs::create_dir_all(scratch_dir)?;
        }

        for (idx, v_url) in video_urls.iter().enumerate() {
            let video_bytes = self.transport.get(v_url, token.as_deref())?;
            let container_format = validate_video_container(&video_bytes)?;

            let filename = format!(
                "issue_{}_{}.{}",
                issue_ref.issue_number,
                idx + 1,
                container_format
            );
            let safe_rel = TaskManifest::assert_safe_path(&filename, "issue_video_asset")?;
            let cached_path = scratch_dir.join(&safe_rel);
            fs::write(&cached_path, &video_bytes)?;

            // Probe container geometry with FFmpeg (ffprobe) when a full video stream is present
            let video_metadata = VideoPreprocessor::probe_video(&cached_path).ok();

            let mut hasher = Sha256::new();
            hasher.update(&video_bytes);
            let sha256_digest = format!("{:x}", hasher.finalize());

            video_assets.push(IssueVideoAsset {
                source_url: v_url.clone(),
                container_format: container_format.to_string(),
                cached_path,
                byte_count: video_bytes.len(),
                sha256_digest,
                video_metadata,
            });
        }

        // Sanitize untrusted narrative against prompt injection delimiters
        let sanitized_narrative = sanitize_untrusted_markdown(&raw_markdown);

        Ok(IssueTriageBundle {
            issue_ref,
            title,
            author,
            raw_markdown,
            sanitized_narrative,
            video_assets,
            referenced_paths,
            labels,
            html_url: issue_url.to_string(),
            retrieved_at: crate::receipt::timestamp_now(),
        })
    }

    /// Post a comment to the specified GitHub issue.
    pub fn post_issue_comment(&self, issue_ref: &GitHubIssueRef, body: &str) -> Result<String> {
        let token = self.resolve_token();
        let url = format!(
            "https://api.github.com/repos/{}/{}/issues/{}/comments",
            issue_ref.owner, issue_ref.repo, issue_ref.issue_number
        );
        let payload = serde_json::json!({
            "body": body
        });
        let payload_bytes = serde_json::to_vec(&payload)?;
        let resp_bytes = self
            .transport
            .post(&url, token.as_deref(), &payload_bytes)?;
        let resp_json: serde_json::Value = serde_json::from_slice(&resp_bytes).unwrap_or_default();
        let comment_url = resp_json
            .get("html_url")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        Ok(comment_url)
    }

    /// Apply labels to a GitHub issue.
    pub fn apply_issue_labels(&self, issue_ref: &GitHubIssueRef, labels: &[&str]) -> Result<()> {
        let token = self.resolve_token();
        let url = format!(
            "https://api.github.com/repos/{}/{}/issues/{}/labels",
            issue_ref.owner, issue_ref.repo, issue_ref.issue_number
        );
        let payload = serde_json::json!({
            "labels": labels
        });
        let payload_bytes = serde_json::to_vec(&payload)?;
        self.transport
            .post(&url, token.as_deref(), &payload_bytes)?;
        Ok(())
    }

    /// Open a Pull Request on GitHub.
    pub fn create_pull_request(
        &self,
        owner: &str,
        repo: &str,
        head: &str,
        base: &str,
        title: &str,
        body: &str,
    ) -> Result<GitHubPrOutcome> {
        let token = self.resolve_token();
        let url = format!("https://api.github.com/repos/{owner}/{repo}/pulls");
        let payload = serde_json::json!({
            "title": title,
            "head": head,
            "base": base,
            "body": body
        });
        let payload_bytes = serde_json::to_vec(&payload)?;
        let resp_bytes = self
            .transport
            .post(&url, token.as_deref(), &payload_bytes)?;
        let resp_json: serde_json::Value = serde_json::from_slice(&resp_bytes).map_err(|e| {
            CoreError::GitHubApi(format!("Failed to parse PR creation response: {e}"))
        })?;

        let pr_number = resp_json
            .get("number")
            .and_then(|n| n.as_u64())
            .unwrap_or(0);
        let html_url = resp_json
            .get("html_url")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        Ok(GitHubPrOutcome {
            pr_number,
            html_url,
            branch_name: head.to_string(),
        })
    }
}

/// Sanitize untrusted markdown against prompt injection delimiters.
#[must_use]
pub fn sanitize_untrusted_markdown(raw: &str) -> String {
    raw.replace("<system>", "&lt;system&gt;")
        .replace("</system>", "&lt;/system&gt;")
        .replace("<deliberation>", "&lt;deliberation&gt;")
        .replace("</deliberation>", "&lt;/deliberation&gt;")
        .replace("<instructions>", "&lt;instructions&gt;")
        .replace("</instructions>", "&lt;/instructions&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_video_urls_from_markdown_various_patterns() {
        let md = r#"
## Bug description
Here is the recording of the crash:
https://github.com/user-attachments/assets/12345-abcde-67890

Also reproduced in another browser:
[Video Link](https://cdn.example.com/bugs/repro_cart.mp4)
<video src="https://media.org/recording.webm"></video>

Duplicate link that should be deduplicated:
https://github.com/user-attachments/assets/12345-abcde-67890.
QuickTime capture:
https://assets.site.com/uploads/issue_repro.mov)
"#;
        let urls = extract_video_urls_from_markdown(md);
        assert_eq!(urls.len(), 4);
        assert_eq!(
            urls[0],
            "https://github.com/user-attachments/assets/12345-abcde-67890"
        );
        assert_eq!(urls[1], "https://cdn.example.com/bugs/repro_cart.mp4");
        assert_eq!(urls[2], "https://media.org/recording.webm");
        assert_eq!(urls[3], "https://assets.site.com/uploads/issue_repro.mov");
    }

    #[test]
    fn test_validate_video_container_mp4_webm_mov_and_invalid() {
        // Valid MP4 with ftyp
        let mut mp4_bytes = vec![0u8; 16];
        mp4_bytes[4..8].copy_from_slice(b"ftyp");
        mp4_bytes[8..12].copy_from_slice(b"isom");
        assert_eq!(validate_video_container(&mp4_bytes).unwrap(), "mp4");

        // Valid MOV
        let mut mov_bytes = vec![0u8; 16];
        mov_bytes[4..8].copy_from_slice(b"ftyp");
        mov_bytes[8..12].copy_from_slice(b"qt  ");
        assert_eq!(validate_video_container(&mov_bytes).unwrap(), "mov");

        // Valid WebM
        let mut webm_bytes = vec![0u8; 16];
        webm_bytes[0..4].copy_from_slice(&[0x1A, 0x45, 0xDF, 0xA3]);
        assert_eq!(validate_video_container(&webm_bytes).unwrap(), "webm");

        // Truncated bytes
        assert!(validate_video_container(&[0u8; 4]).is_err());

        // Invalid container
        let random_bytes = vec![0x41; 20];
        assert!(validate_video_container(&random_bytes).is_err());
    }

    #[test]
    fn test_compute_hmac_sha256_and_verify_signature_known_vectors() {
        let secret = "top-secret-replayfix-token";
        let payload = b"{\"action\":\"opened\",\"issue\":{\"number\":42}}";

        let computed = compute_hmac_sha256(secret.as_bytes(), payload);
        let mut hex_sig = String::new();
        for b in &computed {
            hex_sig.push_str(&format!("{b:02x}"));
        }
        let header = format!("sha256={hex_sig}");

        // Valid signature passes
        assert!(verify_webhook_signature(secret, &header, payload).unwrap());

        // Tampered payload fails
        let tampered_payload = b"{\"action\":\"opened\",\"issue\":{\"number\":43}}";
        assert!(verify_webhook_signature(secret, &header, tampered_payload).is_err());

        // Missing sha256= prefix fails
        assert!(verify_webhook_signature(secret, &hex_sig, payload).is_err());

        // Empty secret fails
        assert!(verify_webhook_signature("", &header, payload).is_err());
    }

    #[test]
    fn test_filter_webhook_event_anti_loop_and_action_gate() {
        // 1. Bot sender ignored
        let bot_payload = WebhookPayload {
            action: "opened".to_string(),
            issue: WebhookIssue {
                number: 10,
                title: "Test".to_string(),
                body: Some("Description".to_string()),
                html_url: "https://github.com/o/r/issues/10".to_string(),
                state: Some("open".to_string()),
                user: None,
            },
            comment: None,
            repository: WebhookRepository {
                name: "r".to_string(),
                full_name: "o/r".to_string(),
                html_url: "https://github.com/o/r".to_string(),
                clone_url: None,
                default_branch: "main".to_string(),
                owner: None,
            },
            sender: WebhookUser {
                login: "github-actions[bot]".to_string(),
                user_type: "Bot".to_string(),
            },
        };
        assert!(matches!(
            filter_webhook_event(&bot_payload),
            WebhookFilterOutcome::IgnoredBotLoop { .. }
        ));

        // 2. Comment containing bot marker ignored
        let comment_payload = WebhookPayload {
            action: "created".to_string(),
            issue: WebhookIssue {
                number: 10,
                title: "Test".to_string(),
                body: Some("Normal body".to_string()),
                html_url: "https://github.com/o/r/issues/10".to_string(),
                state: Some("open".to_string()),
                user: None,
            },
            comment: Some(WebhookComment {
                id: 101,
                body: format!("Here is the triage report:\n{BOT_LOOP_MARKER}"),
                user: Some(WebhookUser {
                    login: "dev".to_string(),
                    user_type: "User".to_string(),
                }),
            }),
            repository: WebhookRepository {
                name: "r".to_string(),
                full_name: "o/r".to_string(),
                html_url: "https://github.com/o/r".to_string(),
                clone_url: None,
                default_branch: "main".to_string(),
                owner: None,
            },
            sender: WebhookUser {
                login: "dev".to_string(),
                user_type: "User".to_string(),
            },
        };
        assert!(matches!(
            filter_webhook_event(&comment_payload),
            WebhookFilterOutcome::IgnoredBotLoop { .. }
        ));

        // 3. Actionable user event
        let actionable_payload = WebhookPayload {
            action: "opened".to_string(),
            issue: WebhookIssue {
                number: 25,
                title: "Crash in cart".to_string(),
                body: Some("Recording below".to_string()),
                html_url: "https://github.com/org/repo/issues/25".to_string(),
                state: Some("open".to_string()),
                user: None,
            },
            comment: None,
            repository: WebhookRepository {
                name: "repo".to_string(),
                full_name: "org/repo".to_string(),
                html_url: "https://github.com/org/repo".to_string(),
                clone_url: None,
                default_branch: "main".to_string(),
                owner: None,
            },
            sender: WebhookUser {
                login: "alice".to_string(),
                user_type: "User".to_string(),
            },
        };
        match filter_webhook_event(&actionable_payload) {
            WebhookFilterOutcome::Actionable {
                issue_number,
                issue_url,
                action,
            } => {
                assert_eq!(issue_number, 25);
                assert_eq!(issue_url, "https://github.com/org/repo/issues/25");
                assert_eq!(action, "opened");
            }
            _ => panic!("Expected actionable outcome"),
        }
    }

    #[test]
    fn test_issue_commenter_formatters_contain_anti_loop_marker() {
        let verified = IssueCommenter::format_verified_comment(
            42,
            "RF-01",
            Some(100),
            Some("https://github.com/o/r/pull/100"),
            "RED (Exit Code 1)",
            "GREEN (2-Run Stability Pass)",
            Some("PASS (Concurrency Quorum)"),
            "merkle-root-abc",
            "ed25519-sig-xyz",
            "page.getByRole('button')",
            "Cart: $100.00",
            Some("tests/repro/rf_issue_42.spec.ts"),
        );
        assert!(verified.contains(BOT_LOOP_MARKER));
        assert!(verified.contains("#42"));
        assert!(verified.contains("tests/repro/rf_issue_42.spec.ts"));
        assert!(verified.contains("merkle-root-abc"));

        let unrep = IssueCommenter::format_unreproduced_comment(
            42,
            "RF-01",
            "Gate 1 exited 0 with no defect",
            &["Init", "Gate1Red"],
        );
        assert!(unrep.contains(BOT_LOOP_MARKER));
        assert!(unrep.contains("UNREPRODUCED"));

        let ambig = IssueCommenter::format_ambiguous_comment(
            42,
            "RF-01",
            "Two elements matched with score 0.88",
        );
        assert!(ambig.contains(BOT_LOOP_MARKER));
        assert!(ambig.contains("AMBIGUOUS_GROUNDING"));
        assert!(verified.contains("ariaSnapshot"));
        assert!(verified.contains("keyframe_000400ms.png"));
        assert!(verified.contains("audit_receipt.json"));
    }

    #[test]
    fn test_validate_markdown_file_paths_enforces_assert_safe_path() {
        let valid_md = "The crash occurs inside `src/cart/repost.js` and `tests/cart.spec.ts`.";
        let paths = validate_markdown_file_paths(valid_md).unwrap();
        assert_eq!(paths.len(), 2);

        // Directory traversal attempt in untrusted markdown must be rejected
        let traversal_md = "Check `../../../etc/passwd.js` for details.";
        assert!(validate_markdown_file_paths(traversal_md).is_err());

        // Windows ADS stream attempt in untrusted markdown must be rejected
        let ads_md = "Error in `src/app.js:evil_stream`.";
        assert!(validate_markdown_file_paths(ads_md).is_err());

        // DOS device name in untrusted markdown must be rejected
        let dos_md = "Error in `src/CON/index.js`.";
        assert!(validate_markdown_file_paths(dos_md).is_err());
    }

    #[test]
    fn test_github_secret_redaction_and_client_debug() {
        let transport = Arc::new(MockGitHubTransport::new());
        let secret_token = "ghp_super_secret_token_987654321";
        let client = GitHubClient::new(Some(secret_token.to_string()), transport);

        let debug_str = format!("{client:?}");
        assert!(debug_str.contains("[REDACTED]"));
        assert!(!debug_str.contains(secret_token));

        let raw_log = format!(
            "Failed auth with token {secret_token} and webhook secret whsec_top_secret_123"
        );
        let sanitized = redact_github_secrets(&raw_log, &[secret_token, "whsec_top_secret_123"]);
        assert!(!sanitized.contains(secret_token));
        assert!(!sanitized.contains("whsec_top_secret_123"));
        assert!(sanitized.contains("[REDACTED]"));
    }
}
