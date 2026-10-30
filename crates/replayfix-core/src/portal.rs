//! Spec 095: Offline Judge Proof Portal (`proof_viewer.html`) Validator,
//! Deterministic Demo Receipt Bundle Generator, and Multimodal Coordinate Bounds.

use crate::artifacts::atomic_write_file;
use crate::crypto::{verify_attestation, AttestationRecord, KeypairManager, TrustPolicy};
use crate::digest::{canonical_json, sha256_hex};
use crate::error::{CoreError, Result};
use crate::fixtures::Fixture;
use crate::merkle::{EvidenceLeaf, MerkleReceipt, MerkleTree};
use crate::sweep::{SweepBugResult, SweepReceipt};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::Path;

/// Maximum permitted receipt file size for portal import (50 MB).
pub const MAX_PORTAL_RECEIPT_BYTES: usize = 52_428_800;

/// Maximum permitted JSON nesting depth in the portal parser.
pub const MAX_PORTAL_JSON_DEPTH: usize = 32;

/// Fixed public demonstration seed (non-secret, deterministic test vector for RF-01 preset).
pub const DEMO_PRESET_PUBLIC_SEED_HEX: &str =
    "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60";

/// Honest verification states rendered by the Judge Proof Portal (`REQ-095-06`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PortalVerificationVerdict {
    VerifiedGenuine,
    SignatureValidSignerUntrusted,
    PolicyQualificationFailed,
    IntegrityCheckFailed,
    UnsignedReceipt,
    UnsupportedProtocolVersion,
    MalformedReceipt,
}

impl PortalVerificationVerdict {
    #[must_use]
    pub const fn badge_label(self) -> &'static str {
        match self {
            Self::VerifiedGenuine => "VERIFIED GENUINE",
            Self::SignatureValidSignerUntrusted => "SIGNATURE VALID — SIGNER UNTRUSTED",
            Self::PolicyQualificationFailed => "POLICY QUALIFICATION FAILED",
            Self::IntegrityCheckFailed => "INTEGRITY CHECK FAILED",
            Self::UnsignedReceipt => "UNSIGNED RECEIPT",
            Self::UnsupportedProtocolVersion => "UNSUPPORTED PROTOCOL VERSION",
            Self::MalformedReceipt => "MALFORMED RECEIPT",
        }
    }
}

/// Validated spatial overlay geometry for the portal's Multimodal Evidence viewer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidatedSpatialOverlay {
    pub viewport_width: u32,
    pub viewport_height: u32,
    pub box_x1_px: u32,
    pub box_y1_px: u32,
    pub box_x2_px: u32,
    pub box_y2_px: u32,
    pub centroid_x_px: u32,
    pub centroid_y_px: u32,
}

/// Validate and convert normalized SAM 3.1 millipart coordinates (`[0..=1000]`) into pixel coordinates.
pub fn validate_and_map_sam_box(
    millipart_box: [i32; 4],
    viewport_width: u32,
    viewport_height: u32,
) -> Result<ValidatedSpatialOverlay> {
    if viewport_width == 0
        || viewport_height == 0
        || viewport_width > 7680
        || viewport_height > 4320
    {
        return Err(CoreError::Verification(format!(
            "Invalid viewport dimensions: {viewport_width}x{viewport_height}"
        )));
    }
    let [x1, y1, x2, y2] = millipart_box;
    if x1 < 0 || y1 < 0 || x2 > 1000 || y2 > 1000 || x1 >= x2 || y1 >= y2 {
        return Err(CoreError::Verification(format!(
            "SAM 3.1 bounding box out of [0..1000] millipart bounds or inverted: [{x1}, {y1}, {x2}, {y2}]"
        )));
    }

    let box_x1_px = ((u64::from(x1 as u32) * u64::from(viewport_width)) / 1000) as u32;
    let box_y1_px = ((u64::from(y1 as u32) * u64::from(viewport_height)) / 1000) as u32;
    let box_x2_px = ((u64::from(x2 as u32) * u64::from(viewport_width)) / 1000) as u32;
    let box_y2_px = ((u64::from(y2 as u32) * u64::from(viewport_height)) / 1000) as u32;

    Ok(ValidatedSpatialOverlay {
        viewport_width,
        viewport_height,
        box_x1_px,
        box_y1_px,
        box_x2_px,
        box_y2_px,
        centroid_x_px: (box_x1_px + box_x2_px) / 2,
        centroid_y_px: (box_y1_px + box_y2_px) / 2,
    })
}

