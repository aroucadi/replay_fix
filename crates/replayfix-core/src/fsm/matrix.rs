//! Spec 070: Formal 6-State Differential Diagnosis Matrix for Causal Bug Attribution.

use serde::{Deserialize, Serialize};

/// The 6 canonical differential diagnosis states mandated by Spec 070.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DifferentialState {
    /// Baseline defect reproduced (RED), clean reference passes (GREEN), candidate repair
    /// passes 2 consecutive runs (GREEN, GREEN), and regression parity is 100% satisfied.
    CausalDefectConfirmed,
    /// Reproduction test passes on baseline without any repair applied (invalid test reproduction).
    InvalidTestOverfit,
    /// Reproduction test fails on clean reference baseline (test is not defect-specific).
    QuarantineInvertedBaseline,
    /// Execution crashed due to infrastructure, harness, or runtime failure rather than assertion failure.
    UnattributedSymptom,
    /// Reproduction test passes on buggy unpatched baseline (bug not reproduced at Gate 1).
    CleanPassUnreproduced,
    /// Candidate patch passed Run 1 but failed Run 2 during stability verification.
    FlakyNonDeterministic,
}

impl DifferentialState {
    /// Exit code mandated by Spec 070 contract.
    #[must_use]
    pub const fn exit_code(self) -> i32 {
        match self {
            Self::CausalDefectConfirmed => 0,
            Self::InvalidTestOverfit | Self::CleanPassUnreproduced => 10,
            Self::UnattributedSymptom | Self::FlakyNonDeterministic => 21,
            Self::QuarantineInvertedBaseline => 32,
        }
    }

    /// True if the state represents an unqualified, evidence-backed success.
    #[must_use]
    pub const fn is_success(self) -> bool {
        matches!(self, Self::CausalDefectConfirmed)
    }

    /// Human-readable label for terminal status output.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CausalDefectConfirmed => "CAUSAL_DEFECT_CONFIRMED",
            Self::InvalidTestOverfit => "INVALID_TEST_OVERFIT",
            Self::QuarantineInvertedBaseline => "QUARANTINE_INVERTED_BASELINE",
            Self::UnattributedSymptom => "UNATTRIBUTED_SYMPTOM",
            Self::CleanPassUnreproduced => "CLEAN_PASS_UNREPRODUCED",
            Self::FlakyNonDeterministic => "FLAKY_NON_DETERMINISTIC",
        }
    }
}

/// Confidence rating in the causal diagnosis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CausalConfidence {
    High,
    Medium,
    Low,
    None,
}

/// Outcome of a single test execution run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReproExecutionResult {
    pub passed: bool,
    pub exit_code: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

impl ReproExecutionResult {
    #[must_use]
    pub fn pass(exit_code: i32) -> Self {
        Self {
            passed: true,
            exit_code,
            error_message: None,
        }
    }

    #[must_use]
    pub fn fail(exit_code: i32, msg: impl Into<String>) -> Self {
        Self {
            passed: false,
            exit_code,
            error_message: Some(msg.into()),
        }
    }

    #[must_use]
    pub fn is_infra_crash(&self) -> bool {
        self.exit_code == -1
            || self
                .error_message
                .as_deref()
                .map(|m| {
                    m.contains("INFRA_CRASH") || m.contains("SIGSEGV") || m.contains("EADDRINUSE")
                })
                .unwrap_or(false)
    }
}

/// Comprehensive differential diagnosis report emitted in AuditReceipt / TriageReport.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

/// Causal decision matrix evaluating test results across baseline, clean ref, and patch runs.
#[derive(Debug, Clone, Copy, Default)]
pub struct DifferentialMatrix;

