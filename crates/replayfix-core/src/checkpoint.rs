//! Spec 070: Durable Checkpoint State for Resumable Execution with Stale-State Rejection
//! and Stage Artifact Disaggregation.

use crate::error::{CoreError, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Checkpoint version constant conforming to session_state.schema.json.
pub const CHECKPOINT_VERSION: &str = "1.0";

/// Default time-to-live for checkpoints (1 hour in seconds).
pub const DEFAULT_CHECKPOINT_TTL_SECONDS: u64 = 3600;

/// Execution lifecycle stages captured in checkpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CheckpointStage {
    Initialized,
    ReproducedGate1Red,
    PatchProposed,
    PreflightPassed,
    VerifiedGate2Green,
    ParityLocked,
    ReceiptIssued,
}

impl CheckpointStage {
    /// True when the stage represents a terminal or completed state.
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::ReceiptIssued)
    }

    /// Check whether a progression from self to next is legal.
    pub fn can_advance_to(self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::Initialized, Self::ReproducedGate1Red)
                | (Self::ReproducedGate1Red, Self::PatchProposed)
                | (Self::PatchProposed, Self::PreflightPassed)
                | (Self::PreflightPassed, Self::VerifiedGate2Green)
                | (Self::VerifiedGate2Green, Self::ParityLocked)
                | (Self::ParityLocked, Self::ReceiptIssued)
        )
    }
}

/// Disaggregated stage artifacts captured during execution.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StageArtifacts {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repro_spec_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repro_spec_sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candidate_diff_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candidate_diff_sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preflight_result_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gate2_evidence_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audit_receipt_path: Option<String>,
}

/// Durable checkpoint state for resumable repair execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionStateCheckpoint {
    pub checkpoint_version: String,
    pub bug_id: String,
    pub base_repo_head: String,
    pub manifest_sha256: String,
    pub contract_digest: String,
    pub current_stage: CheckpointStage,
    pub created_at_epoch: u64,
    pub ttl_seconds: u64,
    #[serde(default)]
    pub stage_artifacts: StageArtifacts,
}

impl SessionStateCheckpoint {
    /// Create a new session checkpoint at `INITIALIZED` stage.
    pub fn new(
        bug_id: impl Into<String>,
        base_repo_head: impl Into<String>,
        manifest_sha256: impl Into<String>,
        contract_digest: impl Into<String>,
        now_epoch: u64,
        ttl_seconds: Option<u64>,
    ) -> Self {
        Self {
            checkpoint_version: CHECKPOINT_VERSION.to_string(),
            bug_id: bug_id.into(),
            base_repo_head: base_repo_head.into(),
            manifest_sha256: manifest_sha256.into(),
            contract_digest: contract_digest.into(),
            current_stage: CheckpointStage::Initialized,
            created_at_epoch: now_epoch,
            ttl_seconds: ttl_seconds.unwrap_or(DEFAULT_CHECKPOINT_TTL_SECONDS),
            stage_artifacts: StageArtifacts::default(),
        }
    }

    /// Validate that the checkpoint is fresh, matches git HEAD, and matches task manifest.
    pub fn validate_freshness(
        &self,
        current_head: &str,
        current_manifest_sha256: &str,
        now_epoch: u64,
    ) -> Result<()> {
        if self.checkpoint_version != CHECKPOINT_VERSION {
            return Err(CoreError::Checkpoint(format!(
                "Incompatible checkpoint version '{}' (expected '{}')",
                self.checkpoint_version, CHECKPOINT_VERSION
            )));
        }

        if !self.base_repo_head.eq_ignore_ascii_case(current_head) {
            return Err(CoreError::Checkpoint(format!(
                "STALE_CHECKPOINT: base_repo_head mismatch (checkpoint: {}, current HEAD: {})",
                self.base_repo_head, current_head
            )));
        }

        if !self
            .manifest_sha256
            .eq_ignore_ascii_case(current_manifest_sha256)
        {
            return Err(CoreError::Checkpoint(format!(
                "STALE_CHECKPOINT: manifest_sha256 mismatch (checkpoint: {}, current: {})",
                self.manifest_sha256, current_manifest_sha256
            )));
        }

        let expiration = self.created_at_epoch.saturating_add(self.ttl_seconds);
        if now_epoch > expiration {
            return Err(CoreError::Checkpoint(format!(
                "STALE_CHECKPOINT: checkpoint expired at epoch {} (now: {}, ttl: {}s)",
                expiration, now_epoch, self.ttl_seconds
            )));
        }

        Ok(())
    }

