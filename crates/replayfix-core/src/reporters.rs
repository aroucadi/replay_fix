//! CI/CD Headless Reporters: SARIF v2.1.0 (`results.sarif`) and JUnit XML (`junit.xml`) (Release 1.5).
//!
//! Enables drop-in integration with GitHub Code Scanning, GitHub Actions test dashboards,
//! and enterprise CI pipelines.

use crate::artifacts::atomic_write_file;
use crate::engine::HealOutcome;
use crate::error::Result;
use crate::receipt::{AuditReceipt, TriageReport};
use crate::sweep::SweepReceipt;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Canonical SARIF v2.1.0 schema URI.
pub const SARIF_SCHEMA_URI: &str = "https://json.schemastore.org/sarif-2.1.0.json";

/// Canonical SARIF version string.
pub const SARIF_VERSION: &str = "2.1.0";

/// Top-level SARIF v2.1.0 log structure.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SarifLog {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub version: String,
    pub runs: Vec<SarifRun>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SarifRun {
    pub tool: SarifTool,
    pub results: Vec<SarifResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SarifTool {
    pub driver: SarifDriver,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SarifDriver {
    pub name: String,
    pub version: String,
    #[serde(rename = "informationUri")]
    pub information_uri: String,
    pub rules: Vec<SarifRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SarifRule {
    pub id: String,
    pub name: String,
    #[serde(rename = "shortDescription")]
    pub short_description: SarifMessage,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SarifMessage {
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SarifResult {
    #[serde(rename = "ruleId")]
    pub rule_id: String,
    pub level: String,
    pub message: SarifMessage,
    pub locations: Vec<SarifLocation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub properties: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SarifLocation {
    #[serde(rename = "physicalLocation")]
    pub physical_location: SarifPhysicalLocation,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SarifPhysicalLocation {
    #[serde(rename = "artifactLocation")]
    pub artifact_location: SarifArtifactLocation,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SarifArtifactLocation {
    pub uri: String,
}

/// SARIF v2.1.0 exporter.
#[derive(Debug, Default)]
pub struct SarifReporter;

impl SarifReporter {
    /// Build a SARIF v2.1.0 log from a single `HealOutcome`.
    #[must_use]
    pub fn from_heal_outcome(outcome: &HealOutcome) -> SarifLog {
        match outcome {
            HealOutcome::Verified(receipt) => Self::from_audit_receipt(receipt),
            HealOutcome::Rejected(report) => Self::from_triage_report(report),
        }
    }

    /// Build a SARIF v2.1.0 log from a verified `AuditReceipt`.
    #[must_use]
    pub fn from_audit_receipt(receipt: &AuditReceipt) -> SarifLog {
        let rule = SarifRule {
            id: receipt.task_id.clone(),
            name: format!(
                "ReplayFixVerifiedRepair_{}",
                receipt.task_id.replace('-', "_")
            ),
            short_description: SarifMessage {
                text: format!(
                    "Causally reproduced and verified repair for {} (contract {})",
                    receipt.task_id, receipt.contract_id
                ),
            },
        };
        let result = SarifResult {
            rule_id: receipt.task_id.clone(),
            level: "note".to_string(),
            message: SarifMessage {
                text: format!(
                    "VERIFIED_REPAIRED: Gate 1 {:?} -> Gate 2 {:?} on '{}'",
                    receipt.gate1.verdict, receipt.gate2.verdict, receipt.patch.target_path
                ),
            },
            locations: vec![SarifLocation {
                physical_location: SarifPhysicalLocation {
                    artifact_location: SarifArtifactLocation {
                        uri: receipt.patch.target_path.clone(),
                    },
                },
            }],
            properties: Some(serde_json::json!({
                "status": receipt.status,
                "substrate": receipt.substrate,
                "merkle_root": receipt.merkle_root,
                "patch_sha256": receipt.patch.sha256,
            })),
        };
        Self::wrap_run(vec![rule], vec![result])
    }

    /// Build a SARIF v2.1.0 log from a rejected `TriageReport`.
    #[must_use]
    pub fn from_triage_report(report: &TriageReport) -> SarifLog {
        let rule = SarifRule {
            id: report.task_id.clone(),
            name: format!("ReplayFixRejected_{}", report.task_id.replace('-', "_")),
            short_description: SarifMessage {
                text: format!("Quarantined or rejected repair for {}", report.task_id),
            },
        };
        let result = SarifResult {
            rule_id: report.task_id.clone(),
            level: "error".to_string(),
            message: SarifMessage {
                text: format!("{:?}: {}", report.status, report.reason),
            },
            locations: vec![SarifLocation {
                physical_location: SarifPhysicalLocation {
                    artifact_location: SarifArtifactLocation {
                        uri: report.task_id.clone(),
                    },
                },
            }],
            properties: Some(serde_json::json!({
                "status": report.status,
                "exit_code": report.status.exit_code(),
            })),
        };
        Self::wrap_run(vec![rule], vec![result])
    }

    /// Build a SARIF v2.1.0 log from a multi-bug `SweepReceipt`.
    #[must_use]
    pub fn from_sweep_receipt(sweep: &SweepReceipt) -> SarifLog {
        let mut rules = Vec::new();
        let mut results = Vec::new();
        for item in sweep.results.values() {
            rules.push(SarifRule {
                id: item.bug_id.clone(),
                name: format!("ReplayFixSweep_{}", item.bug_id.replace('-', "_")),
                short_description: SarifMessage {
                    text: format!("Benchmark sweep evaluation for {}", item.bug_id),
                },
            });
            let passed = item.status == "VERIFIED_REPAIRED" || item.exit_code == 0;
            results.push(SarifResult {
                rule_id: item.bug_id.clone(),
                level: if passed { "note" } else { "error" }.to_string(),
                message: SarifMessage {
                    text: format!(
                        "{}: status={} ({}ms)",
                        item.bug_id, item.status, item.elapsed_ms
                    ),
                },
                locations: vec![SarifLocation {
                    physical_location: SarifPhysicalLocation {
                        artifact_location: SarifArtifactLocation {
                            uri: item.target_path.clone(),
                        },
                    },
                }],
                properties: Some(serde_json::json!({
                    "merkle_root": sweep.merkle_root,
                    "patch_sha256": item.patch_sha256,
                })),
            });
        }
        Self::wrap_run(rules, results)
    }

    /// Write a `SarifLog` atomically to `dest_path`.
    pub fn write_to_file(log: &SarifLog, dest_path: &Path) -> Result<()> {
        if let Some(parent) = dest_path.parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }
        let json = serde_json::to_string_pretty(log)?;
        atomic_write_file(dest_path, json.as_bytes())
    }

    fn wrap_run(rules: Vec<SarifRule>, results: Vec<SarifResult>) -> SarifLog {
        SarifLog {
            schema: SARIF_SCHEMA_URI.to_string(),
            version: SARIF_VERSION.to_string(),
            runs: vec![SarifRun {
                tool: SarifTool {
                    driver: SarifDriver {
                        name: "ReplayFix".to_string(),
                        version: env!("CARGO_PKG_VERSION").to_string(),
                        information_uri: "https://github.com/aroucadi/replay_fix".to_string(),
                        rules,
                    },
                },
                results,
            }],
        }
    }
}

/// JUnit XML exporter for CI/CD test dashboards.
#[derive(Debug, Default)]
pub struct JunitReporter;

impl JunitReporter {
    /// Render JUnit XML from a `HealOutcome`.
    #[must_use]
    pub fn from_heal_outcome(outcome: &HealOutcome) -> String {
        match outcome {
            HealOutcome::Verified(receipt) => {
                let id = xml_escape(&receipt.task_id);
                let tests = if receipt.stress_quorum.is_some() {
                    3
                } else {
                    2
                };
                let mut xml = String::new();
                xml.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
                xml.push('\n');
                xml.push_str(&format!(
                    r#"<testsuites name="ReplayFix Verification" tests="{tests}" failures="0" errors="0">"#
                ));
                xml.push('\n');
                xml.push_str(&format!(
                    r#"  <testsuite name="replayfix.{id}" tests="{tests}" failures="0" errors="0">"#
                ));
                xml.push('\n');
                xml.push_str(&format!(
                    r#"    <testcase name="{id}::Gate1_CausalRed" classname="replayfix.{id}" time="0.010"/>"#
                ));
                xml.push('\n');
                xml.push_str(&format!(
                    r#"    <testcase name="{id}::Gate2_StabilityGreen" classname="replayfix.{id}" time="0.020"/>"#
                ));
                xml.push('\n');
                if let Some(ref sq) = receipt.stress_quorum {
                    xml.push_str(&format!(
                        r#"    <testcase name="{id}::Gate3_StressQuorum" classname="replayfix.{id}" time="{:.3}"/>"#,
                        sq.total_duration_ms as f64 / 1000.0
                    ));
                    xml.push('\n');
                }
                xml.push_str("  </testsuite>\n</testsuites>\n");
                xml
            }
            HealOutcome::Rejected(report) => {
                let id = xml_escape(&report.task_id);
                let reason = xml_escape(&report.reason);
                let mut xml = String::new();
                xml.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
                xml.push('\n');
                xml.push_str(r#"<testsuites name="ReplayFix Verification" tests="1" failures="1" errors="0">"#);
                xml.push('\n');
                xml.push_str(&format!(
                    r#"  <testsuite name="replayfix.{id}" tests="1" failures="1" errors="0">"#
                ));
                xml.push('\n');
                xml.push_str(&format!(
                    r#"    <testcase name="{id}::Verification" classname="replayfix.{id}" time="0.010">"#
                ));
                xml.push('\n');
                xml.push_str(&format!(
                    r#"      <failure message="{reason}" type="{:?}">{reason}</failure>"#,
                    report.status
                ));
                xml.push('\n');
                xml.push_str("    </testcase>\n  </testsuite>\n</testsuites>\n");
                xml
            }
        }
    }

    /// Render JUnit XML from a `SweepReceipt`.
    #[must_use]
    pub fn from_sweep_receipt(sweep: &SweepReceipt) -> String {
        let failures = sweep.failed_bugs;
        let total = sweep.total_bugs;
        let mut xml = String::new();
        xml.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
        xml.push('\n');
        xml.push_str(&format!(
            r#"<testsuites name="ReplayFix Sweep" tests="{total}" failures="{failures}" errors="0">"#
        ));
        xml.push('\n');
        xml.push_str(&format!(
            r#"  <testsuite name="replayfix.sweep" tests="{total}" failures="{failures}" errors="0">"#
        ));
        xml.push('\n');
        for r in sweep.results.values() {
            let secs = r.elapsed_ms as f64 / 1000.0;
            let id = xml_escape(&r.bug_id);
            if r.exit_code == 0 || r.status == "VERIFIED_REPAIRED" {
                xml.push_str(&format!(
                    r#"    <testcase name="{id}" classname="replayfix.sweep" time="{secs:.3}"/>"#
                ));
                xml.push('\n');
            } else {
                let msg = xml_escape(r.error.as_deref().unwrap_or("Gate verification failed"));
                xml.push_str(&format!(
                    r#"    <testcase name="{id}" classname="replayfix.sweep" time="{secs:.3}">"#
                ));
                xml.push('\n');
                xml.push_str(&format!(
                    r#"      <failure message="{msg}" type="GateFailure">{msg}</failure>"#
                ));
                xml.push('\n');
                xml.push_str("    </testcase>\n");
            }
        }
        xml.push_str("  </testsuite>\n</testsuites>\n");
        xml
    }

    /// Write JUnit XML string atomically to `dest_path`.
    pub fn write_to_file(xml_content: &str, dest_path: &Path) -> Result<()> {
        if let Some(parent) = dest_path.parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }
        atomic_write_file(dest_path, xml_content.as_bytes())
    }
}

fn xml_escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace(char::from(39), "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::MockHealer;
    use crate::fixtures::Fixture;

    #[test]
    fn test_sarif_and_junit_generation_from_verified_outcome() {
        let f = Fixture::for_task("RF-01").unwrap();
        let outcome = MockHealer::run(&f, f.builtin_diff).unwrap();

        let sarif = SarifReporter::from_heal_outcome(&outcome);
        assert_eq!(sarif.version, "2.1.0");
        assert_eq!(sarif.runs[0].results[0].rule_id, "RF-01");

        let junit = JunitReporter::from_heal_outcome(&outcome);
        assert!(junit.contains("<testsuites"));
        assert!(junit.contains("RF-01::Gate1_CausalRed"));
        assert!(junit.contains("RF-01::Gate2_StabilityGreen"));
    }
}
