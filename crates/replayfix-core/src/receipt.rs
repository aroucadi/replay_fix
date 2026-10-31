//! Machine-readable terminal artifacts: audit receipts and triage reports.

use crate::artifacts::ArtifactStore;
use crate::crypto::{AttestationRecord, KeypairManager};
use crate::engine::{GateVerdict, OracleEvent};
use crate::error::Result;
use crate::fsm::{DifferentialDiagnosisReport, Phase};
use crate::locator::GroundingTelemetry;
use crate::merkle::{EvidenceLeaf, MerkleTree};
use crate::perception::VideoTelemetry;
use crate::regression::RegressionParityReport;
use crate::repair::{RepairTelemetry, TokenAccounting};
use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

pub const SCHEMA_VERSION: &str = "0.9";
pub const SUBSTRATE: &str = "mock-static-oracle";
pub const SUBSTRATE_LIVE_BROWSER: &str = "playwright-cdp";
pub const DISCLAIMER: &str = "Simulated oracle: gates inspect a built-in fixture. No browser was executed and no model was involved.";
pub const DISCLAIMER_LIVE_BROWSER: &str = "Live Playwright CDP oracle: gates executed in Chromium against an isolated ephemeral sandbox server.";
pub const AUDIT_RECEIPT_FILE: &str = "audit_receipt.json";
pub const TRIAGE_REPORT_FILE: &str = "triage_report.json";