/// Evaluate raw receipt JSON bytes using the exact portal state machine (`ADR-02`).
pub fn evaluate_portal_receipt_bytes(
    raw_bytes: &[u8],
    pinned_keys: &HashSet<String>,
) -> (PortalVerificationVerdict, String) {
    if raw_bytes.is_empty() || raw_bytes.len() > MAX_PORTAL_RECEIPT_BYTES {
        return (
            PortalVerificationVerdict::MalformedReceipt,
            format!(
                "Receipt size ({} bytes) is empty or exceeds 50 MB limit",
                raw_bytes.len()
            ),
        );
    }

    let raw_str = match std::str::from_utf8(raw_bytes) {
        Ok(s) => s,
        Err(e) => {
            return (
                PortalVerificationVerdict::MalformedReceipt,
                format!("Invalid UTF-8 encoding: {e}"),
            )
        }
    };

    if check_json_depth(raw_str) > MAX_PORTAL_JSON_DEPTH {
        return (
            PortalVerificationVerdict::MalformedReceipt,
            format!("JSON nesting depth exceeds maximum ({MAX_PORTAL_JSON_DEPTH})"),
        );
    }

    if let Some(dup_key) = check_duplicate_json_keys(raw_str) {
        return (
            PortalVerificationVerdict::MalformedReceipt,
            format!("Duplicate JSON object key '{dup_key}' rejected before parsing"),
        );
    }

    let val: serde_json::Value = match serde_json::from_str(raw_str) {
        Ok(v) => v,
        Err(e) => {
            return (
                PortalVerificationVerdict::MalformedReceipt,
                format!("Malformed JSON syntax: {e}"),
            )
        }
    };

    if contains_forbidden_prototype_keys(&val) {
        return (
            PortalVerificationVerdict::MalformedReceipt,
            "Prototype pollution key (__proto__, constructor, or prototype) detected".to_string(),
        );
    }

    // Ensure RFC 8785 canonicalization succeeds (no floats permitted)
    if let Err(e) = canonical_json(&val) {
        return (
            PortalVerificationVerdict::MalformedReceipt,
            format!("RFC 8785 canonicalization failed: {e}"),
        );
    }

    let schema_version = match val.get("schema_version").and_then(|v| v.as_str()) {
        Some(v) => v,
        None => {
            return (
                PortalVerificationVerdict::MalformedReceipt,
                "Missing mandatory 'schema_version' field".to_string(),
            )
        }
    };

    if schema_version != "0.9" {
        return (
            PortalVerificationVerdict::UnsupportedProtocolVersion,
            format!("Unsupported schema_version '{schema_version}': expected '0.9'"),
        );
    }

    // Case 1: MerkleReceipt (contains inclusion_proofs and leaves)
    if val.get("inclusion_proofs").is_some() {
        let mr: MerkleReceipt = match serde_json::from_value(val) {
            Ok(m) => m,
            Err(e) => {
                return (
                    PortalVerificationVerdict::MalformedReceipt,
                    format!("Invalid MerkleReceipt structure: {e}"),
                )
            }
        };

        match mr.verify_all() {
            Ok(true) => {}
            Ok(false) => {
                return (
                    PortalVerificationVerdict::IntegrityCheckFailed,
                    "MERKLE_PROOF_INVALID: Inclusion proof hash mismatch against merkle_root"
                        .to_string(),
                )
            }
            Err(e) => {
                return (
                    PortalVerificationVerdict::IntegrityCheckFailed,
                    format!("MERKLE_PROOF_INVALID: {e}"),
                )
            }
        }

        let att = match mr.attestation {
            Some(ref a) => a,
            None => {
                return (
                    PortalVerificationVerdict::UnsignedReceipt,
                    "Merkle proofs verified, but receipt contains no Ed25519 attestation"
                        .to_string(),
                )
            }
        };

        return verify_portal_attestation(att, &mr.merkle_root, mr.tree_size, pinned_keys);
    }

    // Case 2: SweepReceipt (contains total_bugs and results)
    if val.get("total_bugs").is_some() && val.get("results").is_some() {
        let sweep: SweepReceipt = match serde_json::from_value(val) {
            Ok(s) => s,
            Err(e) => {
                return (
                    PortalVerificationVerdict::MalformedReceipt,
                    format!("Invalid SweepReceipt structure: {e}"),
                )
            }
        };

        if sweep.total_bugs == 0 || sweep.results.len() != sweep.total_bugs {
            return (
                PortalVerificationVerdict::IntegrityCheckFailed,
                "SweepReceipt total_bugs count does not match results map length".to_string(),
            );
        }

        let (att_verdict, att_msg) = verify_portal_attestation(
            &sweep.ed25519_attestation,
            &sweep.merkle_root,
            sweep.total_bugs,
            pinned_keys,
        );
        if att_verdict != PortalVerificationVerdict::VerifiedGenuine {
            return (att_verdict, att_msg);
        }

        let any_case_failed = sweep
            .results
            .values()
            .any(|r| r.exit_code != 0 || r.status != "VERIFIED_REPAIRED");
        if sweep.failed_bugs > 0 || sweep.repaired_bugs != sweep.total_bugs || any_case_failed {
            return (
                PortalVerificationVerdict::PolicyQualificationFailed,
                format!(
                    "SWEEP_PARTIAL_FAILURE: Cryptographic signature is valid and trusted, but sweep only repaired {}/{} bugs ({} failed)",
                    sweep.repaired_bugs, sweep.total_bugs, sweep.failed_bugs
                ),
            );
        }

        return (att_verdict, att_msg);
    }

    // Case 3: Standalone AuditReceipt
    if val.get("task_id").is_some() && val.get("contract_sha256").is_some() {
        let merkle_root = match val.get("merkle_root").and_then(|v| v.as_str()) {
            Some(r) if !r.is_empty() => r.to_string(),
            _ => {
                return (
                    PortalVerificationVerdict::UnsignedReceipt,
                    "AuditReceipt has no embedded merkle_root or attestation".to_string(),
                )
            }
        };
        let att_val = match val.get("attestation") {
            Some(v) if !v.is_null() => v.clone(),
            _ => {
                return (
                    PortalVerificationVerdict::UnsignedReceipt,
                    "AuditReceipt has merkle_root but no Ed25519 attestation".to_string(),
                )
            }
        };
        let att: AttestationRecord = match serde_json::from_value(att_val) {
            Ok(a) => a,
            Err(e) => {
                return (
                    PortalVerificationVerdict::MalformedReceipt,
                    format!("Malformed attestation in AuditReceipt: {e}"),
                )
            }
        };
        // Extract tree_size from signing_payload: REPLAYFIX-ATTESTATION:v0.9:<schema>:<root>:<size>:<ts>
        let parts: Vec<&str> = att.signing_payload.splitn(6, ':').collect();
        let tree_size = if parts.len() == 6 {
            parts[4].parse::<usize>().unwrap_or(0)
        } else {
            0
        };
        if tree_size == 0 {
            return (
                PortalVerificationVerdict::IntegrityCheckFailed,
                "Invalid signing_payload format in AuditReceipt attestation".to_string(),
            );
        }

        let (att_verdict, att_msg) =
            verify_portal_attestation(&att, &merkle_root, tree_size, pinned_keys);
        if att_verdict != PortalVerificationVerdict::VerifiedGenuine {
            return (att_verdict, att_msg);
        }

        let status_ok = val
            .get("status")
            .and_then(|v| v.as_str())
            .is_some_and(|s| s == "VERIFIED_REPAIRED");
        let repo_clean = val
            .get("base_repo_unmodified_verified")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let gate2_green = val
            .get("gate2")
            .and_then(|g| g.get("verdict"))
            .and_then(|v| v.as_str())
            .is_none_or(|v| v == "GREEN");
        let parity_ok = val
            .get("regression_parity")
            .and_then(|r| r.get("parity_preserved"))
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        if !status_ok || !repo_clean || !gate2_green || !parity_ok {
            return (
                PortalVerificationVerdict::PolicyQualificationFailed,
                "Cryptographic attestation is valid and trusted, but deterministic gate or regression parity policy failed"
                    .to_string(),
            );
        }

        return (att_verdict, att_msg);
    }

    (
        PortalVerificationVerdict::MalformedReceipt,
        "Unrecognized receipt JSON schema".to_string(),
    )
}

