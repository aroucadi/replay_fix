//! Spec 070: Structured Regression Parity Engine Enforcing F_patched \ F_baseline = ∅.

use crate::error::{CoreError, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Status of an individual test case.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TestStatus {
    Passed,
    Failed,
    Skipped,
    TimedOut,
}

impl TestStatus {
    #[must_use]
    pub const fn is_failure(self) -> bool {
        matches!(self, Self::Failed | Self::TimedOut)
    }

    #[must_use]
    pub const fn is_skipped(self) -> bool {
        matches!(self, Self::Skipped)
    }
}

/// Normalized test suite result for a single test run.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestSuiteSnapshot {
    pub tests: BTreeMap<String, TestStatus>,
    pub duration_ms: u64,
}

impl TestSuiteSnapshot {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, test_id: impl Into<String>, status: TestStatus) {
        self.tests.insert(test_id.into(), status);
    }

    #[must_use]
    pub fn total_count(&self) -> usize {
        self.tests.len()
    }

    #[must_use]
    pub fn failures(&self) -> BTreeSet<String> {
        self.tests
            .iter()
            .filter(|(_, status)| status.is_failure())
            .map(|(id, _)| id.clone())
            .collect()
    }

    #[must_use]
    pub fn skipped_count(&self) -> usize {
        self.tests
            .values()
            .filter(|status| status.is_skipped())
            .count()
    }

    /// Parse Playwright / Jest JSON reporter output, or generic JSON summary.
    pub fn parse_json(raw_json: &str) -> Result<Self> {
        let value: serde_json::Value = serde_json::from_str(raw_json)?;
        let mut snapshot = Self::new();

        // 1. Try generic summary format: { "tests": [ { "id": "...", "status": "..." } ] }
        if let Some(tests_arr) = value.get("tests").and_then(|v| v.as_array()) {
            for item in tests_arr {
                let id = item
                    .get("id")
                    .or_else(|| item.get("name"))
                    .or_else(|| item.get("title"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown_test");
                let status_str = item
                    .get("status")
                    .and_then(|v| v.as_str())
                    .unwrap_or("failed");
                let status = match status_str.to_ascii_lowercase().as_str() {
                    "passed" | "pass" | "ok" => TestStatus::Passed,
                    "skipped" | "pending" | "ignored" => TestStatus::Skipped,
                    "timedout" | "timeout" => TestStatus::TimedOut,
                    _ => TestStatus::Failed,
                };
                snapshot.insert(id, status);
            }
            if let Some(dur) = value.get("duration_ms").and_then(|v| v.as_u64()) {
                snapshot.duration_ms = dur;
            }
            return Ok(snapshot);
        }

        // 2. Try Playwright JSON reporter format: { "suites": [ ... ] }
        if let Some(suites) = value.get("suites").and_then(|v| v.as_array()) {
            Self::extract_playwright_suites(suites, "", &mut snapshot);
            return Ok(snapshot);
        }

        Err(CoreError::Regression(
            "Unrecognized test report JSON schema; expected 'tests' array or Playwright 'suites'"
                .to_string(),
        ))
    }

    fn extract_playwright_suites(suites: &[serde_json::Value], prefix: &str, snapshot: &mut Self) {
        for suite in suites {
            let title = suite.get("title").and_then(|v| v.as_str()).unwrap_or("");
            let suite_prefix = if prefix.is_empty() {
                title.to_string()
            } else if title.is_empty() {
                prefix.to_string()
            } else {
                format!("{prefix} > {title}")
            };

            if let Some(specs) = suite.get("specs").and_then(|v| v.as_array()) {
                for spec in specs {
                    let spec_title = spec.get("title").and_then(|v| v.as_str()).unwrap_or("");
                    let test_id = if suite_prefix.is_empty() {
                        spec_title.to_string()
                    } else {
                        format!("{suite_prefix} > {spec_title}")
                    };

                    let mut spec_status = TestStatus::Passed;
                    if let Some(tests) = spec.get("tests").and_then(|v| v.as_array()) {
                        for t in tests {
                            if let Some(results) = t.get("results").and_then(|v| v.as_array()) {
                                for r in results {
                                    if let Some(st) = r.get("status").and_then(|v| v.as_str()) {
                                        match st {
                                            "failed" | "timedOut" => {
                                                spec_status = TestStatus::Failed;
                                                break;
                                            }
                                            "skipped" if spec_status != TestStatus::Failed => {
                                                spec_status = TestStatus::Skipped;
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                            }
                        }
                    }
                    snapshot.insert(test_id, spec_status);
                }
            }

            if let Some(sub_suites) = suite.get("suites").and_then(|v| v.as_array()) {
                Self::extract_playwright_suites(sub_suites, &suite_prefix, snapshot);
            }
        }
    }
}

/// Detailed regression parity audit report conforming to regression_parity.schema.json.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, Default)]
pub struct RegressionEvaluator;

impl RegressionEvaluator {
    /// Evaluate regression parity by verifying:
    /// 1. F_patched \ F_baseline = ∅ (zero newly introduced failures)
    /// 2. T_baseline \ T_patched = ∅ (zero removed tests)
    /// 3. Skipped_patched <= Skipped_baseline (zero silent test skips)
    #[must_use]
    pub fn evaluate(
        baseline: &TestSuiteSnapshot,
        patched: &TestSuiteSnapshot,
    ) -> RegressionParityReport {
        let f_baseline = baseline.failures();
        let f_patched = patched.failures();

        let new_failures: Vec<String> = f_patched.difference(&f_baseline).cloned().collect();

        let t_baseline_keys: BTreeSet<String> = baseline.tests.keys().cloned().collect();
        let t_patched_keys: BTreeSet<String> = patched.tests.keys().cloned().collect();
        let missing_tests: Vec<String> = t_baseline_keys
            .difference(&t_patched_keys)
            .cloned()
            .collect();

        let baseline_skipped = baseline.skipped_count();
        let patched_skipped = patched.skipped_count();
        let skipped_increase = (patched_skipped as isize) - (baseline_skipped as isize);

        let parity_passed =
            new_failures.is_empty() && missing_tests.is_empty() && skipped_increase <= 0;

        RegressionParityReport {
            schema_version: "0.7".to_string(),
            baseline_total_tests: baseline.total_count(),
            patched_total_tests: patched.total_count(),
            baseline_failures: f_baseline.into_iter().collect(),
            patched_failures: f_patched.into_iter().collect(),
            new_failures,
            missing_tests,
            baseline_skipped_count: baseline_skipped,
            patched_skipped_count: patched_skipped,
            skipped_increase,
            parity_passed,
            duration_ms: patched.duration_ms,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_perfect_parity() {
        let mut baseline = TestSuiteSnapshot::new();
        baseline.insert("test_a", TestStatus::Passed);
        baseline.insert("test_b", TestStatus::Failed);

        let mut patched = TestSuiteSnapshot::new();
        patched.insert("test_a", TestStatus::Passed);
        patched.insert("test_b", TestStatus::Failed);

        let report = RegressionEvaluator::evaluate(&baseline, &patched);
        assert!(report.parity_passed);
        assert!(report.new_failures.is_empty());
        assert!(report.missing_tests.is_empty());
    }

    #[test]
    fn test_subset_repair_is_allowed() {
        let mut baseline = TestSuiteSnapshot::new();
        baseline.insert("test_a", TestStatus::Failed);
        baseline.insert("test_b", TestStatus::Failed);

        let mut patched = TestSuiteSnapshot::new();
        patched.insert("test_a", TestStatus::Passed); // fixed!
        patched.insert("test_b", TestStatus::Failed); // pre-existing, still failing

        let report = RegressionEvaluator::evaluate(&baseline, &patched);
        assert!(report.parity_passed);
        assert!(report.new_failures.is_empty());
    }

    #[test]
    fn test_new_failure_rejected() {
        let mut baseline = TestSuiteSnapshot::new();
        baseline.insert("test_a", TestStatus::Passed);
        baseline.insert("test_b", TestStatus::Passed);

        let mut patched = TestSuiteSnapshot::new();
        patched.insert("test_a", TestStatus::Passed);
        patched.insert("test_b", TestStatus::Failed); // Regression!

        let report = RegressionEvaluator::evaluate(&baseline, &patched);
        assert!(!report.parity_passed);
        assert_eq!(report.new_failures, vec!["test_b"]);
    }

    #[test]
    fn test_deleted_test_rejected() {
        let mut baseline = TestSuiteSnapshot::new();
        baseline.insert("test_a", TestStatus::Passed);
        baseline.insert("test_b", TestStatus::Passed);

        let mut patched = TestSuiteSnapshot::new();
        patched.insert("test_a", TestStatus::Passed);
        // test_b deleted!

        let report = RegressionEvaluator::evaluate(&baseline, &patched);
        assert!(!report.parity_passed);
        assert_eq!(report.missing_tests, vec!["test_b"]);
    }

    #[test]
    fn test_silent_skip_inflation_rejected() {
        let mut baseline = TestSuiteSnapshot::new();
        baseline.insert("test_a", TestStatus::Passed);
        baseline.insert("test_b", TestStatus::Passed);

        let mut patched = TestSuiteSnapshot::new();
        patched.insert("test_a", TestStatus::Passed);
        patched.insert("test_b", TestStatus::Skipped); // Skipped instead of run!

        let report = RegressionEvaluator::evaluate(&baseline, &patched);
        assert!(!report.parity_passed);
        assert_eq!(report.skipped_increase, 1);
    }

    #[test]
    fn test_parse_playwright_json() {
        let json = r#"{
            "suites": [
                {
                    "title": "Checkout",
                    "specs": [
                        {
                            "title": "calculates tax",
                            "tests": [
                                {
                                    "results": [
                                        { "status": "passed" }
                                    ]
                                }
                            ]
                        }
                    ]
                }
            ]
        }"#;

        let snapshot = TestSuiteSnapshot::parse_json(json).unwrap();
        assert_eq!(snapshot.total_count(), 1);
        assert!(snapshot.failures().is_empty());
        assert_eq!(
            snapshot.tests.get("Checkout > calculates tax"),
            Some(&TestStatus::Passed)
        );
    }
}
