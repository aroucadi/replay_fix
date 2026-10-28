# Data Model: Spec 070 Differential Diagnosis & Cryptographic Anti-Cheat

## Rust Domain Entities (`crates/replayfix-core`)

### 1. `DifferentialState` Enum (`crates/replayfix-core/src/fsm/matrix.rs`)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DifferentialState {
    /// Baseline fails, clean baseline passes, patch passes 2x, 100% regression parity.
    CausalDefectConfirmed,
    /// Reproduction test passes on baseline without any fix applied.
    InvalidTestOverfit,
    /// Reproduction test fails on clean reference baseline (test is not defect-specific).
    QuarantineInvertedBaseline,
    /// Test or harness crashed due to infrastructure error rather than assertion failure.
    UnattributedSymptom,
    /// Reproduction test passes on buggy unpatched baseline (bug not reproduced at Gate 1).
    CleanPassUnreproduced,
    /// Patch passed Run 1 but failed Run 2 during stability verification.
    FlakyNonDeterministic,
}

impl DifferentialState {
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::CausalDefectConfirmed => 0,
            Self::InvalidTestOverfit => 10,
            Self::CleanPassUnreproduced => 10,
            Self::UnattributedSymptom => 21,
            Self::FlakyNonDeterministic => 21,
            Self::QuarantineInvertedBaseline => 32,
        }
    }
}
```

### 2. `DifferentialDiagnosisReport` (`crates/replayfix-core/src/fsm/matrix.rs`)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DifferentialDiagnosisReport {
    pub schema_version: String,
    pub bug_id: String,
    pub state: DifferentialState,
    pub clean_ref: String,
    pub clean_ref_commit: String,
    pub baseline_result: ReproExecutionResult,
    pub clean_result: ReproExecutionResult,
    pub patched_run1_result: ReproExecutionResult,
    pub patched_run2_result: ReproExecutionResult,
    pub regression_parity_passed: bool,
    pub causal_confidence: CausalConfidence,
    pub summary: String,
    pub timestamp_epoch_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CausalConfidence {
    High,
    Medium,
    Low,
    None,
}
```

### 3. `RegressionParityReport` & `TestIdentity` (`crates/replayfix-core/src/regression.rs`)

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TestIdentity {
    pub file: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegressionParityReport {
    pub schema_version: String,
    pub baseline_total_tests: usize,
    pub patched_total_tests: usize,
    pub baseline_failures: Vec<String>,
    pub patched_failures: Vec<String>,
    pub new_failures: Vec<String>,
    pub missing_tests: Vec<String>,
    pub baseline_skipped_count: usize,
    pub patched_skipped_count: usize,
    pub skipped_increase: isize,
    pub parity_passed: bool,
    pub duration_ms: u64,
}
```

### 4. `SessionStateCheckpoint` & `StageArtifacts` (`crates/replayfix-core/src/checkpoint.rs`)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStateCheckpoint {
    pub checkpoint_version: String,
    pub bug_id: String,
    pub base_repo_head: String,
    pub manifest_sha256: String,
    pub contract_digest: String,
    pub current_stage: CheckpointStage,
    pub created_at_epoch: u64,
    pub ttl_seconds: u64,
    pub stage_artifacts: StageArtifacts,
}

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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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
```