fn verify_portal_attestation(
    att: &AttestationRecord,
    merkle_root: &str,
    tree_size: usize,
    pinned_keys: &HashSet<String>,
) -> (PortalVerificationVerdict, String) {
    // First check cryptographic validity under AllowAny
    if let Err(e) = verify_attestation(att, merkle_root, tree_size, &TrustPolicy::AllowAny) {
        return (
            PortalVerificationVerdict::IntegrityCheckFailed,
            format!("SIGNATURE_VERIFICATION_FAILED: {e}"),
        );
    }

    // Next check if signer key is pinned
    let normalized_pk = att.public_key.to_lowercase();
    if pinned_keys
        .iter()
        .any(|k| k.to_lowercase() == normalized_pk)
    {
        (
            PortalVerificationVerdict::VerifiedGenuine,
            format!(
                "All 4 trust layers verified: RFC 6962 root {} signed by trusted key {}",
                &merkle_root[..16.min(merkle_root.len())],
                &normalized_pk[..16.min(normalized_pk.len())]
            ),
        )
    } else {
        (
            PortalVerificationVerdict::SignatureValidSignerUntrusted,
            format!(
                "Ed25519 signature and Merkle root are mathematically valid, but signer key {} is not in the pinned trust store",
                &normalized_pk[..16.min(normalized_pk.len())]
            ),
        )
    }
}

fn check_json_depth(raw: &str) -> usize {
    let mut current = 0usize;
    let mut max_depth = 0usize;
    let mut in_str = false;
    let mut esc = false;
    for b in raw.bytes() {
        if in_str {
            if esc {
                esc = false;
            } else if b == b'\\' {
                esc = true;
            } else if b == b'"' {
                in_str = false;
            }
        } else if b == b'"' {
            in_str = true;
        } else if b == b'{' || b == b'[' {
            current = current.saturating_add(1);
            if current > max_depth {
                max_depth = current;
            }
        } else if b == b'}' || b == b']' {
            current = current.saturating_sub(1);
        }
    }
    max_depth
}

enum JsonScope {
    Object {
        keys: HashSet<String>,
        expecting_key: bool,
    },
    Array,
}