impl DifferentialMatrix {
    /// Evaluate all 4 reproduction runs and regression parity to resolve the canonical differential state.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn evaluate(
        bug_id: &str,
        clean_ref: &str,
        clean_ref_commit: &str,
        baseline: ReproExecutionResult,
        clean: ReproExecutionResult,
        patched_run1: ReproExecutionResult,
        patched_run2: ReproExecutionResult,
        regression_parity_passed: bool,
        now_epoch_ms: u64,
    ) -> DifferentialDiagnosisReport {
        let (state, confidence, summary) = if baseline.is_infra_crash()
            || clean.is_infra_crash()
            || patched_run1.is_infra_crash()
            || patched_run2.is_infra_crash()
        {
            (
                DifferentialState::UnattributedSymptom,
                CausalConfidence::None,
                "Reproduction aborted due to harness or infrastructure failure".to_string(),
            )
        } else if baseline.passed {
            if !clean.passed {
                (
                    DifferentialState::InvalidTestOverfit,
                    CausalConfidence::Low,
                    "Reproduction test unexpectedly passed on buggy baseline and failed on clean reference".to_string(),
                )
            } else {
                (
                    DifferentialState::CleanPassUnreproduced,
                    CausalConfidence::Low,
                    "Reproduction test passed on unpatched baseline; bug was not reproduced (Gate 1 not RED)".to_string(),
                )
            }
        } else if !clean.passed {
            (
                DifferentialState::QuarantineInvertedBaseline,
                CausalConfidence::Low,
                format!(
                    "Reproduction test failed on clean reference baseline '{}' ({clean_ref_commit}); test is not defect-specific",
                    clean_ref
                ),
            )
        } else if !patched_run1.passed || !patched_run2.passed {
            if patched_run1.passed != patched_run2.passed {
                (
                    DifferentialState::FlakyNonDeterministic,
                    CausalConfidence::Low,
                    "Patch exhibited non-deterministic behavior across the 2-run stability pass"
                        .to_string(),
                )
            } else {
                (
                    DifferentialState::FlakyNonDeterministic,
                    CausalConfidence::None,
                    "Candidate repair failed Gate 2 verification across stability runs".to_string(),
                )
            }
        } else if !regression_parity_passed {
            (
                DifferentialState::FlakyNonDeterministic,
                CausalConfidence::Medium,
                "Candidate repair passed reproduction test but introduced regressions in pre-existing test suite".to_string(),
            )
        } else {
            (
                DifferentialState::CausalDefectConfirmed,
                CausalConfidence::High,
                "Causal defect confirmed: baseline reproduced, clean reference healthy, patch verified stable 2x with zero regressions".to_string(),
            )
        };

        DifferentialDiagnosisReport {
            schema_version: "0.9".to_string(),
            bug_id: bug_id.to_string(),
            state,
            clean_ref: clean_ref.to_string(),
            clean_ref_commit: clean_ref_commit.to_string(),
            baseline_result: baseline,
            clean_result: clean,
            patched_run1_result: patched_run1,
            patched_run2_result: patched_run2,
            regression_parity_passed,
            causal_confidence: confidence,
            summary,
            timestamp_epoch_ms: now_epoch_ms,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_causal_defect_confirmed() {
        let rep = DifferentialMatrix::evaluate(
            "RF-01",
            "main",
            "1234567890123456789012345678901234567890",
            ReproExecutionResult::fail(1, "Assertion failed"),
            ReproExecutionResult::pass(0),
            ReproExecutionResult::pass(0),
            ReproExecutionResult::pass(0),
            true,
            1234567,
        );

        assert_eq!(rep.state, DifferentialState::CausalDefectConfirmed);
        assert_eq!(rep.state.exit_code(), 0);
        assert_eq!(rep.causal_confidence, CausalConfidence::High);
    }

    #[test]
    fn test_quarantine_inverted_baseline() {
        let rep = DifferentialMatrix::evaluate(
            "RF-01",
            "main",
            "1234567890123456789012345678901234567890",
            ReproExecutionResult::fail(1, "Assertion failed"),
            ReproExecutionResult::fail(1, "Clean ref failed too"),
            ReproExecutionResult::pass(0),
            ReproExecutionResult::pass(0),
            true,
            1234567,
        );

        assert_eq!(rep.state, DifferentialState::QuarantineInvertedBaseline);
        assert_eq!(rep.state.exit_code(), 32);
    }

    #[test]
    fn test_clean_pass_unreproduced() {
        let rep = DifferentialMatrix::evaluate(
            "RF-01",
            "main",
            "1234567890123456789012345678901234567890",
            ReproExecutionResult::pass(0),
            ReproExecutionResult::pass(0),
            ReproExecutionResult::pass(0),
            ReproExecutionResult::pass(0),
            true,
            1234567,
        );

        assert_eq!(rep.state, DifferentialState::CleanPassUnreproduced);
        assert_eq!(rep.state.exit_code(), 10);
    }

    #[test]
    fn test_flaky_non_deterministic() {
        let rep = DifferentialMatrix::evaluate(
            "RF-01",
            "main",
            "1234567890123456789012345678901234567890",
            ReproExecutionResult::fail(1, "Assertion failed"),
            ReproExecutionResult::pass(0),
            ReproExecutionResult::pass(0),
            ReproExecutionResult::fail(1, "Run 2 failed"),
            true,
            1234567,
        );

        assert_eq!(rep.state, DifferentialState::FlakyNonDeterministic);
        assert_eq!(rep.state.exit_code(), 21);
    }
}