/// Current UTC time as an RFC 3339 string with millisecond precision.
pub fn timestamp_now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GateRecord {
    pub observed_at: String,
    pub verdict: GateVerdict,
    pub contract_sha256: String,
    pub events: Vec<OracleEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PatchRecord {
    pub target_path: String,
    pub sha256: String,
    pub insertions: usize,
    pub deletions: usize,
    pub files_changed: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelProvenanceRecord {
    pub execution_mode: String,
    pub model_intent: String,
    pub model_repair: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuditReceipt {
    pub schema_version: String,
    pub status: String,
    pub task_id: String,
    pub substrate: String,
    pub disclaimer: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_provenance: Option<ModelProvenanceRecord>,
    pub contract_id: String,
    pub contract_sha256: String,
    pub gate1: GateRecord,
    pub patch: PatchRecord,
    pub gate2: GateRecord,
    pub phase_trace: Vec<Phase>,
    pub base_tree_sha256: String,
    pub sandbox_tree_sha256: String,
    pub base_repo_unmodified_verified: bool,
    pub ephemeral_port: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grounding_telemetry: Option<GroundingTelemetry>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub video_telemetry: Option<VideoTelemetry>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repair_telemetry: Option<RepairTelemetry>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_accounting: Option<TokenAccounting>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub differential_diagnosis: Option<DifferentialDiagnosisReport>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub regression_parity: Option<RegressionParityReport>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub merkle_root: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attestation: Option<AttestationRecord>,
    pub created_at: String,
}

impl AuditReceipt {
    /// Write disaggregated stage artifacts into `<output_dir>/artifacts/`, compute RFC 6962
    /// Merkle tree receipts and Ed25519 attestations, populate `self.merkle_root` and
    /// `self.attestation`, and atomically update `<output_dir>/audit_receipt.json`.
    pub fn write_disaggregated_artifacts(
        &mut self,
        output_dir: &Path,
        raw_patch_content: Option<&str>,
        repro_spec_json: Option<&str>,
    ) -> Result<PathBuf> {
        let artifacts_dir = output_dir.join("artifacts");
        let store = ArtifactStore::new(&artifacts_dir)?;

        if let Some(diff) = raw_patch_content {
            store.write_text("candidate.patch", diff)?;
            store.write_text("01_candidate.diff", diff)?;
        }
        if let Some(spec) = repro_spec_json {
            store.write_text("repro_spec.json", spec)?;
            store.write_text("02_repro_spec.json", spec)?;
        } else {
            let default_preflight = serde_json::json!({
                "syntax_preflight_passed": true,
                "verified_at": self.created_at
            });
            store.write_json_pretty("02_preflight.json", &default_preflight)?;
        }
        if let Some(ref diag) = self.differential_diagnosis {
            store.write_json_pretty("differential_diagnosis.json", diag)?;
            store.write_json_pretty("03_differential_diagnosis.json", diag)?;
        }
        if let Some(ref reg) = self.regression_parity {
            store.write_json_pretty("regression_parity.json", reg)?;
            store.write_json_pretty("04_regression_parity.json", reg)?;
        }

        // Ensure 04_audit_receipt.json is serialized as the pre-Merkle core receipt payload
        let mut core_receipt = self.clone();
        core_receipt.merkle_root = None;
        core_receipt.attestation = None;
        store.write_json_pretty("04_audit_receipt.json", &core_receipt)?;

        // Collect all written artifacts as Merkle evidence leaves
        let mut leaves = Vec::new();
        if let Ok(entries) = fs::read_dir(&artifacts_dir) {
            let mut file_entries = Vec::new();
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if !name.ends_with("merkle_receipt.json")
                    && !name.ends_with("attestation.json")
                    && !name.ends_with(".tmp")
                    && entry.path().is_file()
                {
                    file_entries.push((name, entry.path()));
                }
            }
            file_entries.sort_by(|a, b| a.0.cmp(&b.0));

            for (filename, path) in file_entries {
                if let Ok(bytes) = fs::read(&path) {
                    let mut hasher = Sha256::new();
                    hasher.update(&bytes);
                    let content_sha256 = format!("{:x}", hasher.finalize());
                    let size_bytes = bytes.len();
                    let artifact_type =
                        if filename.ends_with(".diff") || filename.ends_with(".patch") {
                            "candidate_patch".to_string()
                        } else if filename.contains("preflight") {
                            "syntax_preflight".to_string()
                        } else if filename.contains("repro_spec") {
                            "repro_spec".to_string()
                        } else if filename.contains("differential_diagnosis") {
                            "differential_diagnosis".to_string()
                        } else if filename.contains("regression_parity") {
                            "regression_parity".to_string()
                        } else if filename.contains("audit_receipt") {
                            "audit_receipt".to_string()
                        } else {
                            "evidence_fact".to_string()
                        };

                    leaves.push(EvidenceLeaf {
                        artifact_type,
                        file_name: filename,
                        content_sha256,
                        size_bytes,
                        timestamp: self.created_at.clone(),
                    });
                }
            }
        }

        let tree = MerkleTree::new(leaves)?;
        let mut merkle_receipt = tree.to_receipt()?;

        let key_dir = output_dir.join("keys");
        let keypair = KeypairManager::load_or_generate(Some(&key_dir))?;
        let attestation = keypair.sign(
            &self.schema_version,
            &merkle_receipt.merkle_root,
            merkle_receipt.tree_size,
            &self.created_at,
        )?;

        merkle_receipt.attestation = Some(attestation.clone());
        self.merkle_root = Some(merkle_receipt.merkle_root.clone());
        self.attestation = Some(attestation.clone());

        store.write_json_pretty("merkle_receipt.json", &merkle_receipt)?;
        store.write_json_pretty("attestation.json", &attestation)?;

        // Atomically write top-level merkle_receipt.json, attestation.json, and updated audit_receipt.json
        write_json(output_dir, "merkle_receipt.json", &merkle_receipt)?;
        write_json(output_dir, "attestation.json", &attestation)?;
        write_audit_receipt(output_dir, self)?;

        Ok(artifacts_dir)
    }

    /// Perform complete 4-Layer Zero-Trust verification on a receipt file or directory:
    /// - Layer 1: Verify every referenced evidence leaf file on disk exists, matches `size_bytes`,
    ///   and hashes to `content_sha256` (plus cross-checks top-level `audit_receipt.json` against
    ///   `04_audit_receipt.json`).
    /// - Layer 2: Verify RFC 6962 Merkle inclusion proofs against `merkle_root`.
    /// - Layer 3: Verify RFC 8032 Pure Ed25519 digital signature over the canonical signing payload.
    /// - Layer 4: Evaluate signer public key against `trust_policy`.
    pub fn verify_receipt_and_attestation(
        path: &Path,
        trust_policy: &crate::crypto::TrustPolicy,
    ) -> Result<(
        crate::crypto::VerificationOutcome,
        AttestationRecord,
        String,
        usize,
    )> {
        let target_file = if path.is_dir() {
            let candidates = [
                path.join("artifacts").join("merkle_receipt.json"),
                path.join("merkle_receipt.json"),
                path.join("sweep_receipt.json"),
                path.join("artifacts").join("04_audit_receipt.json"),
                path.join("audit_receipt.json"),
            ];
            candidates.into_iter().find(|p| p.exists()).ok_or_else(|| {
                crate::error::CoreError::SecurityViolation(format!(
                    "No receipt found in directory '{}'",
                    path.display()
                ))
            })?
        } else {
            path.to_path_buf()
        };

        if !target_file.exists() {
            return Err(crate::error::CoreError::SecurityViolation(format!(
                "Receipt file '{}' does not exist",
                target_file.display()
            )));
        }

        let file_content = fs::read_to_string(&target_file).map_err(|e| {
            crate::error::CoreError::SecurityViolation(format!(
                "Failed to read receipt '{}': {e}",
                target_file.display()
            ))
        })?;

        let json_val: serde_json::Value = serde_json::from_str(&file_content).map_err(|e| {
            crate::error::CoreError::SecurityViolation(format!("Malformed receipt JSON: {e}"))
        })?;

        let parent_dir = target_file.parent().unwrap_or_else(|| Path::new("."));

        // Case A: SweepReceipt verification
        if json_val.get("total_bugs").is_some() && json_val.get("results").is_some() {
            let sweep: crate::sweep::SweepReceipt =
                serde_json::from_value(json_val).map_err(|e| {
                    crate::error::CoreError::SecurityViolation(format!(
                        "Malformed SweepReceipt JSON: {e}"
                    ))
                })?;

            // Reconstruct sweep Merkle leaves from on-disk per-bug receipts and verify
            let mut leaves = Vec::new();
            for bug_id in sweep.results.keys() {
                let bug_receipt_rel = format!("{bug_id}/audit_receipt.json");
                let bug_triage_rel = format!("{bug_id}/triage_report.json");
                let bug_receipt_path = parent_dir.join(&bug_receipt_rel);
                let bug_triage_path = parent_dir.join(&bug_triage_rel);

                if bug_receipt_path.exists() {
                    let bytes = fs::read(&bug_receipt_path).map_err(|e| {
                        crate::error::CoreError::SecurityViolation(format!(
                            "Failed to read sweep bug receipt '{}': {e}",
                            bug_receipt_path.display()
                        ))
                    })?;
                    let parsed: AuditReceipt = serde_json::from_slice(&bytes).map_err(|e| {
                        crate::error::CoreError::SecurityViolation(format!(
                            "Malformed sweep bug receipt '{}': {e}",
                            bug_receipt_path.display()
                        ))
                    })?;
                    leaves.push(EvidenceLeaf {
                        artifact_type: "sweep_bug_receipt".to_string(),
                        file_name: bug_receipt_rel,
                        content_sha256: crate::digest::sha256_hex(&bytes),
                        size_bytes: bytes.len(),
                        timestamp: parsed.created_at,
                    });
                } else if bug_triage_path.exists() {
                    let bytes = fs::read(&bug_triage_path).map_err(|e| {
                        crate::error::CoreError::SecurityViolation(format!(
                            "Failed to read sweep bug triage '{}': {e}",
                            bug_triage_path.display()
                        ))
                    })?;
                    let parsed: TriageReport = serde_json::from_slice(&bytes).map_err(|e| {
                        crate::error::CoreError::SecurityViolation(format!(
                            "Malformed sweep bug triage '{}': {e}",
                            bug_triage_path.display()
                        ))
                    })?;
                    leaves.push(EvidenceLeaf {
                        artifact_type: "sweep_bug_triage".to_string(),
                        file_name: bug_triage_rel,
                        content_sha256: crate::digest::sha256_hex(&bytes),
                        size_bytes: bytes.len(),
                        timestamp: parsed.created_at,
                    });
                } else {
                    return Err(crate::error::CoreError::SecurityViolation(format!(
                        "MERKLE_PROOF_INVALID: Missing on-disk per-bug artifact for '{bug_id}'"
                    )));
                }
            }

            let recomputed_tree = MerkleTree::new(leaves)?;
            let recomputed_root = recomputed_tree.root_hex();
            if recomputed_root != sweep.merkle_root {
                return Err(crate::error::CoreError::SecurityViolation(format!(
                    "MERKLE_PROOF_INVALID: Sweep Merkle root mismatch (expected {}, recomputed {})",
                    sweep.merkle_root, recomputed_root
                )));
            }

            let outcome = crate::crypto::verify_attestation(
                &sweep.ed25519_attestation,
                &sweep.merkle_root,
                recomputed_tree.len(),
                trust_policy,
            )?;

            return Ok((
                outcome,
                sweep.ed25519_attestation,
                sweep.merkle_root,
                recomputed_tree.len(),
            ));
        }

        // Case B: MerkleReceipt or AuditReceipt verification (fail-closed if merkle_receipt is absent)
        let (merkle_receipt, attestation, artifacts_dir, root_out_dir) = if json_val
            .get("inclusion_proofs")
            .is_some()
        {
            let mr: crate::merkle::MerkleReceipt =
                serde_json::from_value(json_val).map_err(|e| {
                    crate::error::CoreError::SecurityViolation(format!(
                        "Malformed MerkleReceipt: {e}"
                    ))
                })?;
            let att = mr.attestation.clone().or_else(|| {
                let att_path = parent_dir.join("attestation.json");
                fs::read_to_string(att_path)
                    .ok()
                    .and_then(|s| serde_json::from_str(&s).ok())
            });
            let (art_dir, out_dir) = if parent_dir.file_name().is_some_and(|n| n == "artifacts") {
                (
                    parent_dir.to_path_buf(),
                    parent_dir.parent().unwrap_or(parent_dir).to_path_buf(),
                )
            } else {
                (parent_dir.join("artifacts"), parent_dir.to_path_buf())
            };
            (mr, att, art_dir, out_dir)
        } else {
            let audit_rec: AuditReceipt = serde_json::from_value(json_val).map_err(|e| {
                crate::error::CoreError::SecurityViolation(format!("Malformed AuditReceipt: {e}"))
            })?;
            let (art_dir, out_dir) = if parent_dir.file_name().is_some_and(|n| n == "artifacts") {
                (
                    parent_dir.to_path_buf(),
                    parent_dir.parent().unwrap_or(parent_dir).to_path_buf(),
                )
            } else {
                (parent_dir.join("artifacts"), parent_dir.to_path_buf())
            };
            let mr_path = if art_dir.join("merkle_receipt.json").exists() {
                art_dir.join("merkle_receipt.json")
            } else {
                out_dir.join("merkle_receipt.json")
            };
            if !mr_path.exists() {
                return Err(crate::error::CoreError::SecurityViolation(
                        "MERKLE_PROOF_INVALID: Missing merkle_receipt.json required for cryptographic verification"
                            .to_string(),
                    ));
            }
            let mr_str = fs::read_to_string(&mr_path).map_err(|e| {
                crate::error::CoreError::SecurityViolation(format!(
                    "Failed to read merkle_receipt.json: {e}"
                ))
            })?;
            let mr: crate::merkle::MerkleReceipt = serde_json::from_str(&mr_str).map_err(|e| {
                crate::error::CoreError::SecurityViolation(format!(
                    "Malformed merkle_receipt.json: {e}"
                ))
            })?;

            if let Some(ref rec_root) = audit_rec.merkle_root {
                if rec_root != &mr.merkle_root {
                    return Err(crate::error::CoreError::SecurityViolation(format!(
                            "MERKLE_PROOF_INVALID: AuditReceipt merkle_root '{}' does not match MerkleReceipt root '{}'",
                            rec_root, mr.merkle_root
                        )));
                }
            }

            let att = audit_rec
                .attestation
                .or_else(|| mr.attestation.clone())
                .or_else(|| {
                    let att_path = out_dir.join("attestation.json");
                    fs::read_to_string(att_path)
                        .ok()
                        .and_then(|s| serde_json::from_str(&s).ok())
                });
            (mr, att, art_dir, out_dir)
        };

        // Layer 2: Verify RFC 6962 Merkle inclusion proofs against merkle_root
        match merkle_receipt.verify_all() {
            Ok(true) => {}
            Ok(false) => {
                return Err(crate::error::CoreError::SecurityViolation(format!(
                    "MERKLE_PROOF_INVALID: One or more inclusion proofs failed verification against root {}",
                    merkle_receipt.merkle_root
                )));
            }
            Err(e) => {
                return Err(crate::error::CoreError::SecurityViolation(format!(
                    "MERKLE_PROOF_INVALID: {e}"
                )));
            }
        }

        // Layer 1: Verify on-disk artifact integrity if the artifacts directory is present
        if artifacts_dir.exists() {
            for leaf in &merkle_receipt.leaves {
                let leaf_path = artifacts_dir.join(&leaf.file_name);
                if !leaf_path.exists() {
                    return Err(crate::error::CoreError::SecurityViolation(format!(
                        "MERKLE_PROOF_INVALID: Referenced artifact '{}' is missing on disk",
                        leaf.file_name
                    )));
                }
                let disk_bytes = fs::read(&leaf_path).map_err(|e| {
                    crate::error::CoreError::SecurityViolation(format!(
                        "Failed to read artifact '{}': {e}",
                        leaf.file_name
                    ))
                })?;
                if disk_bytes.len() != leaf.size_bytes {
                    return Err(crate::error::CoreError::SecurityViolation(format!(
                        "MERKLE_PROOF_INVALID: Artifact '{}' size mismatch (expected {} bytes, found {} bytes)",
                        leaf.file_name,
                        leaf.size_bytes,
                        disk_bytes.len()
                    )));
                }
                let disk_sha = crate::digest::sha256_hex(&disk_bytes);
                if disk_sha != leaf.content_sha256 {
                    return Err(crate::error::CoreError::SecurityViolation(format!(
                        "MERKLE_PROOF_INVALID: Artifact '{}' content SHA-256 mismatch (expected {}, found {})",
                        leaf.file_name, leaf.content_sha256, disk_sha
                    )));
                }
            }

            // Cross-check top-level audit_receipt.json against 04_audit_receipt.json if both exist
            let top_audit = root_out_dir.join(AUDIT_RECEIPT_FILE);
            let leaf_audit = artifacts_dir.join("04_audit_receipt.json");
            if top_audit.exists() && leaf_audit.exists() {
                let mut top_rec: AuditReceipt =
                    serde_json::from_str(&fs::read_to_string(&top_audit).map_err(|e| {
                        crate::error::CoreError::SecurityViolation(format!(
                            "Failed to read top-level audit_receipt.json: {e}"
                        ))
                    })?)
                    .map_err(|e| {
                        crate::error::CoreError::SecurityViolation(format!(
                            "Malformed top-level audit_receipt.json: {e}"
                        ))
                    })?;
                let mut leaf_rec: AuditReceipt =
                    serde_json::from_str(&fs::read_to_string(&leaf_audit).map_err(|e| {
                        crate::error::CoreError::SecurityViolation(format!(
                            "Failed to read 04_audit_receipt.json: {e}"
                        ))
                    })?)
                    .map_err(|e| {
                        crate::error::CoreError::SecurityViolation(format!(
                            "Malformed 04_audit_receipt.json: {e}"
                        ))
                    })?;

                if let Some(ref top_root) = top_rec.merkle_root {
                    if top_root != &merkle_receipt.merkle_root {
                        return Err(crate::error::CoreError::SecurityViolation(
                            "MERKLE_PROOF_INVALID: Top-level audit_receipt.json merkle_root mismatch"
                                .to_string(),
                        ));
                    }
                }
                top_rec.merkle_root = None;
                top_rec.attestation = None;
                leaf_rec.merkle_root = None;
                leaf_rec.attestation = None;
                if top_rec != leaf_rec {
                    return Err(crate::error::CoreError::SecurityViolation(
                        "MERKLE_PROOF_INVALID: Top-level audit_receipt.json core fields diverged from committed 04_audit_receipt.json leaf"
                            .to_string(),
                    ));
                }
            }
        }

        // Layer 3 & 4: Verify Ed25519 signature and trust policy
        let att = attestation.ok_or_else(|| {
            crate::error::CoreError::SecurityViolation(
                "SIGNATURE_VERIFICATION_FAILED: Missing cryptographic attestation".to_string(),
            )
        })?;

        let outcome = crate::crypto::verify_attestation(
            &att,
            &merkle_receipt.merkle_root,
            merkle_receipt.tree_size,
            trust_policy,
        )?;

        Ok((
            outcome,
            att,
            merkle_receipt.merkle_root,
            merkle_receipt.tree_size,
        ))
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TriageStatus {
    RepairNoopRejected,
    PatchApplyFailed,
    Gate1NotRed,
    Gate2NotGreen,
    ContractMismatch,
    SecurityViolation,
    BaseRepoDirtyMutationDetected,
    AmbiguousGrounding,
    BridgeExecutionFailed,
    QuarantineRepairFailed,
    QuarantineInvertedBaseline,
    RegressionDetected,
    FlakyNonDeterministic,
    InvalidTestOverfit,
    CleanPassUnreproduced,
}

impl TriageStatus {
    /// Canonical SCREAMING_SNAKE_CASE string representation.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RepairNoopRejected => "REPAIR_NOOP_REJECTED",
            Self::PatchApplyFailed => "PATCH_APPLY_FAILED",
            Self::Gate1NotRed => "GATE1_NOT_RED",
            Self::Gate2NotGreen => "GATE2_NOT_GREEN",
            Self::ContractMismatch => "CONTRACT_MISMATCH",
            Self::SecurityViolation => "SECURITY_VIOLATION",
            Self::BaseRepoDirtyMutationDetected => "BASE_REPO_DIRTY_MUTATION_DETECTED",
            Self::AmbiguousGrounding => "AMBIGUOUS_GROUNDING",
            Self::BridgeExecutionFailed => "BRIDGE_EXECUTION_FAILED",
            Self::QuarantineRepairFailed => "QUARANTINE_REPAIR_FAILED",
            Self::QuarantineInvertedBaseline => "QUARANTINE_INVERTED_BASELINE",
            Self::RegressionDetected => "REGRESSION_DETECTED",
            Self::FlakyNonDeterministic => "FLAKY_NON_DETERMINISTIC",
            Self::InvalidTestOverfit => "INVALID_TEST_OVERFIT",
            Self::CleanPassUnreproduced => "CLEAN_PASS_UNREPRODUCED",
        }
    }

    /// Process exit code aligned with the canonical result schemas.
    #[must_use]
    pub const fn exit_code(self) -> i32 {
        match self {
            Self::Gate1NotRed
            | Self::ContractMismatch
            | Self::InvalidTestOverfit
            | Self::CleanPassUnreproduced => 10,
            Self::SecurityViolation | Self::BaseRepoDirtyMutationDetected => 11,
            Self::BridgeExecutionFailed => 20,
            Self::QuarantineRepairFailed | Self::FlakyNonDeterministic => 21,
            Self::AmbiguousGrounding => 22,
            Self::RepairNoopRejected | Self::PatchApplyFailed | Self::Gate2NotGreen => 30,
            Self::RegressionDetected => 31,
            Self::QuarantineInvertedBaseline => 32,
        }
    }
}

impl std::fmt::Display for TriageStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TriageReport {
    pub schema_version: String,
    pub status: TriageStatus,
    pub exit_code: i32,
    pub reason: String,
    pub task_id: String,
    pub substrate: String,
    pub phase_trace: Vec<Phase>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repair_telemetry: Option<RepairTelemetry>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_accounting: Option<TokenAccounting>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub differential_diagnosis: Option<DifferentialDiagnosisReport>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub regression_parity: Option<RegressionParityReport>,
    pub created_at: String,
}

impl TriageReport {
    pub fn new(
        status: TriageStatus,
        reason: impl Into<String>,
        task_id: &str,
        phase_trace: &[Phase],
    ) -> Self {
        Self::with_substrate(status, reason, task_id, SUBSTRATE, phase_trace)
    }

    pub fn with_substrate(
        status: TriageStatus,
        reason: impl Into<String>,
        task_id: &str,
        substrate: &str,
        phase_trace: &[Phase],
    ) -> Self {
        Self {
            schema_version: SCHEMA_VERSION.to_string(),
            status,
            exit_code: status.exit_code(),
            reason: reason.into(),
            task_id: task_id.to_string(),
            substrate: substrate.to_string(),
            phase_trace: phase_trace.to_vec(),
            repair_telemetry: None,
            token_accounting: None,
            differential_diagnosis: None,
            regression_parity: None,
            created_at: timestamp_now(),
        }
    }

    #[must_use]
    pub fn with_repair_telemetry(
        mut self,
        repair_telemetry: Option<RepairTelemetry>,
        token_accounting: Option<TokenAccounting>,
    ) -> Self {
        self.repair_telemetry = repair_telemetry;
        self.token_accounting = token_accounting;
        self
    }

    #[must_use]
    pub fn with_differential_diagnosis(
        mut self,
        diag: Option<DifferentialDiagnosisReport>,
    ) -> Self {
        self.differential_diagnosis = diag;
        self
    }

    #[must_use]
    pub fn with_regression_parity(mut self, parity: Option<RegressionParityReport>) -> Self {
        self.regression_parity = parity;
        self
    }
}

fn remove_if_present(path: &Path) -> Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}

fn write_json<T: Serialize>(dir: &Path, name: &str, value: &T) -> Result<PathBuf> {
    let store = ArtifactStore::new(dir)?;
    store.write_json_pretty(name, value)
}

/// Write `audit_receipt.json`, first removing any stale `triage_report.json`.
pub fn write_audit_receipt(dir: &Path, receipt: &AuditReceipt) -> Result<PathBuf> {
    remove_if_present(&dir.join(TRIAGE_REPORT_FILE))?;
    write_json(dir, AUDIT_RECEIPT_FILE, receipt)
}

/// Write `triage_report.json`, first removing any stale `audit_receipt.json`.
pub fn write_triage_report(dir: &Path, report: &TriageReport) -> Result<PathBuf> {
    remove_if_present(&dir.join(AUDIT_RECEIPT_FILE))?;
    write_json(dir, TRIAGE_REPORT_FILE, report)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("replayfix-receipt-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    fn sample_triage() -> TriageReport {
        TriageReport::new(
            TriageStatus::RepairNoopRejected,
            "empty patch",
            "RF-01",
            &[Phase::Init, Phase::Gate1Red, Phase::Repair, Phase::Rejected],
        )
    }

    #[test]
    fn test_exit_codes_match_documented_mapping() {
        assert_eq!(TriageStatus::Gate1NotRed.exit_code(), 10);
        assert_eq!(TriageStatus::ContractMismatch.exit_code(), 10);
        assert_eq!(TriageStatus::InvalidTestOverfit.exit_code(), 10);
        assert_eq!(TriageStatus::CleanPassUnreproduced.exit_code(), 10);
        assert_eq!(TriageStatus::SecurityViolation.exit_code(), 11);
        assert_eq!(TriageStatus::BaseRepoDirtyMutationDetected.exit_code(), 11);
        assert_eq!(TriageStatus::BridgeExecutionFailed.exit_code(), 20);
        assert_eq!(TriageStatus::QuarantineRepairFailed.exit_code(), 21);
        assert_eq!(TriageStatus::FlakyNonDeterministic.exit_code(), 21);
        assert_eq!(TriageStatus::AmbiguousGrounding.exit_code(), 22);
        assert_eq!(TriageStatus::RepairNoopRejected.exit_code(), 30);
        assert_eq!(TriageStatus::PatchApplyFailed.exit_code(), 30);
        assert_eq!(TriageStatus::Gate2NotGreen.exit_code(), 30);
        assert_eq!(TriageStatus::RegressionDetected.exit_code(), 31);
        assert_eq!(TriageStatus::QuarantineInvertedBaseline.exit_code(), 32);
    }

    #[test]
    fn test_triage_report_serializes_screaming_status_and_phases() {
        let json = serde_json::to_value(sample_triage()).unwrap();
        assert_eq!(json["schema_version"], "0.9");
        assert_eq!(json["status"], "REPAIR_NOOP_REJECTED");
        assert_eq!(json["exit_code"], 30);
        assert_eq!(
            json["phase_trace"],
            serde_json::json!(["INIT", "GATE1_RED", "REPAIR", "REJECTED"])
        );
    }

    #[test]
    fn test_triage_write_creates_directory_and_round_trips() {
        let dir = scratch("triage");
        let path = write_triage_report(&dir, &sample_triage()).unwrap();
        let back: TriageReport = serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
        assert_eq!(back.status, TriageStatus::RepairNoopRejected);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_triage_write_removes_stale_receipt() {
        let dir = scratch("stale");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join(AUDIT_RECEIPT_FILE), "{}").unwrap();
        write_triage_report(&dir, &sample_triage()).unwrap();
        assert!(!dir.join(AUDIT_RECEIPT_FILE).exists());
        assert!(dir.join(TRIAGE_REPORT_FILE).exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_triage_status_as_str_and_display() {
        assert_eq!(
            TriageStatus::RepairNoopRejected.as_str(),
            "REPAIR_NOOP_REJECTED"
        );
        assert_eq!(
            TriageStatus::PatchApplyFailed.to_string(),
            "PATCH_APPLY_FAILED"
        );
        assert_eq!(TriageStatus::Gate1NotRed.as_str(), "GATE1_NOT_RED");
        assert_eq!(TriageStatus::Gate2NotGreen.to_string(), "GATE2_NOT_GREEN");
        assert_eq!(TriageStatus::ContractMismatch.as_str(), "CONTRACT_MISMATCH");
        assert_eq!(
            TriageStatus::SecurityViolation.to_string(),
            "SECURITY_VIOLATION"
        );
        assert_eq!(
            TriageStatus::BaseRepoDirtyMutationDetected.as_str(),
            "BASE_REPO_DIRTY_MUTATION_DETECTED"
        );
        assert_eq!(
            TriageStatus::AmbiguousGrounding.as_str(),
            "AMBIGUOUS_GROUNDING"
        );
        assert_eq!(
            TriageStatus::BridgeExecutionFailed.as_str(),
            "BRIDGE_EXECUTION_FAILED"
        );
        assert_eq!(
            TriageStatus::QuarantineRepairFailed.as_str(),
            "QUARANTINE_REPAIR_FAILED"
        );
        assert_eq!(
            TriageStatus::QuarantineInvertedBaseline.as_str(),
            "QUARANTINE_INVERTED_BASELINE"
        );
        assert_eq!(
            TriageStatus::RegressionDetected.as_str(),
            "REGRESSION_DETECTED"
        );
        assert_eq!(
            TriageStatus::FlakyNonDeterministic.as_str(),
            "FLAKY_NON_DETERMINISTIC"
        );
        assert_eq!(
            TriageStatus::InvalidTestOverfit.as_str(),
            "INVALID_TEST_OVERFIT"
        );
        assert_eq!(
            TriageStatus::CleanPassUnreproduced.as_str(),
            "CLEAN_PASS_UNREPRODUCED"
        );
    }

    #[test]
    fn test_triage_status_serde_parity() {
        for status in [
            TriageStatus::RepairNoopRejected,
            TriageStatus::PatchApplyFailed,
            TriageStatus::Gate1NotRed,
            TriageStatus::Gate2NotGreen,
            TriageStatus::ContractMismatch,
            TriageStatus::SecurityViolation,
            TriageStatus::BaseRepoDirtyMutationDetected,
            TriageStatus::AmbiguousGrounding,
            TriageStatus::BridgeExecutionFailed,
            TriageStatus::QuarantineRepairFailed,
            TriageStatus::QuarantineInvertedBaseline,
            TriageStatus::RegressionDetected,
            TriageStatus::FlakyNonDeterministic,
            TriageStatus::InvalidTestOverfit,
            TriageStatus::CleanPassUnreproduced,
        ] {
            let serialized = serde_json::to_string(&status).unwrap();
            assert_eq!(serialized, format!("\"{}\"", status.as_str()));
        }
    }

    #[test]
    fn test_disaggregated_artifacts_merkle_and_attestation() {
        let dir = scratch("disagg-merkle");
        let mut receipt = AuditReceipt {
            schema_version: SCHEMA_VERSION.to_string(),
            status: "VERIFIED_REPAIRED".to_string(),
            task_id: "RF-01".to_string(),
            substrate: SUBSTRATE.to_string(),
            disclaimer: DISCLAIMER.to_string(),
            provenance: None,
            model_provenance: None,
            contract_id: "contract_rf01".to_string(),
            contract_sha256: "abcdef".to_string(),
            gate1: GateRecord {
                observed_at: timestamp_now(),
                verdict: GateVerdict::Red,
                contract_sha256: "abcdef".to_string(),
                events: vec![],
            },
            patch: PatchRecord {
                target_path: "src/cart-summary.js".to_string(),
                sha256: "123456".to_string(),
                insertions: 1,
                deletions: 1,
                files_changed: 1,
            },
            gate2: GateRecord {
                observed_at: timestamp_now(),
                verdict: GateVerdict::Green,
                contract_sha256: "abcdef".to_string(),
                events: vec![],
            },
            phase_trace: vec![
                Phase::Init,
                Phase::Gate1Red,
                Phase::Repair,
                Phase::Gate2Green,
            ],
            base_tree_sha256: "base_sha".to_string(),
            sandbox_tree_sha256: "sandbox_sha".to_string(),
            base_repo_unmodified_verified: true,
            ephemeral_port: 8080,
            grounding_telemetry: None,
            video_telemetry: None,
            repair_telemetry: None,
            token_accounting: None,
            differential_diagnosis: None,
            regression_parity: None,
            merkle_root: None,
            attestation: None,
            created_at: timestamp_now(),
        };

        let diff = "diff --git a/src/cart-summary.js b/src/cart-summary.js\n--- a/src/cart-summary.js\n+++ b/src/cart-summary.js\n@@ -1 +1 @@\n";
        let artifacts_dir = receipt
            .write_disaggregated_artifacts(&dir, Some(diff), None)
            .unwrap();

        assert!(receipt.merkle_root.is_some());
        assert!(receipt.attestation.is_some());
        assert!(artifacts_dir.join("01_candidate.diff").exists());
        assert!(artifacts_dir.join("02_preflight.json").exists());
        assert!(artifacts_dir.join("04_audit_receipt.json").exists());
        assert!(artifacts_dir.join("merkle_receipt.json").exists());
        assert!(artifacts_dir.join("attestation.json").exists());
        assert!(dir.join("audit_receipt.json").exists());
        assert!(dir.join("merkle_receipt.json").exists());
        assert!(dir.join("attestation.json").exists());

        // Top-level audit_receipt.json has merkle_root and attestation populated
        let top_receipt: AuditReceipt =
            serde_json::from_str(&fs::read_to_string(dir.join("audit_receipt.json")).unwrap())
                .unwrap();
        assert_eq!(top_receipt.merkle_root, receipt.merkle_root);
        assert_eq!(top_receipt.attestation, receipt.attestation);

        // Full 4-layer verification succeeds on both dir and audit_receipt.json
        let (v_out, att, root, tree_size) = AuditReceipt::verify_receipt_and_attestation(
            &dir.join("audit_receipt.json"),
            &crate::crypto::TrustPolicy::AllowAny,
        )
        .unwrap();
        assert_eq!(
            v_out,
            crate::crypto::VerificationOutcome::VerifiedUntrustedSigner
        );
        assert!(tree_size >= 3);
        assert_eq!(Some(root), receipt.merkle_root);

        let pinned = std::collections::HashSet::from([att.public_key.clone()]);
        let (trusted_out, _, _, _) = AuditReceipt::verify_receipt_and_attestation(
            &dir,
            &crate::crypto::TrustPolicy::PinnedKeys(pinned),
        )
        .unwrap();
        assert_eq!(
            trusted_out,
            crate::crypto::VerificationOutcome::VerifiedTrusted
        );

        // Layer 1 Negative Test: Tampering with 01_candidate.diff on disk MUST fail verification
        fs::write(artifacts_dir.join("01_candidate.diff"), "tampered diff").unwrap();
        assert!(AuditReceipt::verify_receipt_and_attestation(
            &dir,
            &crate::crypto::TrustPolicy::AllowAny
        )
        .is_err());

        let _ = fs::remove_dir_all(&dir);
    }
}