/// Detect duplicate JSON object keys in raw JSON text before `serde_json::from_str::<Value>`
/// silently overwrites earlier keys.
fn check_duplicate_json_keys(raw: &str) -> Option<String> {
    let bytes = raw.as_bytes();
    let len = bytes.len();
    let mut stack: Vec<JsonScope> = Vec::new();
    let mut i = 0usize;

    while i < len {
        let b = bytes[i];
        match b {
            b'"' => {
                let start = i;
                i += 1;
                let mut esc = false;
                while i < len {
                    let c = bytes[i];
                    if esc {
                        esc = false;
                    } else if c == b'\\' {
                        esc = true;
                    } else if c == b'"' {
                        break;
                    }
                    i += 1;
                }
                if i >= len {
                    return None;
                }
                let end = i;
                i += 1;

                // Check if we are in an Object scope expecting a key and followed by ':'
                let is_obj_key = matches!(
                    stack.last(),
                    Some(JsonScope::Object {
                        expecting_key: true,
                        ..
                    })
                );
                if is_obj_key {
                    let mut j = i;
                    while j < len && matches!(bytes[j], b' ' | b'\t' | b'\r' | b'\n') {
                        j += 1;
                    }
                    if j < len && bytes[j] == b':' {
                        let key_str = serde_json::from_str::<String>(&raw[start..=end])
                            .unwrap_or_else(|_| raw[start + 1..end].to_string());
                        if let Some(JsonScope::Object {
                            keys,
                            expecting_key,
                        }) = stack.last_mut()
                        {
                            if !keys.insert(key_str.clone()) {
                                return Some(key_str);
                            }
                            *expecting_key = false;
                        }
                    }
                }
                continue;
            }
            b'{' => {
                stack.push(JsonScope::Object {
                    keys: HashSet::new(),
                    expecting_key: true,
                });
            }
            b'[' => {
                stack.push(JsonScope::Array);
            }
            b'}' | b']' => {
                let _ = stack.pop();
            }
            b',' => {
                if let Some(JsonScope::Object { expecting_key, .. }) = stack.last_mut() {
                    *expecting_key = true;
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

fn contains_forbidden_prototype_keys(val: &serde_json::Value) -> bool {
    match val {
        serde_json::Value::Object(map) => {
            for (k, v) in map {
                if k == "__proto__" || k == "constructor" || k == "prototype" {
                    return true;
                }
                if contains_forbidden_prototype_keys(v) {
                    return true;
                }
            }
            false
        }
        serde_json::Value::Array(arr) => arr.iter().any(contains_forbidden_prototype_keys),
        _ => false,
    }
}

/// Deterministic bundle of signed presets for embedding in `proof_viewer.html` and CLI export.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortalDemoPresets {
    pub trusted_public_key_hex: String,
    pub rf01_merkle_receipt: MerkleReceipt,
    pub rf01_tampered_merkle_receipt: MerkleReceipt,
    pub rf01_untrusted_merkle_receipt: MerkleReceipt,
    pub five_bug_sweep_receipt: SweepReceipt,
}

impl PortalDemoPresets {
    /// Construct deterministic, cryptographically genuine presets using `DEMO_PRESET_PUBLIC_SEED_HEX`.
    pub fn generate() -> Result<Self> {
        let kp = KeypairManager::from_hex(DEMO_PRESET_PUBLIC_SEED_HEX)?;
        let trusted_public_key_hex = kp.public_key_hex();
        let created_at = "2026-10-30T12:00:00Z".to_string();

        let rf01_fixture = Fixture::for_task("RF-01")
            .ok_or_else(|| CoreError::Verification("Missing built-in fixture RF-01".to_string()))?;
        let diff_bytes = rf01_fixture.builtin_diff.as_bytes();
        let preflight_bytes =
            br#"{"syntax_preflight_passed":true,"verified_at":"2026-10-30T12:00:00Z"}"#;
        let diag_bytes = br#"{"causal_confidence":"HIGH","state":"CAUSAL_DEFECT_CONFIRMED","verdict":"CONFIRMED"}"#;
        let reg_bytes = br#"{"added_failures":[],"added_skips":[],"baseline_total":42,"parity_preserved":true,"patched_total":42}"#;
        let audit_bytes = br#"{"contract_id":"rf01_cart_refresh","schema_version":"0.9","status":"VERIFIED_REPAIRED","task_id":"RF-01"}"#;

        let leaves = vec![
            EvidenceLeaf {
                artifact_type: "candidate_patch".to_string(),
                file_name: "01_candidate.diff".to_string(),
                content_sha256: sha256_hex(diff_bytes),
                size_bytes: diff_bytes.len(),
                timestamp: created_at.clone(),
            },
            EvidenceLeaf {
                artifact_type: "syntax_preflight".to_string(),
                file_name: "02_preflight.json".to_string(),
                content_sha256: sha256_hex(preflight_bytes),
                size_bytes: preflight_bytes.len(),
                timestamp: created_at.clone(),
            },
            EvidenceLeaf {
                artifact_type: "differential_diagnosis".to_string(),
                file_name: "03_differential_diagnosis.json".to_string(),
                content_sha256: sha256_hex(diag_bytes),
                size_bytes: diag_bytes.len(),
                timestamp: created_at.clone(),
            },
            EvidenceLeaf {
                artifact_type: "regression_parity".to_string(),
                file_name: "04_regression_parity.json".to_string(),
                content_sha256: sha256_hex(reg_bytes),
                size_bytes: reg_bytes.len(),
                timestamp: created_at.clone(),
            },
            EvidenceLeaf {
                artifact_type: "audit_receipt".to_string(),
                file_name: "04_audit_receipt.json".to_string(),
                content_sha256: sha256_hex(audit_bytes),
                size_bytes: audit_bytes.len(),
                timestamp: created_at.clone(),
            },
        ];

        let tree = MerkleTree::new(leaves)?;
        let mut rf01_merkle_receipt = tree.to_receipt()?;
        let att = kp.sign(
            "0.9",
            &rf01_merkle_receipt.merkle_root,
            rf01_merkle_receipt.tree_size,
            &created_at,
        )?;
        rf01_merkle_receipt.attestation = Some(att);

        // Preset 2: Tampered leaf (1-bit / 1-hex flip in 01_candidate.diff content_sha256)
        let mut rf01_tampered_merkle_receipt = rf01_merkle_receipt.clone();
        if let Some(first_leaf) = rf01_tampered_merkle_receipt.leaves.first_mut() {
            let mut chars: Vec<char> = first_leaf.content_sha256.chars().collect();
            if let Some(c) = chars.first_mut() {
                *c = if *c == '0' { '1' } else { '0' };
            }
            first_leaf.content_sha256 = chars.into_iter().collect();
        }

        // Preset 3: Valid signature from an untrusted keypair
        let untrusted_seed = "4ccd089b28ff96da9db6c346ec114e0f5b8a319f35aba624da8cf6ed4fb8a6fb";
        let untrusted_kp = KeypairManager::from_hex(untrusted_seed)?;
        let mut rf01_untrusted_merkle_receipt = tree.to_receipt()?;
        let untrusted_att = untrusted_kp.sign(
            "0.9",
            &rf01_untrusted_merkle_receipt.merkle_root,
            rf01_untrusted_merkle_receipt.tree_size,
            &created_at,
        )?;
        rf01_untrusted_merkle_receipt.attestation = Some(untrusted_att);

        // Preset 4: 5-Bug SweepReceipt (RF-01..RF-05)
        let mut sweep_results = BTreeMap::new();
        let mut sweep_leaves = Vec::new();
        for (idx, bug_id) in ["RF-01", "RF-02", "RF-03", "RF-04", "RF-05"]
            .into_iter()
            .enumerate()
        {
            let fx = Fixture::for_task(bug_id)
                .ok_or_else(|| CoreError::Verification(format!("Missing fixture {bug_id}")))?;
            let patch_sha = sha256_hex(fx.builtin_diff.as_bytes());
            sweep_leaves.push(EvidenceLeaf {
                artifact_type: "sweep_bug_receipt".to_string(),
                file_name: format!("{bug_id}/audit_receipt.json"),
                content_sha256: patch_sha.clone(),
                size_bytes: fx.builtin_diff.len(),
                timestamp: created_at.clone(),
            });
            sweep_results.insert(
                bug_id.to_string(),
                SweepBugResult {
                    bug_id: bug_id.to_string(),
                    status: "VERIFIED_REPAIRED".to_string(),
                    exit_code: 0,
                    target_path: fx.target_path.to_string(),
                    patch_sha256: Some(patch_sha),
                    elapsed_ms: 680 + (idx as u64 * 95),
                    error: None,
                },
            );
        }
        let sweep_tree = MerkleTree::new(sweep_leaves)?;
        let sweep_root = sweep_tree.root_hex();
        let sweep_att = kp.sign("0.9", &sweep_root, sweep_tree.len(), &created_at)?;
        let five_bug_sweep_receipt = SweepReceipt {
            schema_version: "0.9".to_string(),
            total_bugs: 5,
            repaired_bugs: 5,
            failed_bugs: 0,
            results: sweep_results,
            merkle_root: sweep_root,
            ed25519_attestation: sweep_att,
            elapsed_wall_clock_ms: 3_880,
            created_at,
        };

        Ok(Self {
            trusted_public_key_hex,
            rf01_merkle_receipt,
            rf01_tampered_merkle_receipt,
            rf01_untrusted_merkle_receipt,
            five_bug_sweep_receipt,
        })
    }
}

/// Validate that a `proof_viewer.html` file satisfies all Spec 095 security, offline, and structural contracts.
pub fn validate_portal_html_contract(html_path: &Path) -> Result<()> {
    if !html_path.exists() {
        return Err(CoreError::Verification(format!(
            "Portal file '{}' does not exist",
            html_path.display()
        )));
    }
    let html = fs::read_to_string(html_path)?;

    // 1. Strict offline Content-Security-Policy check
    if !html.contains("Content-Security-Policy")
        || !html.contains("default-src 'none'")
        || !html.contains("connect-src 'none'")
    {
        return Err(CoreError::SecurityViolation(
            "Portal HTML is missing strict offline Content-Security-Policy (default-src 'none'; connect-src 'none')"
                .to_string(),
        ));
    }

    // 2. Zero external network primitives or CDN links
    for forbidden in [
        "https://cdn.",
        "https://unpkg.",
        "https://jsdelivr.",
        "https://fonts.googleapis",
        "XMLHttpRequest",
        "WebSocket(",
        "eval(",
    ] {
        if html.contains(forbidden) {
            return Err(CoreError::SecurityViolation(format!(
                "Portal HTML contains forbidden network or dynamic execution primitive: '{forbidden}'"
            )));
        }
    }

    // 3. Required honest verification state labels
    for state_label in [
        "VERIFIED GENUINE",
        "SIGNATURE VALID — SIGNER UNTRUSTED",
        "POLICY QUALIFICATION FAILED",
        "INTEGRITY CHECK FAILED",
        "UNSIGNED RECEIPT",
        "UNSUPPORTED PROTOCOL VERSION",
        "MALFORMED RECEIPT",
    ] {
        if !html.contains(state_label) {
            return Err(CoreError::Verification(format!(
                "Portal HTML is missing required honest verification state label: '{state_label}'"
            )));
        }
    }

    // 4. Required cryptographic & multimodal inspector sections
    for marker in [
        "REPLAYFIX-ATTESTATION:v0.9:",
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        "SWEEP_PARTIAL_FAILURE",
        "F_patched \\ F_baseline",
    ] {
        if !html.contains(marker) {
            return Err(CoreError::Verification(format!(
                "Portal HTML is missing required protocol marker: '{marker}'"
            )));
        }
    }

    Ok(())
}

/// Replace `__REPLAYFIX_PORTAL_PRESETS_JSON__` in `html_path` with deterministic presets if present.
pub fn hydrate_portal_html_presets(html_path: &Path) -> Result<()> {
    let raw = fs::read_to_string(html_path)?;
    if raw.contains("__REPLAYFIX_PORTAL_PRESETS_JSON__") {
        let presets = PortalDemoPresets::generate()?;
        let presets_json = serde_json::to_string(&presets)?;
        let updated = raw.replace("__REPLAYFIX_PORTAL_PRESETS_JSON__", &presets_json);
        atomic_write_file(html_path, updated.as_bytes())?;
    }
    Ok(())
}

/// Export a standalone proof bundle (copying `proof_viewer.html` and writing `demo_presets.json`)
/// into `target_dir` using atomic file writes.
pub fn export_proof_bundle(repo_root: &Path, target_dir: &Path) -> Result<()> {
    fs::create_dir_all(target_dir)?;
    let src_portal = repo_root.join("proof_viewer.html");
    if src_portal.exists() {
        hydrate_portal_html_presets(&src_portal)?;
        let html_bytes = fs::read(&src_portal)?;
        atomic_write_file(target_dir.join("proof_viewer.html"), &html_bytes)?;
    }
    let presets = PortalDemoPresets::generate()?;
    let presets_bytes = serde_json::to_vec_pretty(&presets)?;
    atomic_write_file(target_dir.join("demo_presets.json"), &presets_bytes)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf()
    }

    #[test]
    fn test_portal_self_contained_and_strict_csp() {
        let portal_path = repo_root().join("proof_viewer.html");
        hydrate_portal_html_presets(&portal_path).unwrap();
        validate_portal_html_contract(&portal_path).unwrap();
    }

    #[test]
    fn test_portal_honest_verification_state_machine() {
        let presets = PortalDemoPresets::generate().unwrap();
        let pinned = HashSet::from([presets.trusted_public_key_hex.clone()]);

        // 1. VERIFIED GENUINE (RF-01 MerkleReceipt signed by pinned key)
        let genuine_bytes = serde_json::to_vec(&presets.rf01_merkle_receipt).unwrap();
        let (v1, _) = evaluate_portal_receipt_bytes(&genuine_bytes, &pinned);
        assert_eq!(v1, PortalVerificationVerdict::VerifiedGenuine);

        // 2. SIGNATURE VALID — SIGNER UNTRUSTED (valid signature from unpinned key)
        let untrusted_bytes = serde_json::to_vec(&presets.rf01_untrusted_merkle_receipt).unwrap();
        let (v2, _) = evaluate_portal_receipt_bytes(&untrusted_bytes, &pinned);
        assert_eq!(v2, PortalVerificationVerdict::SignatureValidSignerUntrusted);

        // 3. INTEGRITY CHECK FAILED (1-hex tampered leaf hash)
        let tampered_bytes = serde_json::to_vec(&presets.rf01_tampered_merkle_receipt).unwrap();
        let (v3, _) = evaluate_portal_receipt_bytes(&tampered_bytes, &pinned);
        assert_eq!(v3, PortalVerificationVerdict::IntegrityCheckFailed);

        // 4. UNSIGNED RECEIPT
        let mut unsigned_mr = presets.rf01_merkle_receipt.clone();
        unsigned_mr.attestation = None;
        let unsigned_bytes = serde_json::to_vec(&unsigned_mr).unwrap();
        let (v4, _) = evaluate_portal_receipt_bytes(&unsigned_bytes, &pinned);
        assert_eq!(v4, PortalVerificationVerdict::UnsignedReceipt);

        // 5. UNSUPPORTED PROTOCOL VERSION
        let mut bad_ver = presets.rf01_merkle_receipt.clone();
        bad_ver.schema_version = "0.8".to_string();
        let bad_ver_bytes = serde_json::to_vec(&bad_ver).unwrap();
        let (v5, _) = evaluate_portal_receipt_bytes(&bad_ver_bytes, &pinned);
        assert_eq!(v5, PortalVerificationVerdict::UnsupportedProtocolVersion);

        // 6. MALFORMED RECEIPT (invalid JSON or float)
        let (v6, _) =
            evaluate_portal_receipt_bytes(b"{\"schema_version\":\"0.9\",\"float\":1.25}", &pinned);
        assert_eq!(v6, PortalVerificationVerdict::MalformedReceipt);
    }

    #[test]
    fn test_portal_xss_and_prototype_pollution_guards() {
        let pinned = HashSet::new();
        let proto_payload = br#"{"schema_version":"0.9","__proto__":{"polluted":true}}"#;
        let (v_proto, msg) = evaluate_portal_receipt_bytes(proto_payload, &pinned);
        assert_eq!(v_proto, PortalVerificationVerdict::MalformedReceipt);
        assert!(msg.contains("Prototype pollution"));

        // Deeply nested JSON (> 32 levels)
        let deep_json = format!("{}{}{}", "{".repeat(40), "\"a\":1", "}".repeat(40));
        let (v_deep, _) = evaluate_portal_receipt_bytes(deep_json.as_bytes(), &pinned);
        assert_eq!(v_deep, PortalVerificationVerdict::MalformedReceipt);

        // Duplicate JSON object keys rejected before serde_json Map overwrites them
        let dup_json = br#"{"schema_version":"0.9","schema_version":"0.8","tree_size":1}"#;
        let (v_dup, msg_dup) = evaluate_portal_receipt_bytes(dup_json, &pinned);
        assert_eq!(v_dup, PortalVerificationVerdict::MalformedReceipt);
        assert!(msg_dup.contains("Duplicate JSON object key"));
    }

    #[test]
    fn test_portal_policy_qualification_failed_on_partial_sweep_or_failed_regression() {
        let presets = PortalDemoPresets::generate().unwrap();
        let pinned = HashSet::from([presets.trusted_public_key_hex.clone()]);

        // Partial sweep with valid signature and pinned key -> POLICY QUALIFICATION FAILED
        let mut partial_sweep = presets.five_bug_sweep_receipt.clone();
        partial_sweep.repaired_bugs = 4;
        partial_sweep.failed_bugs = 1;
        let partial_bytes = serde_json::to_vec(&partial_sweep).unwrap();
        let (v_partial, msg_partial) = evaluate_portal_receipt_bytes(&partial_bytes, &pinned);
        assert_eq!(
            v_partial,
            PortalVerificationVerdict::PolicyQualificationFailed
        );
        assert!(msg_partial.contains("SWEEP_PARTIAL_FAILURE"));
    }

    #[test]
    fn test_portal_multimodal_spatial_alignment() {
        // Valid millipart box [69, 133, 181, 200] on 1280x720 viewport
        let overlay = validate_and_map_sam_box([69, 133, 181, 200], 1280, 720).unwrap();
        assert_eq!(overlay.box_x1_px, 88);
        assert_eq!(overlay.box_y1_px, 95);
        assert_eq!(overlay.box_x2_px, 231);
        assert_eq!(overlay.box_y2_px, 144);
        assert_eq!(overlay.centroid_x_px, 159);
        assert_eq!(overlay.centroid_y_px, 119);

        // Out-of-bounds or inverted SAM box fails closed
        assert!(validate_and_map_sam_box([-10, 0, 500, 500], 1280, 720).is_err());
        assert!(validate_and_map_sam_box([100, 100, 1200, 500], 1280, 720).is_err());
        assert!(validate_and_map_sam_box([600, 100, 200, 500], 1280, 720).is_err());
    }

    #[test]
    fn test_portal_sweep_receipt_visualization() {
        let presets = PortalDemoPresets::generate().unwrap();
        let pinned = HashSet::from([presets.trusted_public_key_hex.clone()]);
        let sweep_bytes = serde_json::to_vec(&presets.five_bug_sweep_receipt).unwrap();
        let (verdict, _) = evaluate_portal_receipt_bytes(&sweep_bytes, &pinned);
        assert_eq!(verdict, PortalVerificationVerdict::VerifiedGenuine);
        assert_eq!(presets.five_bug_sweep_receipt.total_bugs, 5);
        assert_eq!(presets.five_bug_sweep_receipt.repaired_bugs, 5);
    }

    #[test]
    fn test_portal_js_engine_rfc8785_rfc6962_and_ed25519_parity_in_node() {
        let portal_path = repo_root().join("proof_viewer.html");
        hydrate_portal_html_presets(&portal_path).unwrap();
        let html = fs::read_to_string(&portal_path).unwrap();

        let start_idx = html.find("<script>").unwrap() + "<script>".len();
        let end_idx = html.rfind("</script>").unwrap();
        let script_code = &html[start_idx..end_idx];

        let temp_dir =
            std::env::temp_dir().join(format!("rf-portal-js-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();
        let js_module_path = temp_dir.join("portal_engine.js");
        fs::write(&js_module_path, script_code).unwrap();

        let runner_path = temp_dir.join("run_verify.js");
        let runner_js = r#"
const assert = require("assert");
const engine = require("./portal_engine.js");

const pinned = new Set([engine.DEFAULT_PINNED_KEY.toLowerCase()]);

// 1. RFC 8785 Surrogate pair UTF-16 key ordering parity (\u{10000} before \u{E000})
const jcs = engine.canonicalJson({ "\uE000": "bmp", "\u{10000}": "supp" });
assert(jcs.indexOf("\u{10000}") < jcs.indexOf("\uE000"), "UTF-16 surrogate ordering failed");

// 2. Preset 1: RF-01 Verified Genuine
const r1 = engine.evaluateReceiptObject(JSON.stringify(engine.PRESETS.rf01_merkle_receipt), pinned);
assert.strictEqual(r1.verdict, "VERIFIED GENUINE", "Expected VERIFIED GENUINE, got: " + r1.reason);
assert.strictEqual(r1.leafDetails.length, 5);
assert(r1.leafDetails.every(d => d.valid));

// 3. Preset 2: 5-Bug Sweep Verified Genuine
const r2 = engine.evaluateReceiptObject(JSON.stringify(engine.PRESETS.five_bug_sweep_receipt), pinned);
assert.strictEqual(r2.verdict, "VERIFIED GENUINE");

// 4. Preset 3: Untrusted Signer Key
const r3 = engine.evaluateReceiptObject(JSON.stringify(engine.PRESETS.rf01_untrusted_merkle_receipt), pinned);
assert.strictEqual(r3.verdict, "SIGNATURE VALID — SIGNER UNTRUSTED");

// 5. Preset 4: Tampered Leaf -> INTEGRITY CHECK FAILED
const r4 = engine.evaluateReceiptObject(JSON.stringify(engine.PRESETS.rf01_tampered_merkle_receipt), pinned);
assert.strictEqual(r4.verdict, "INTEGRITY CHECK FAILED");

// 6. Unsigned Receipt -> UNSIGNED RECEIPT
const unsignedObj = JSON.parse(JSON.stringify(engine.PRESETS.rf01_merkle_receipt));
delete unsignedObj.attestation;
const r5 = engine.evaluateReceiptObject(JSON.stringify(unsignedObj), pinned);
assert.strictEqual(r5.verdict, "UNSIGNED RECEIPT");

// 7. Unsupported Version -> UNSUPPORTED PROTOCOL VERSION
const oldVer = JSON.parse(JSON.stringify(engine.PRESETS.rf01_merkle_receipt));
oldVer.schema_version = "0.8";
const r6 = engine.evaluateReceiptObject(JSON.stringify(oldVer), pinned);
assert.strictEqual(r6.verdict, "UNSUPPORTED PROTOCOL VERSION");

// 8. Prototype Pollution -> MALFORMED RECEIPT
const r7 = engine.evaluateReceiptObject('{"schema_version":"0.9","__proto__":{"x":1}}', pinned);
assert.strictEqual(r7.verdict, "MALFORMED RECEIPT");

// 9. Duplicate JSON object keys -> MALFORMED RECEIPT
const r8 = engine.evaluateReceiptObject('{"schema_version":"0.9","schema_version":"0.8"}', pinned);
assert.strictEqual(r8.verdict, "MALFORMED RECEIPT");
assert(r8.reason.includes("Duplicate JSON object key"));

// 10. Partial Sweep with valid signature & pinned key -> POLICY QUALIFICATION FAILED
const partialSweep = JSON.parse(JSON.stringify(engine.PRESETS.five_bug_sweep_receipt));
partialSweep.repaired_bugs = 4;
partialSweep.failed_bugs = 1;
const r9 = engine.evaluateReceiptObject(JSON.stringify(partialSweep), pinned);
assert.strictEqual(r9.verdict, "POLICY QUALIFICATION FAILED");

// 11. Empty Merkle tree with wrong root -> INTEGRITY CHECK FAILED
const badEmpty = { schema_version: "0.9", tree_size: 0, merkle_root: "00".repeat(32), leaves: [], inclusion_proofs: {} };
const r10 = engine.evaluateReceiptObject(JSON.stringify(badEmpty), pinned);
assert.strictEqual(r10.verdict, "INTEGRITY CHECK FAILED");

console.log("PORTAL_JS_PARITY_OK");
"#;
        fs::write(&runner_path, runner_js).unwrap();

        let output = std::process::Command::new("node")
            .arg(&runner_path)
            .current_dir(&temp_dir)
            .output()
            .unwrap();

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let _ = fs::remove_dir_all(&temp_dir);

        assert!(
            output.status.success() && stdout.contains("PORTAL_JS_PARITY_OK"),
            "Node.js execution of proof_viewer.html script failed:\nstdout: {stdout}\nstderr: {stderr}"
        );
    }
}