    /// Advance the checkpoint to the next stage if valid.
    pub fn advance_to(&mut self, next_stage: CheckpointStage) -> Result<()> {
        if !self.current_stage.can_advance_to(next_stage) {
            return Err(CoreError::Checkpoint(format!(
                "Illegal stage transition from {:?} to {:?}",
                self.current_stage, next_stage
            )));
        }
        self.current_stage = next_stage;
        Ok(())
    }

    /// Save checkpoint to file in RFC-friendly formatted JSON.
    pub fn save_to_path(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let serialized = serde_json::to_string_pretty(self)?;
        fs::write(path, serialized)?;
        Ok(())
    }

    /// Load checkpoint from file.
    pub fn load_from_path(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Err(CoreError::Checkpoint(format!(
                "Checkpoint file '{}' does not exist",
                path.display()
            )));
        }
        let contents = fs::read_to_string(path)?;
        let checkpoint: Self = serde_json::from_str(&contents)?;
        Ok(checkpoint)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TempDirGuard(std::path::PathBuf);

    impl TempDirGuard {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir()
                .join(format!("replayfix-cp-test-{}-{name}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDirGuard {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn test_checkpoint_lifecycle_and_transition() {
        let mut cp = SessionStateCheckpoint::new(
            "RF-01",
            "1111111111111111111111111111111111111111",
            "2222222222222222222222222222222222222222222222222222222222222222",
            "contract-hash",
            1000,
            Some(3600),
        );

        assert_eq!(cp.current_stage, CheckpointStage::Initialized);
        assert!(cp.advance_to(CheckpointStage::ReproducedGate1Red).is_ok());
        assert_eq!(cp.current_stage, CheckpointStage::ReproducedGate1Red);

        // Disallow skipping stages
        assert!(cp.advance_to(CheckpointStage::VerifiedGate2Green).is_err());
    }

    #[test]
    fn test_checkpoint_freshness_validation() {
        let cp = SessionStateCheckpoint::new(
            "RF-01",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "digest",
            1000,
            Some(60),
        );

        // Fresh & matching
        assert!(cp
            .validate_freshness(
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                1050
            )
            .is_ok());

        // Head mismatch
        assert!(cp
            .validate_freshness(
                "cccccccccccccccccccccccccccccccccccccccc",
                "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                1050
            )
            .is_err());

        // Manifest mismatch
        assert!(cp
            .validate_freshness(
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
                1050
            )
            .is_err());

        // Expired TTL (1000 + 60 < 1061)
        assert!(cp
            .validate_freshness(
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                1061
            )
            .is_err());
    }

    #[test]
    fn test_checkpoint_roundtrip_save_load() {
        let guard = TempDirGuard::new("roundtrip");
        let path = guard.path().join("session_state.json");

        let mut cp = SessionStateCheckpoint::new(
            "RF-02",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "digest",
            2000,
            Some(3600),
        );
        cp.stage_artifacts.candidate_diff_path = Some(".replayfix/candidate.patch".into());
        cp.stage_artifacts.candidate_diff_sha256 = Some("feedbeef".into());

        cp.save_to_path(&path).unwrap();
        let loaded = SessionStateCheckpoint::load_from_path(&path).unwrap();

        assert_eq!(cp, loaded);
        assert_eq!(
            loaded.stage_artifacts.candidate_diff_path.as_deref(),
            Some(".replayfix/candidate.patch")
        );
    }
}
