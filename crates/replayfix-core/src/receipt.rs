//! Machine-readable terminal artifacts: audit receipts and triage reports.

use crate::engine::{GateVerdict, OracleEvent};
use crate::error::Result;
use crate::fsm::Phase;
use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

pub const SCHEMA_VERSION: &str = "0.3";
pub const SUBSTRATE: &str = "mock-static-oracle";
pub const DISCLAIMER: &str = "Simulated oracle: gates inspect a built-in fixture. No browser was executed and no model was involved.";
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
pub struct AuditReceipt {
    pub schema_version: String,
    pub status: String,
    pub task_id: String,
    pub substrate: String,
    pub disclaimer: String,
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
    pub created_at: String,
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
        }
    }

    /// Process exit code aligned with the canonical result schemas.
    #[must_use]
    pub const fn exit_code(self) -> i32 {
        match self {
            Self::Gate1NotRed | Self::ContractMismatch => 10,
            Self::SecurityViolation | Self::BaseRepoDirtyMutationDetected => 11,
            Self::RepairNoopRejected | Self::PatchApplyFailed | Self::Gate2NotGreen => 30,
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
    pub created_at: String,
}

impl TriageReport {
    pub fn new(
        status: TriageStatus,
        reason: impl Into<String>,
        task_id: &str,
        phase_trace: &[Phase],
    ) -> Self {
        Self {
            schema_version: SCHEMA_VERSION.to_string(),
            status,
            exit_code: status.exit_code(),
            reason: reason.into(),
            task_id: task_id.to_string(),
            substrate: SUBSTRATE.to_string(),
            phase_trace: phase_trace.to_vec(),
            created_at: timestamp_now(),
        }
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
    fs::create_dir_all(dir)?;
    let path = dir.join(name);
    let mut text = serde_json::to_string_pretty(value)?;
    text.push('\n');
    fs::write(&path, text)?;
    Ok(path)
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
        assert_eq!(TriageStatus::SecurityViolation.exit_code(), 11);
        assert_eq!(TriageStatus::BaseRepoDirtyMutationDetected.exit_code(), 11);
        assert_eq!(TriageStatus::RepairNoopRejected.exit_code(), 30);
        assert_eq!(TriageStatus::PatchApplyFailed.exit_code(), 30);
        assert_eq!(TriageStatus::Gate2NotGreen.exit_code(), 30);
    }

    #[test]
    fn test_triage_report_serializes_screaming_status_and_phases() {
        let json = serde_json::to_value(sample_triage()).unwrap();
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
        ] {
            let serialized = serde_json::to_string(&status).unwrap();
            assert_eq!(serialized, format!("\"{}\"", status.as_str()));
        }
    }
}
