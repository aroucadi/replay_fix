//! Spec 090: High-Performance Five-Bug Benchmark Sweep (`replayfix sweep`),
//! Ephemeral Scratch Isolation, Aggregate Merkle Root, and Ed25519 Attestation.

use crate::artifacts::atomic_write_file;
use crate::crypto::{AttestationRecord, KeypairManager};
use crate::digest::sha256_hex;
use crate::engine::{HealOutcome, LiveBrowserOptions, MockHealer};
use crate::error::{CoreError, Result};
use crate::fixtures::Fixture;
use crate::merkle::{EvidenceLeaf, MerkleTree};
use crate::receipt::{timestamp_now, write_triage_report, SCHEMA_VERSION};
use crate::worktree::{ScratchSandbox, WorktreeManifest};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

/// Individual bug verification outcome within a sweep.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SweepBugResult {
    pub bug_id: String,
    pub status: String,
    pub exit_code: i32,
    pub target_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub patch_sha256: Option<String>,
    pub elapsed_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Consolidated sweep receipt artifact (`sweep_receipt.json`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SweepReceipt {
    pub schema_version: String,
    pub total_bugs: usize,
    pub repaired_bugs: usize,
    pub failed_bugs: usize,
    pub results: BTreeMap<String, SweepBugResult>,
    pub merkle_root: String,
    pub ed25519_attestation: AttestationRecord,
    pub elapsed_wall_clock_ms: u64,
    pub created_at: String,
}

/// Options configuring the benchmark sweep execution.
#[derive(Debug, Clone)]
pub struct SweepOptions {
    pub bug_ids: Vec<String>,
    pub mock_heal: bool,
    pub live_browser: bool,
    pub output_dir: PathBuf,
}

impl Default for SweepOptions {
    fn default() -> Self {
        Self {
            bug_ids: vec![
                "RF-01".to_string(),
                "RF-02".to_string(),
                "RF-03".to_string(),
                "RF-04".to_string(),
                "RF-05".to_string(),
            ],
            mock_heal: true,
            live_browser: false,
            output_dir: PathBuf::from(".replayfix"),
        }
    }
}

/// Execute the authoritative 5-bug benchmark sweep across isolated scratch sandboxes.
pub fn run_sweep(options: &SweepOptions, base_repo: &Path) -> Result<SweepReceipt> {
    let start_wall_clock = Instant::now();
    let supported = Fixture::supported_tasks();

    for bug_id in &options.bug_ids {
        if !supported.contains(&bug_id.as_str()) {
            return Err(CoreError::SecurityViolation(format!(
                "UNSUPPORTED_BENCHMARK_TASK: '{bug_id}' is not in supported tasks {supported:?}"
            )));
        }
    }

    fs::create_dir_all(&options.output_dir)?;

    let pre_manifest = WorktreeManifest::capture_with_excludes(
        base_repo,
        std::slice::from_ref(&options.output_dir),
    )?;

    let mut results = BTreeMap::new();
    let mut leaves = Vec::new();
    let mut repaired_count = 0;
    let mut failed_count = 0;

    for bug_id in &options.bug_ids {
        let bug_start = Instant::now();
        let fixture = Fixture::for_task(bug_id).ok_or_else(|| {
            CoreError::SecurityViolation(format!("Unknown benchmark task: '{bug_id}'"))
        })?;
        let bug_out_dir = options.output_dir.join(bug_id);
        fs::create_dir_all(&bug_out_dir)?;

        let sandbox = ScratchSandbox::create(base_repo, bug_id)?;
        sandbox.write_file(fixture.target_path, fixture.buggy_source)?;

        let outcome = if options.live_browser {
            let live_opts = LiveBrowserOptions {
                headed: false,
                candidate_override: None,
                video_path: None,
                keyframe_dir: None,
                model_slug: None,
                repair_mode: None,
                clean_ref: None,
                verify_matrix: false,
                mock_regression: false,
            };
            MockHealer::run_live_browser_in_repo(
                &fixture,
                fixture.builtin_diff,
                sandbox.root(),
                std::slice::from_ref(&bug_out_dir),
                &live_opts,
            )?
        } else {
            MockHealer::run_in_repo_with_regression(
                &fixture,
                fixture.builtin_diff,
                sandbox.root(),
                std::slice::from_ref(&bug_out_dir),
                false,
            )?
        };

        let bug_elapsed = bug_start.elapsed().as_millis() as u64;

        match outcome {
            HealOutcome::Verified(mut receipt) => {
                repaired_count += 1;
                receipt.write_disaggregated_artifacts(
                    &bug_out_dir,
                    Some(fixture.builtin_diff),
                    None,
                )?;

                let receipt_bytes = fs::read(bug_out_dir.join("audit_receipt.json"))?;
                let content_sha256 = sha256_hex(&receipt_bytes);

                leaves.push(EvidenceLeaf {
                    artifact_type: "sweep_bug_receipt".to_string(),
                    file_name: format!("{bug_id}/audit_receipt.json"),
                    content_sha256,
                    size_bytes: receipt_bytes.len(),
                    timestamp: receipt.created_at.clone(),
                });

                results.insert(
                    bug_id.clone(),
                    SweepBugResult {
                        bug_id: bug_id.clone(),
                        status: "VERIFIED_REPAIRED".to_string(),
                        exit_code: 0,
                        target_path: receipt.patch.target_path,
                        patch_sha256: Some(receipt.patch.sha256),
                        elapsed_ms: bug_elapsed,
                        error: None,
                    },
                );
            }
            HealOutcome::Rejected(triage) => {
                failed_count += 1;
                write_triage_report(&bug_out_dir, &triage)?;

                let triage_bytes = fs::read(bug_out_dir.join("triage_report.json"))?;
                let content_sha256 = sha256_hex(&triage_bytes);

                leaves.push(EvidenceLeaf {
                    artifact_type: "sweep_bug_triage".to_string(),
                    file_name: format!("{bug_id}/triage_report.json"),
                    content_sha256,
                    size_bytes: triage_bytes.len(),
                    timestamp: triage.created_at.clone(),
                });

                results.insert(
                    bug_id.clone(),
                    SweepBugResult {
                        bug_id: bug_id.clone(),
                        status: triage.status.as_str().to_string(),
                        exit_code: triage.exit_code,
                        target_path: fixture.target_path.to_string(),
                        patch_sha256: None,
                        elapsed_ms: bug_elapsed,
                        error: Some(triage.reason),
                    },
                );
            }
        }
    }

    let post_manifest = WorktreeManifest::capture_with_excludes(
        base_repo,
        std::slice::from_ref(&options.output_dir),
    )?;
    pre_manifest.verify_unchanged(&post_manifest)?;

    let merkle_tree = MerkleTree::new(leaves)?;
    let merkle_root = merkle_tree.root_hex();
    let key_dir = options.output_dir.join("keys");
    let keypair = KeypairManager::load_or_generate(Some(&key_dir))?;
    let now = timestamp_now();
    let attestation = keypair.sign(SCHEMA_VERSION, &merkle_root, merkle_tree.len(), &now)?;

    let total_elapsed = start_wall_clock.elapsed().as_millis() as u64;

    let receipt = SweepReceipt {
        schema_version: SCHEMA_VERSION.to_string(),
        total_bugs: options.bug_ids.len(),
        repaired_bugs: repaired_count,
        failed_bugs: failed_count,
        results,
        merkle_root,
        ed25519_attestation: attestation,
        elapsed_wall_clock_ms: total_elapsed,
        created_at: now,
    };

    let receipt_path = options.output_dir.join("sweep_receipt.json");
    let json_bytes = serde_json::to_vec_pretty(&receipt)?;
    atomic_write_file(&receipt_path, &json_bytes)?;

    Ok(receipt)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "replayfix-sweep-test-{}-{name}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn test_sweep_all_five_bugs_mock_heal_sub_60s() {
        let base_dir = scratch_dir("base");
        let out_dir = scratch_dir("out");

        let options = SweepOptions {
            bug_ids: vec![
                "RF-01".to_string(),
                "RF-02".to_string(),
                "RF-03".to_string(),
                "RF-04".to_string(),
                "RF-05".to_string(),
            ],
            mock_heal: true,
            live_browser: false,
            output_dir: out_dir.clone(),
        };

        let receipt = run_sweep(&options, &base_dir).unwrap();
        assert_eq!(receipt.schema_version, "0.9");
        assert_eq!(receipt.total_bugs, 5);
        assert_eq!(receipt.repaired_bugs, 5);
        assert_eq!(receipt.failed_bugs, 0);
        assert!(
            receipt.elapsed_wall_clock_ms < 60_000,
            "Sweep exceeded 60s benchmark ceiling: {}ms",
            receipt.elapsed_wall_clock_ms
        );
        assert!(!receipt.merkle_root.is_empty());
        assert_eq!(receipt.ed25519_attestation.algorithm, "Ed25519");

        // Verify all 5 results
        for bug_id in &["RF-01", "RF-02", "RF-03", "RF-04", "RF-05"] {
            let res = receipt.results.get(*bug_id).unwrap();
            assert_eq!(res.status, "VERIFIED_REPAIRED");
            assert_eq!(res.exit_code, 0);
            assert!(res.patch_sha256.is_some());
        }

        assert!(out_dir.join("sweep_receipt.json").exists());
        let _ = fs::remove_dir_all(&base_dir);
        let _ = fs::remove_dir_all(&out_dir);
    }
}
