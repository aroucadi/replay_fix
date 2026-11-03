//! End-to-end tests that drive the compiled `replayfix` binary.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const BASE: [&str; 7] = [
    "repro",
    "--task-id",
    "RF-01",
    "--bug-ref",
    "bug/RF-01",
    "--video",
    "recordings/RF-01.mp4",
];

fn scratch(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("cli-{name}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(dir: &Path, extra: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_replayfix"))
        .current_dir(dir)
        .args(BASE)
        .args(extra)
        .output()
        .expect("failed to launch replayfix")
}

fn code(out: &Output) -> i32 {
    out.status.code().unwrap_or(-1)
}

fn json(path: &Path) -> serde_json::Value {
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn mock_heal_verifies_and_writes_receipt() {
    let dir = scratch("success");
    let out = run(&dir, &["--mock-heal"]);
    assert_eq!(
        code(&out),
        0,
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let receipt = json(&dir.join(".replayfix/audit_receipt.json"));
    assert_eq!(receipt["schema_version"], "0.9");
    assert_eq!(receipt["status"], "VERIFIED_REPAIRED");
    assert_eq!(receipt["gate1"]["verdict"], "RED");
    assert_eq!(receipt["gate2"]["verdict"], "GREEN");
    assert_eq!(
        receipt["gate1"]["contract_sha256"],
        receipt["gate2"]["contract_sha256"]
    );
    assert_eq!(
        receipt["phase_trace"],
        serde_json::json!(["INIT", "GATE1_RED", "REPAIR", "GATE2_GREEN"])
    );
    assert_eq!(receipt["base_repo_unmodified_verified"], true);
    assert_eq!(receipt["base_tree_sha256"].as_str().unwrap().len(), 64);
    assert_eq!(receipt["sandbox_tree_sha256"].as_str().unwrap().len(), 64);
    assert_ne!(receipt["base_tree_sha256"], receipt["sandbox_tree_sha256"]);
    assert!(receipt["ephemeral_port"].as_u64().unwrap() > 0);
    assert_eq!(receipt["provenance"], "deterministic-fixture");
    assert!(receipt["disclaimer"]
        .as_str()
        .unwrap()
        .contains("Deterministic fixture replay"));
    assert!(!dir.join(".replayfix/triage_report.json").exists());
}

#[test]
fn live_browser_verifies_and_writes_cdp_grounding_receipt() {
    let dir = scratch("live-browser");
    let out = run(&dir, &["--live-browser"]);
    assert_eq!(
        code(&out),
        0,
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let receipt = json(&dir.join(".replayfix/audit_receipt.json"));
    assert_eq!(receipt["schema_version"], "0.9");
    assert_eq!(receipt["status"], "VERIFIED_REPAIRED");
    assert_eq!(receipt["substrate"], "playwright-cdp");
    assert_eq!(receipt["gate1"]["verdict"], "RED");
    assert_eq!(receipt["gate2"]["verdict"], "GREEN");
    assert_eq!(receipt["base_repo_unmodified_verified"], true);
    assert_eq!(
        receipt["grounding_telemetry"]["matched_selector"],
        "page.getByRole('button', { name: 'Refresh Cart', exact: true })"
    );
    assert_eq!(
        receipt["grounding_telemetry"]["element_from_point_match"],
        true
    );
    assert_eq!(
        receipt["grounding_telemetry"]["accessible_name_match"],
        true
    );
    assert_eq!(receipt["grounding_telemetry"]["candidate_score_bps"], 10000);
}

#[test]
fn live_browser_ambiguous_grounding_exits_22_with_triage_report() {
    let dir = scratch("live-ambiguous");
    let out = run(&dir, &["--live-browser", "--mock-ambiguous"]);
    assert_eq!(
        code(&out),
        22,
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let triage = json(&dir.join(".replayfix/triage_report.json"));
    assert_eq!(triage["schema_version"], "0.9");
    assert_eq!(triage["status"], "AMBIGUOUS_GROUNDING");
    assert_eq!(triage["exit_code"], 22);
    assert_eq!(triage["substrate"], "playwright-cdp");
    assert!(!dir.join(".replayfix/audit_receipt.json").exists());
}

#[test]
fn noop_patch_is_rejected_with_triage_report() {
    let dir = scratch("noop");
    let out = run(&dir, &["--mock-heal", "--mock-patch", "noop"]);
    assert_eq!(code(&out), 30);

    let triage = json(&dir.join(".replayfix/triage_report.json"));
    assert_eq!(triage["schema_version"], "0.9");
    assert_eq!(triage["status"], "REPAIR_NOOP_REJECTED");
    assert_eq!(triage["exit_code"], 30);
    assert!(!dir.join(".replayfix/audit_receipt.json").exists());
}

#[test]
fn rejected_run_removes_stale_receipt() {
    let dir = scratch("stale");
    assert_eq!(code(&run(&dir, &["--mock-heal"])), 0);
    assert!(dir.join(".replayfix/audit_receipt.json").exists());

    assert_eq!(
        code(&run(&dir, &["--mock-heal", "--mock-patch", "noop"])),
        30
    );
    assert!(!dir.join(".replayfix/audit_receipt.json").exists());
    assert!(dir.join(".replayfix/triage_report.json").exists());

    assert_eq!(code(&run(&dir, &["--mock-heal"])), 0);
    assert!(!dir.join(".replayfix/triage_report.json").exists());
}

#[test]
fn custom_patch_file_is_applied() {
    let dir = scratch("custom");
    fs::write(
        dir.join("fix.diff"),
        "--- a/src/cart-summary.js\n+++ b/src/cart-summary.js\n@@ -2 +2 @@\n-  return cart.items.length;\n+  return (cart.items ?? []).length;\n",
    )
    .unwrap();
    let out = run(&dir, &["--mock-heal", "--mock-patch", "fix.diff"]);
    assert_eq!(
        code(&out),
        0,
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn ineffective_patch_file_ends_with_gate2_not_green() {
    let dir = scratch("ineffective");
    fs::write(
        dir.join("bad.diff"),
        "--- a/src/cart-summary.js\n+++ b/src/cart-summary.js\n@@ -2 +2 @@\n-  return cart.items.length;\n+  return cart.items.length + 1;\n",
    )
    .unwrap();
    let out = run(&dir, &["--mock-heal", "--mock-patch", "bad.diff"]);
    assert_eq!(code(&out), 30);
    assert_eq!(
        json(&dir.join(".replayfix/triage_report.json"))["status"],
        "GATE2_NOT_GREEN"
    );
}

#[test]
fn unsafe_output_dir_is_a_security_violation() {
    let dir = scratch("unsafe-out");
    let out = run(&dir, &["--mock-heal", "--output-dir", "../escape"]);
    assert_eq!(code(&out), 11);
    assert!(String::from_utf8_lossy(&out.stderr).contains("SECURITY_VIOLATION"));
    assert!(!dir.join("../escape").exists());
}

#[test]
fn unsafe_patch_path_is_a_security_violation() {
    let dir = scratch("unsafe-patch");
    let out = run(&dir, &["--mock-heal", "--mock-patch", "../../secret.diff"]);
    assert_eq!(code(&out), 11);
}

#[test]
fn missing_patch_file_is_a_usage_error() {
    let dir = scratch("missing-patch");
    let out = run(&dir, &["--mock-heal", "--mock-patch", "nope.diff"]);
    assert_eq!(code(&out), 2);
    assert!(String::from_utf8_lossy(&out.stderr).contains("Cannot read patch file"));
}

#[test]
fn oversized_patch_file_is_rejected() {
    let dir = scratch("oversized");
    fs::write(dir.join("big.diff"), vec![b'x'; 1024 * 1024 + 1]).unwrap();
    let out = run(&dir, &["--mock-heal", "--mock-patch", "big.diff"]);
    assert_eq!(code(&out), 2);
    assert!(String::from_utf8_lossy(&out.stderr).contains("limit"));
}

#[test]
fn unknown_task_lists_supported_fixtures() {
    let dir = scratch("unknown-task");
    let out = Command::new(env!("CARGO_BIN_EXE_replayfix"))
        .current_dir(&dir)
        .args([
            "repro",
            "--task-id",
            "RF-99",
            "--bug-ref",
            "bug/x",
            "--video",
            "v.mp4",
            "--mock-heal",
        ])
        .output()
        .unwrap();
    assert_eq!(code(&out), 2);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("RF-99") && stderr.contains("RF-01"));
    assert!(!dir.join(".replayfix").exists());
}

#[test]
fn mock_flags_require_mock_heal() {
    let dir = scratch("requires");
    assert_eq!(code(&run(&dir, &["--mock-patch", "noop"])), 2);
    assert_eq!(code(&run(&dir, &["--output-dir", "out"])), 2);
}

#[test]
fn repro_without_mock_heal_keeps_v01_behaviour() {
    let dir = scratch("v01");
    let out = run(&dir, &[]);
    assert_eq!(code(&out), 0);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("VALIDATED"));
    assert!(stdout.contains("--mock-heal"));
    assert!(!dir.join(".replayfix").exists());
}

#[test]
fn repro_works_through_a_manifest_file() {
    let dir = scratch("manifest");
    fs::write(
        dir.join("task.json"),
        r#"{"task_id":"RF-01","title":"Cart count crash","repo_path":".","bug_ref":"bug/RF-01","video_path":"v.mp4"}"#,
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_replayfix"))
        .current_dir(&dir)
        .args(["repro", "--manifest", "task.json", "--mock-heal"])
        .output()
        .unwrap();
    assert_eq!(
        code(&out),
        0,
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn version_comes_from_crate_metadata() {
    let out = Command::new(env!("CARGO_BIN_EXE_replayfix"))
        .arg("--version")
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&out.stdout).contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn missing_required_arguments_exit_with_code_2() {
    let out = Command::new(env!("CARGO_BIN_EXE_replayfix"))
        .args(["repro", "--task-id", "RF-01"])
        .output()
        .unwrap();
    assert_eq!(code(&out), 2);
}

#[test]
fn mock_heal_leaves_existing_base_repo_files_unmodified() {
    let dir = scratch("preserve-repo");
    fs::create_dir_all(dir.join("src")).unwrap();
    let buggy = "export function cartItemCount(cart) {\n  return cart.items.length;\n}\n";
    fs::write(dir.join("src/cart-summary.js"), buggy).unwrap();

    let out = run(&dir, &["--mock-heal"]);
    assert_eq!(
        code(&out),
        0,
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // Base repository file must remain completely untouched
    let after = fs::read_to_string(dir.join("src/cart-summary.js")).unwrap();
    assert_eq!(after, buggy);

    let receipt = json(&dir.join(".replayfix/audit_receipt.json"));
    assert_eq!(receipt["base_repo_unmodified_verified"], true);
    assert_ne!(receipt["base_tree_sha256"], receipt["sandbox_tree_sha256"]);
}

#[test]
fn nonexistent_repo_directory_is_a_usage_error() {
    let dir = scratch("missing-repo");
    let out = run(&dir, &["--repo", "does-not-exist", "--mock-heal"]);
    assert_eq!(code(&out), 2);
    assert!(String::from_utf8_lossy(&out.stderr).contains("does not exist"));
}

#[test]
fn run_subcommand_alias_with_bug_flag_works() {
    let dir = scratch("run-bug-alias");
    let out = Command::new(env!("CARGO_BIN_EXE_replayfix"))
        .current_dir(&dir)
        .args(["run", "--bug", "RF-01", "--mock-heal"])
        .output()
        .unwrap();
    assert_eq!(
        code(&out),
        0,
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let receipt = json(&dir.join(".replayfix/audit_receipt.json"));
    assert_eq!(receipt["task_id"], "RF-01");
    assert_eq!(receipt["status"], "VERIFIED_REPAIRED");
}

#[test]
fn live_browser_video_telemetry_ingests_keyframes_sam_and_asr() {
    let dir = scratch("video-telemetry-e2e");
    let video_dir = dir.join("assets").join("fixtures");
    fs::create_dir_all(&video_dir).unwrap();
    let mp4_path = video_dir.join("rf01_demo.mp4");
    replayfix_core::VideoPreprocessor::synthesize_demo_video(&mp4_path).unwrap();

    let out = Command::new(env!("CARGO_BIN_EXE_replayfix"))
        .current_dir(&dir)
        .args([
            "run",
            "--video",
            "assets/fixtures/rf01_demo.mp4",
            "--repo",
            ".",
        ])
        .output()
        .unwrap();
    assert_eq!(
        code(&out),
        0,
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let receipt = json(&dir.join(".replayfix/audit_receipt.json"));
    assert_eq!(receipt["schema_version"], "0.9");
    assert_eq!(receipt["status"], "VERIFIED_REPAIRED");
    assert_eq!(receipt["substrate"], "playwright-cdp");
    assert_eq!(
        receipt["video_telemetry"]["sam_snap"]["model"],
        "local_diff_heuristic"
    );
    assert_eq!(
        receipt["video_telemetry"]["sam_snap"]["calibrated_x_css"],
        160
    );
    assert_eq!(
        receipt["video_telemetry"]["sam_snap"]["calibrated_y_css"],
        120
    );
    assert_eq!(
        receipt["video_telemetry"]["narration"]["model"],
        "simulated_fixture"
    );
    let keyframes = receipt["video_telemetry"]["keyframe_paths"]
        .as_array()
        .expect("keyframe_paths array");
    assert_eq!(keyframes.len(), 1);
    assert!(dir.join(".replayfix/keyframes").exists());
}

#[test]
fn model_autonomous_repair_and_syntax_retry_write_receipt() {
    let dir = scratch("model-autonomous-retry");
    let out = Command::new(env!("CARGO_BIN_EXE_replayfix"))
        .current_dir(&dir)
        .args([
            "run",
            "--bug",
            "RF-01",
            "--model",
            "muse-spark-1.3",
            "--mock-syntax-retry",
            "--output-dir",
            "out-retry",
        ])
        .output()
        .unwrap();
    assert_eq!(
        code(&out),
        0,
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let receipt = json(&dir.join("out-retry/audit_receipt.json"));
    assert_eq!(receipt["schema_version"], "0.9");
    assert_eq!(receipt["provenance"], "deterministic-fixture");
    assert_eq!(
        receipt["repair_telemetry"]["requested_model"],
        "muse-spark-1.3"
    );
    assert_eq!(
        receipt["repair_telemetry"]["serving_model"],
        "muse-spark-1.3"
    );
    assert_eq!(receipt["repair_telemetry"]["reasoning_effort"], "xhigh");
    assert_eq!(receipt["repair_telemetry"]["attempts_used"], 2);
    assert_eq!(receipt["repair_telemetry"]["gate2_runs_completed"], 2);
    assert_eq!(receipt["repair_telemetry"]["gate2_stability_passed"], true);
    assert_eq!(
        receipt["repair_telemetry"]["encrypted_deliberation_preserved"],
        true
    );
    assert!(
        receipt.get("token_accounting").is_none() || receipt["token_accounting"].is_null(),
        "token_accounting must be None on deterministic-fixture runs"
    );
}

#[test]
fn mock_failing_repair_exits_21_with_quarantine_triage_report() {
    let dir = scratch("quarantine-v06");
    let out = Command::new(env!("CARGO_BIN_EXE_replayfix"))
        .current_dir(&dir)
        .args([
            "run",
            "--bug",
            "RF-01",
            "--mock-failing-repair",
            "--output-dir",
            "out-quarantine",
        ])
        .output()
        .unwrap();
    assert_eq!(
        code(&out),
        21,
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let triage = json(&dir.join("out-quarantine/triage_report.json"));
    assert_eq!(triage["schema_version"], "0.9");
    assert_eq!(triage["status"], "QUARANTINE_REPAIR_FAILED");
    assert_eq!(triage["exit_code"], 21);
    assert_eq!(triage["repair_telemetry"]["attempts_used"], 3);
    assert_eq!(triage["repair_telemetry"]["max_attempts"], 3);
    assert_eq!(triage["repair_telemetry"]["gate2_stability_passed"], false);
    assert!(
        triage["token_accounting"]["prompt_tokens"]
            .as_u64()
            .unwrap()
            > 0
    );
}

#[test]
fn disallowed_model_slug_and_test_file_patch_are_rejected() {
    let dir = scratch("disallowed-and-anticheat");

    // 1. muse-wonder-full-1.0 is rejected at CLI validation with exit code 2
    let bad_model = Command::new(env!("CARGO_BIN_EXE_replayfix"))
        .current_dir(&dir)
        .args(["run", "--bug", "RF-01", "--model", "muse-wonder-full-1.0"])
        .output()
        .unwrap();
    assert_eq!(code(&bad_model), 2);
    assert!(String::from_utf8_lossy(&bad_model.stderr).contains("MODEL_DISALLOWED"));

    // 2. reasoning effort 'max' is rejected at CLI validation with exit code 2
    let bad_effort = Command::new(env!("CARGO_BIN_EXE_replayfix"))
        .current_dir(&dir)
        .args([
            "run",
            "--bug",
            "RF-01",
            "--model",
            "muse-spark-1.3",
            "--reasoning-effort",
            "max",
        ])
        .output()
        .unwrap();
    assert_eq!(code(&bad_effort), 2);
    assert!(String::from_utf8_lossy(&bad_effort.stderr).contains("EFFORT_MAX_PROHIBITED"));

    // 3. Patch targeting tests/cart.spec.ts is rejected with SECURITY_VIOLATION (exit code 11)
    fs::write(
        dir.join("cheat.diff"),
        "--- a/tests/cart.spec.ts\n+++ b/tests/cart.spec.ts\n@@ -1 +1 @@\n-expect(1).toBe(0);\n+expect(1).toBe(1);\n",
    )
    .unwrap();
    let cheat_out = run(&dir, &["--mock-heal", "--mock-patch", "cheat.diff"]);
    assert_eq!(code(&cheat_out), 11);
    let triage = json(&dir.join(".replayfix/triage_report.json"));
    assert_eq!(triage["status"], "SECURITY_VIOLATION");
    assert!(triage["reason"]
        .as_str()
        .unwrap()
        .contains("TEST_MUTATION_PROHIBITED"));
}

#[test]
fn freeze_spec_subcommand_locks_files_and_generates_spec() {
    let dir = scratch("freeze-spec");
    let out_file = dir.join("specs/RF-01.spec.json");

    let out = Command::new(env!("CARGO_BIN_EXE_replayfix"))
        .current_dir(&dir)
        .args([
            "freeze-spec",
            "--bug",
            "RF-01",
            "--repo",
            ".",
            "--output",
            out_file.to_str().unwrap(),
        ])
        .output()
        .expect("failed to launch replayfix freeze-spec");

    assert_eq!(
        code(&out),
        0,
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    assert!(out_file.exists());
    let spec = json(&out_file);
    assert_eq!(spec["schema_version"], "0.9");
    assert_eq!(spec["bug_id"], "RF-01");
    assert_eq!(spec["aggregate_sha256"].as_str().unwrap().len(), 64);
    assert_eq!(spec["contract_digest"].as_str().unwrap().len(), 64);

    let repro_spec_ts = dir
        .join("tests")
        .join("repro")
        .join("rf_01_regression.spec.ts");
    assert!(repro_spec_ts.exists());
    let ts_content = fs::read_to_string(&repro_spec_ts).unwrap();
    assert!(ts_content.contains("REPLAYFIX-FROZEN-SPEC: RF-01"));
    assert!(ts_content.contains("AGGREGATE-SHA256:"));
    assert!(ts_content.contains("@playwright/test"));
}

#[test]
fn mock_pkg_test_mutation_is_rejected_as_security_violation_exit_11() {
    let dir = scratch("pkg-test-mutation");
    fs::write(
        dir.join("package.json"),
        "{\"scripts\": {\"test\": \"jest\"}}\n",
    )
    .unwrap();
    let out = run(&dir, &["--mock-heal", "--mock-pkg-test-mutation"]);

    assert_eq!(
        code(&out),
        11,
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let triage = json(&dir.join(".replayfix/triage_report.json"));
    assert_eq!(triage["schema_version"], "0.9");
    assert_eq!(triage["status"], "SECURITY_VIOLATION");
    assert_eq!(triage["exit_code"], 11);
}

#[test]
fn repro_run_with_clean_ref_and_verify_matrix() {
    let dir = scratch("matrix-clean-ref");
    let out = run(
        &dir,
        &["--mock-heal", "--clean-ref", "main", "--verify-matrix"],
    );

    assert_eq!(
        code(&out),
        0,
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let receipt = json(&dir.join(".replayfix/audit_receipt.json"));
    assert_eq!(receipt["schema_version"], "0.9");
    assert_eq!(receipt["status"], "VERIFIED_REPAIRED");
    assert_eq!(
        receipt["differential_diagnosis"]["state"],
        "CAUSAL_DEFECT_CONFIRMED"
    );
    assert_eq!(
        receipt["differential_diagnosis"]["causal_confidence"],
        "HIGH"
    );
    assert_eq!(receipt["regression_parity"]["parity_passed"], true);

    // Verify disaggregated artifacts were written
    assert!(dir.join(".replayfix/artifacts/candidate.patch").exists());
    assert!(dir.join(".replayfix/artifacts/01_candidate.diff").exists());
    assert!(dir.join(".replayfix/artifacts/02_preflight.json").exists());
    assert!(dir
        .join(".replayfix/artifacts/differential_diagnosis.json")
        .exists());
    assert!(dir
        .join(".replayfix/artifacts/03_differential_diagnosis.json")
        .exists());
    assert!(dir
        .join(".replayfix/artifacts/regression_parity.json")
        .exists());
    assert!(dir
        .join(".replayfix/artifacts/04_regression_parity.json")
        .exists());
    assert!(dir
        .join(".replayfix/artifacts/04_audit_receipt.json")
        .exists());
}

#[test]
fn mock_regression_triggers_exit_31_and_triage_report() {
    let dir = scratch("regression-detected");
    let out = run(&dir, &["--mock-heal", "--mock-regression"]);

    assert_eq!(
        code(&out),
        31,
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let triage = json(&dir.join(".replayfix/triage_report.json"));
    assert_eq!(triage["schema_version"], "0.9");
    assert_eq!(triage["status"], "REGRESSION_DETECTED");
    assert_eq!(triage["exit_code"], 31);
    assert_eq!(triage["regression_parity"]["parity_passed"], false);
    assert!(!triage["regression_parity"]["new_failures"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[test]
fn test_trace_generates_zip_and_skips_gui_in_non_interactive() {
    let dir = scratch("trace-export");
    let out = run(&dir, &["--mock-heal", "--trace"]);

    assert_eq!(
        code(&out),
        0,
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Trace Export"));
    assert!(stdout.contains("true"));
    let trace_path = dir.join(".replayfix/artifacts/playwright.trace.zip");
    assert!(trace_path.exists());
    let trace_meta = fs::metadata(&trace_path).unwrap();
    assert!(trace_meta.len() > 0);
}

#[test]
fn test_issue_invalid_url_fails_with_exit_23() {
    let dir = scratch("invalid-issue");
    let out = run(
        &dir,
        &["--issue", "https://github.com/not/an/issue/pull/123"],
    );

    assert_eq!(code(&out), 23);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("Invalid --issue URL"));
}

#[test]
fn test_issue_ingestion_and_repro_success() {
    let dir = scratch("issue-ingest-success");
    let out = run(
        &dir,
        &[
            "--issue",
            "https://github.com/mock/repo/issues/101",
            "--bug",
            "RF-01",
            "--mock-heal",
        ],
    );

    assert_eq!(
        code(&out),
        0,
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("GitHub Issue"));
    assert!(stdout.contains("https://github.com/mock/repo/issues/101"));

    let artifact = json(&dir.join(".replayfix/artifacts/issue_ingestion.json"));
    assert_eq!(artifact["issue_ref"]["owner"], "mock");
    assert_eq!(artifact["issue_ref"]["repo"], "repo");
    assert_eq!(artifact["issue_ref"]["issue_number"], 101);
    assert!(!artifact["extracted_steps"].as_array().unwrap().is_empty());
}

#[test]
fn test_apply_in_non_interactive_without_yes_refuses_exit_13() {
    let dir = scratch("apply-refusal");
    let cart_file = dir.join("src/cart-summary.js");
    fs::create_dir_all(cart_file.parent().unwrap()).unwrap();
    fs::write(
        &cart_file,
        b"export function cartItemCount(cart) {\n  return cart.items.length;\n}\n",
    )
    .unwrap();

    let out = run(&dir, &["--mock-heal", "--apply"]);

    assert_eq!(code(&out), 13);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("non-interactive session without --yes flag"));

    let audit = json(&dir.join(".replayfix/artifacts/approval_audit.json"));
    assert_eq!(audit["decision"], "Rejected");
    assert_eq!(audit["mode"], "NonInteractiveRejected");
    assert_eq!(audit["tty_detected"], false);

    // Verify target file was not modified
    let content = fs::read_to_string(cart_file).unwrap();
    assert!(content.contains("return cart.items.length;"));
}

#[test]
fn test_apply_with_yes_approves_and_mutates_base_repo_exit_0() {
    let dir = scratch("apply-confirmed");
    let cart_file = dir.join("src/cart-summary.js");
    fs::create_dir_all(cart_file.parent().unwrap()).unwrap();
    fs::write(
        &cart_file,
        b"export function cartItemCount(cart) {\n  return cart.items.length;\n}\n",
    )
    .unwrap();

    let out = run(&dir, &["--mock-heal", "--apply", "--yes"]);

    assert_eq!(
        code(&out),
        0,
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains(
        "Candidate patch confirmed (AutoBypassFlag) and applied to base repository successfully."
    ));

    let audit = json(&dir.join(".replayfix/artifacts/approval_audit.json"));
    assert_eq!(audit["decision"], "Approved");
    assert_eq!(audit["mode"], "AutoBypassFlag");

    assert!(dir.join(".replayfix/artifacts/candidate.patch").exists());

    // Verify target file WAS successfully patched!
    let content = fs::read_to_string(cart_file).unwrap();
    assert!(content.contains("return cart.items?.length || 0;"));
    assert!(!content.contains("return cart.items.length;"));
}

fn run_cmd(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_replayfix"))
        .current_dir(dir)
        .args(args)
        .output()
        .expect("failed to launch replayfix")
}

#[test]
fn test_merkle_receipt_generated_and_verified_exit_0() {
    let dir = scratch("merkle-verify-ok");
    let out = run(&dir, &["--mock-heal"]);
    assert_eq!(code(&out), 0);

    let artifacts = dir.join(".replayfix/artifacts");
    assert!(artifacts.join("merkle_receipt.json").exists());
    assert!(artifacts.join("attestation.json").exists());

    // Verify via CLI
    let verify_out = run_cmd(&dir, &["verify-receipt", ".replayfix", "--json"]);
    assert_eq!(code(&verify_out), 0);
    let v_json: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&verify_out.stdout)).unwrap();
    assert_eq!(v_json["status"], "SUCCESS");
    assert!(!v_json["public_key"].as_str().unwrap().is_empty());
    assert!(!v_json["merkle_root"].as_str().unwrap().is_empty());
}

#[test]
fn test_verify_receipt_tampered_fails_exit_40() {
    let dir = scratch("merkle-tamper");
    let out = run(&dir, &["--mock-heal"]);
    assert_eq!(code(&out), 0);

    let mr_path = dir.join(".replayfix/artifacts/merkle_receipt.json");
    let mut mr_val: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&mr_path).unwrap()).unwrap();
    // Tamper with root
    mr_val["merkle_root"] =
        serde_json::json!("ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff");
    fs::write(&mr_path, serde_json::to_vec_pretty(&mr_val).unwrap()).unwrap();

    let verify_out = run_cmd(&dir, &["verify-receipt", mr_path.to_str().unwrap()]);
    assert_eq!(code(&verify_out), 40);
}

#[test]
fn test_verify_receipt_pinned_key_mismatch_fails_exit_40() {
    let dir = scratch("merkle-untrusted");
    let out = run(&dir, &["--mock-heal"]);
    assert_eq!(code(&out), 0);

    let mr_path = dir.join(".replayfix/artifacts/merkle_receipt.json");
    let verify_out = run_cmd(
        &dir,
        &[
            "verify-receipt",
            mr_path.to_str().unwrap(),
            "--trusted-key",
            "1111111111111111111111111111111111111111111111111111111111111111",
        ],
    );
    assert_eq!(code(&verify_out), 40);
}

#[test]
fn test_sweep_all_five_bugs_cli() {
    let dir = scratch("sweep-cli");
    let out = run_cmd(
        &dir,
        &[
            "sweep",
            "--mock-heal",
            "--repo",
            dir.to_str().unwrap(),
            "--output",
            ".replayfix",
            "--json",
        ],
    );

    assert_eq!(
        code(&out),
        0,
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let sweep_json: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&out.stdout)).unwrap();
    assert_eq!(sweep_json["schema_version"], "0.9");
    assert_eq!(sweep_json["total_bugs"], 5);
    assert_eq!(sweep_json["repaired_bugs"], 5);
    assert_eq!(sweep_json["failed_bugs"], 0);
    assert!(sweep_json["elapsed_wall_clock_ms"].as_u64().unwrap() < 60_000);

    // Each bug has its own receipt with merkle_root and attestation populated
    for bug in &["RF-01", "RF-02", "RF-03", "RF-04", "RF-05"] {
        let bug_receipt_path = dir.join(format!(".replayfix/{bug}/audit_receipt.json"));
        assert!(bug_receipt_path.exists());
        let bug_rec = json(&bug_receipt_path);
        assert!(bug_rec["merkle_root"].is_string());
        assert!(bug_rec["attestation"].is_object());
    }

    // Verify sweep_receipt.json cryptographically via verify-receipt
    let verify_sweep = run_cmd(
        &dir,
        &["verify-receipt", ".replayfix/sweep_receipt.json", "--json"],
    );
    assert_eq!(
        code(&verify_sweep),
        0,
        "stderr: {}",
        String::from_utf8_lossy(&verify_sweep.stderr)
    );
    let vs_json: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&verify_sweep.stdout)).unwrap();
    assert_eq!(vs_json["status"], "SUCCESS");
    assert_eq!(vs_json["tree_size"], 5);
}

#[test]
fn test_verify_receipt_on_disk_artifact_tampering_fails_exit_40() {
    let dir = scratch("merkle-disk-tamper");
    let out = run(&dir, &["--mock-heal"]);
    assert_eq!(code(&out), 0);

    // Tamper with 01_candidate.diff on disk without modifying merkle_receipt.json
    let diff_path = dir.join(".replayfix/artifacts/01_candidate.diff");
    fs::write(&diff_path, "evil injected backdoor diff").unwrap();

    let verify_out = run_cmd(&dir, &["verify-receipt", ".replayfix/audit_receipt.json"]);
    assert_eq!(code(&verify_out), 40);
    let stderr = String::from_utf8_lossy(&verify_out.stderr);
    assert!(stderr.contains("MERKLE_PROOF_INVALID"));
}

#[test]
fn test_verify_receipt_deleted_merkle_receipt_fails_exit_40() {
    let dir = scratch("merkle-deleted");
    let out = run(&dir, &["--mock-heal"]);
    assert_eq!(code(&out), 0);

    // Remove both merkle_receipt.json files and verify that audit_receipt.json fails closed
    let _ = fs::remove_file(dir.join(".replayfix/artifacts/merkle_receipt.json"));
    let _ = fs::remove_file(dir.join(".replayfix/merkle_receipt.json"));

    let verify_out = run_cmd(&dir, &["verify-receipt", ".replayfix/audit_receipt.json"]);
    assert_eq!(code(&verify_out), 40);
    let stderr = String::from_utf8_lossy(&verify_out.stderr);
    assert!(stderr.contains("MERKLE_PROOF_INVALID"));
}

#[test]
fn test_export_proof_bundle_cli() {
    let dir = scratch("export-proof");
    let out = run_cmd(
        &dir,
        &["export-proof", "--output", ".replayfix/proof", "--json"],
    );
    assert_eq!(
        code(&out),
        0,
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let out_json: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&out.stdout)).unwrap();
    assert_eq!(out_json["status"], "SUCCESS");
    assert!(dir.join(".replayfix/proof/demo_presets.json").exists());
}

#[test]
fn test_version_and_release_package_integrity() {
    let dir = scratch("release-package-version");
    let ver_out = run_cmd(&dir, &["--version"]);
    assert_eq!(code(&ver_out), 0);
    let stdout = String::from_utf8_lossy(&ver_out.stdout);
    let expected_ver = format!("replayfix {}", env!("CARGO_PKG_VERSION"));
    assert!(
        stdout.contains(&expected_ver),
        "Expected '{expected_ver}' in --version output, got: {stdout}"
    );

    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let rel_dir = repo_root.join("release/v1.0.0");
    assert!(rel_dir.join("proof-portal/proof_viewer.html").exists());
    assert!(rel_dir.join("demo/replayfix-v1.0.0-demo.mp4").exists());
    assert!(rel_dir.join("demo/demo-evidence-manifest.json").exists());
    assert!(rel_dir
        .join("demo/captions/replayfix-v1.0.0-demo.vtt")
        .exists());
    assert!(rel_dir
        .join("documentation/HACKATHON_SUBMISSION.md")
        .exists());
    assert!(rel_dir.join("QUALIFICATION_REPORT.md").exists());
    assert!(rel_dir.join("SHA256SUMS.txt").exists());

    // 1. Verify every single entry in release/v1.0.0/SHA256SUMS.txt matches on-disk SHA-256
    let sums_raw = fs::read_to_string(rel_dir.join("SHA256SUMS.txt")).unwrap();
    let mut verified_files = 0usize;
    for line in sums_raw.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.splitn(2, "  ").collect();
        assert_eq!(parts.len(), 2, "Malformed SHA256SUMS line: {line}");
        let expected_sha = parts[0].trim();
        let rel_path = parts[1].trim();
        assert!(
            !rel_path.ends_with(".key"),
            "Private key file must never be packaged in release/v1.0.0: {rel_path}"
        );
        let full_path = rel_dir.join(rel_path);
        assert!(
            full_path.exists(),
            "Missing packaged file: {}",
            full_path.display()
        );
        let actual_sha = replayfix_core::digest::sha256_hex(&fs::read(&full_path).unwrap());
        assert_eq!(
            actual_sha, expected_sha,
            "SHA-256 mismatch on packaged file '{rel_path}'"
        );
        verified_files += 1;
    }
    assert!(
        verified_files >= 20,
        "Expected >= 20 files in SHA256SUMS.txt"
    );

    // 2. Verify binaries/windows-x64/SHA256SUMS.txt matches binaries/windows-x64/replayfix.exe
    let bin_sums = fs::read_to_string(rel_dir.join("binaries/windows-x64/SHA256SUMS.txt")).unwrap();
    let bin_expected_sha = bin_sums.split_whitespace().next().unwrap();
    let bin_actual_sha = replayfix_core::digest::sha256_hex(
        &fs::read(rel_dir.join("binaries/windows-x64/replayfix.exe")).unwrap(),
    );
    assert_eq!(bin_actual_sha, bin_expected_sha);

    // 3. Verify demo-evidence-manifest.json hashes match physical artifacts
    let demo_manifest: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(rel_dir.join("demo/demo-evidence-manifest.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        demo_manifest["production_binary"]["sha256"]
            .as_str()
            .unwrap(),
        bin_actual_sha
    );
    for act in demo_manifest["acts"].as_array().unwrap() {
        for (art_path, expected_hash_val) in act["evidence_artifacts"].as_object().unwrap() {
            let expected_h = expected_hash_val.as_str().unwrap();
            let actual_h =
                replayfix_core::digest::sha256_hex(&fs::read(repo_root.join(art_path)).unwrap());
            assert_eq!(
                actual_h, expected_h,
                "Demo manifest artifact hash mismatch for '{art_path}'"
            );
        }
    }
}

#[test]
fn test_live_ai_auth_guard_and_deterministic_fixture_metapulse_videos() {
    let dir = scratch("execution-modes");
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();

    // 1. --mode live-ai without META_API_KEY / REPLAYFIX_AI_KEY must fail closed with Exit Code 30
    let live_out = Command::new(env!("CARGO_BIN_EXE_replayfix"))
        .current_dir(&dir)
        .env_remove("META_API_KEY")
        .env_remove("REPLAYFIX_AI_KEY")
        .env("REPLAYFIX_TEST_NO_REG", "1")
        .args([
            "run",
            "--bug",
            "RF-01",
            "--mode",
            "live-ai",
            "--model-intent",
            "muse-spark-1.3",
            "--model-repair",
            "muse-wonder-1.0",
        ])
        .output()
        .unwrap();
    assert_eq!(code(&live_out), 30);
    let stderr = String::from_utf8_lossy(&live_out.stderr);
    assert!(stderr.contains("ERR_GATEWAY_AUTH_MISSING"));

    // 2. --mode deterministic-fixture across 3 real MetaPulse videos (RF-01, RF-02, RF-03)
    for (bug_id, video_rel) in [
        (
            "RF-01",
            "benchmarks/metapulse/videos/RF-01_quote_repost_crash_with_cursor.mp4",
        ),
        (
            "RF-02",
            "benchmarks/metapulse/videos/RF-02_stale_follower_increment.mp4",
        ),
        (
            "RF-03",
            "benchmarks/metapulse/videos/RF-03_character_counter_underflow.mp4",
        ),
    ] {
        let video_abs = repo_root.join(video_rel).display().to_string();
        let out_sub = format!(".replayfix/{bug_id}");
        let det_out = run_cmd(
            &dir,
            &[
                "run",
                "--video",
                &video_abs,
                "--repo",
                ".",
                "--mode",
                "deterministic-fixture",
                "--model-intent",
                "muse-spark-1.3",
                "--model-repair",
                "muse-wonder-1.0",
                "--output-dir",
                &out_sub,
            ],
        );
        assert_eq!(
            code(&det_out),
            0,
            "Failed on {bug_id}: {}",
            String::from_utf8_lossy(&det_out.stderr)
        );
        let receipt = json(&dir.join(&out_sub).join("audit_receipt.json"));
        assert_eq!(receipt["task_id"], bug_id);
        assert_eq!(receipt["provenance"], "deterministic-fixture");
        assert!(receipt.get("model_provenance").is_none() || receipt["model_provenance"].is_null());
        let vtel = receipt
            .get("video_telemetry")
            .expect("video_telemetry must be populated for real MetaPulse recording");
        assert!(vtel.get("sam_snap").is_some());
        assert_eq!(vtel["sam_snap"]["model"], "local_diff_heuristic");
        assert!(vtel.get("action_trajectory").is_some());
        let actions = &vtel["action_trajectory"]["actions"];
        assert!(actions.is_array());
        assert!(!actions.as_array().unwrap().is_empty());
        assert_eq!(actions[0]["sam_snap"]["model"], "local_diff_heuristic");
    }

    // 4. End-to-end live-browser verification against RF-01_quote_repost_crash_with_cursor.mp4 (Task 120-10)
    let rf01_mp4 = repo_root
        .join("benchmarks/metapulse/videos/RF-01_quote_repost_crash_with_cursor.mp4")
        .display()
        .to_string();
    let out_live = run_cmd(
        &dir,
        &[
            "run",
            "--video",
            &rf01_mp4,
            "--repo",
            ".",
            "--output-dir",
            ".replayfix/rf01_live_video",
        ],
    );
    assert_eq!(
        code(&out_live),
        0,
        "RF-01 live-browser video verification failed: stdout={} stderr={}",
        String::from_utf8_lossy(&out_live.stdout),
        String::from_utf8_lossy(&out_live.stderr)
    );
    let live_receipt = json(&dir.join(".replayfix/rf01_live_video/audit_receipt.json"));
    assert_eq!(live_receipt["status"], "VERIFIED_REPAIRED");
    assert_eq!(
        live_receipt["video_telemetry"]["sam_snap"]["calibrated_x_css"],
        532
    );
    assert_eq!(
        live_receipt["video_telemetry"]["sam_snap"]["calibrated_y_css"],
        374
    );
    assert_eq!(
        live_receipt["grounding_telemetry"]["matched_selector"],
        "page.getByRole('button', { name: 'Publish Quote', exact: true })"
    );
}

#[test]
fn test_provenance_honest_on_default_run() {
    let dir = scratch("provenance-default");
    let out = run_cmd(&dir, &["run", "--bug", "RF-01"]);
    assert_eq!(
        code(&out),
        0,
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let receipt = json(&dir.join(".replayfix/audit_receipt.json"));
    assert_eq!(receipt["provenance"], "deterministic-fixture");
    assert!(receipt.get("model_provenance").is_none() || receipt["model_provenance"].is_null());
    assert!(receipt.get("token_accounting").is_none() || receipt["token_accounting"].is_null());
}

#[test]
fn test_provenance_rejects_mock_with_live_mode() {
    let dir = scratch("provenance-reject-mock-live");
    let out = run_cmd(
        &dir,
        &[
            "run",
            "--bug",
            "RF-01",
            "--mode",
            "live-ai",
            "--mock-syntax-retry",
        ],
    );
    assert_eq!(
        code(&out),
        2,
        "Expected exit code 2 (ERR_USAGE) when combining --mode live-ai with --mock-syntax-retry, got {}: {}",
        code(&out),
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn test_deterministic_fixture_clears_token_accounting() {
    let dir = scratch("provenance-clear-tokens");
    let out = run_cmd(
        &dir,
        &[
            "run",
            "--bug",
            "RF-01",
            "--mode",
            "deterministic-fixture",
            "--model",
            "muse-wonder-1.0",
            "--output-dir",
            "out-det-tokens",
        ],
    );
    assert_eq!(
        code(&out),
        0,
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let receipt = json(&dir.join("out-det-tokens/audit_receipt.json"));
    assert_eq!(receipt["provenance"], "deterministic-fixture");
    assert!(receipt.get("model_provenance").is_none() || receipt["model_provenance"].is_null());
    assert!(
        receipt.get("token_accounting").is_none() || receipt["token_accounting"].is_null(),
        "token_accounting must be None when provenance == deterministic-fixture"
    );
    assert!(receipt["disclaimer"]
        .as_str()
        .unwrap()
        .contains("Zero live external AI tokens billed"));
}

#[test]
fn test_managed_spa_server_boots_on_port_0_and_cleans_up() {
    let dir = scratch("managed-spa-port-0");
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let metapulse_dir = repo_root.join("benchmarks/metapulse");

    let out_managed = run_cmd(
        &dir,
        &[
            "repro",
            "--video",
            "bug.mp4",
            "--repo",
            &metapulse_dir.display().to_string(),
            "--output-dir",
            "out-managed-spa",
        ],
    );
    assert_eq!(
        code(&out_managed),
        0,
        "Managed SPA mode failed: stdout={}\nstderr={}",
        String::from_utf8_lossy(&out_managed.stdout),
        String::from_utf8_lossy(&out_managed.stderr)
    );
    let managed_receipt = json(&dir.join("out-managed-spa/audit_receipt.json"));
    assert_eq!(managed_receipt["status"], "VERIFIED_REPAIRED");
    assert_eq!(managed_receipt["gate1"]["verdict"], "RED");
    assert_eq!(managed_receipt["gate2"]["verdict"], "GREEN");
    assert_eq!(managed_receipt["provenance"], "deterministic-fixture");
    assert!(
        managed_receipt.get("model_provenance").is_none()
            || managed_receipt["model_provenance"].is_null()
    );
    assert!(
        managed_receipt.get("token_accounting").is_none()
            || managed_receipt["token_accounting"].is_null()
    );
    let managed_port = managed_receipt["ephemeral_port"].as_u64().unwrap() as u16;
    assert!(managed_port > 0);
    // Child dev server must be terminated after run completes.
    assert!(!replayfix_core::probe_http_ready(
        "127.0.0.1",
        managed_port,
        "/"
    ));
}

#[test]
fn test_attached_spa_mode_connects_to_external_url() {
    let dir = scratch("attached-spa-mode");
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let metapulse_dir = repo_root.join("benchmarks/metapulse");

    let target_app = dir.join("target-app");
    fs::create_dir_all(target_app.join("public")).unwrap();
    fs::copy(
        metapulse_dir.join("package.json"),
        target_app.join("package.json"),
    )
    .unwrap();
    fs::copy(
        metapulse_dir.join("server.js"),
        target_app.join("server.js"),
    )
    .unwrap();
    fs::copy(
        metapulse_dir.join("public/index.html"),
        target_app.join("public/index.html"),
    )
    .unwrap();
    fs::copy(
        metapulse_dir.join("public/app.js"),
        target_app.join("public/app.js"),
    )
    .unwrap();
    let initial_app_js = fs::read_to_string(target_app.join("public/app.js")).unwrap();

    let external_server = replayfix_core::ManagedSpaServer::spawn(&target_app, None).unwrap();
    let attached_url = external_server.base_url();
    let attached_port = external_server.port();

    // Ensure the external SPA server has fully hydrated both the root HTML shell and /public/app.js
    // before dispatching the attached browser reproduction run.
    let hydration_deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
    let mut dom_hydrated = false;
    while std::time::Instant::now() < hydration_deadline {
        if replayfix_core::probe_http_ready("127.0.0.1", attached_port, "/")
            && replayfix_core::probe_http_ready("127.0.0.1", attached_port, "/public/app.js")
        {
            dom_hydrated = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    assert!(
        dom_hydrated,
        "Attached SPA server failed DOM hydration readiness checks on port {attached_port}"
    );

    // 1. Attached Mode without --apply preserves \Delta repo = 0 (Shadow Scratch First)
    let out_attached = run_cmd(
        &dir,
        &[
            "repro",
            "--video",
            "bug.mp4",
            "--url",
            &attached_url,
            "--repo",
            "target-app",
            "--output-dir",
            "out-attached-spa",
        ],
    );
    assert_eq!(
        code(&out_attached),
        0,
        "Attached SPA mode failed: stdout={}\nstderr={}",
        String::from_utf8_lossy(&out_attached.stdout),
        String::from_utf8_lossy(&out_attached.stderr)
    );
    let attached_receipt = json(&dir.join("out-attached-spa/audit_receipt.json"));
    assert_eq!(attached_receipt["status"], "VERIFIED_REPAIRED");
    assert_eq!(attached_receipt["substrate"], "attached-spa-remote");
    assert_eq!(
        attached_receipt["ephemeral_port"].as_u64().unwrap(),
        u64::from(attached_port)
    );
    assert_eq!(attached_receipt["gate1"]["verdict"], "RED");
    assert_eq!(attached_receipt["gate2"]["verdict"], "GREEN");
    assert_eq!(attached_receipt["provenance"], "deterministic-fixture");
    assert_eq!(
        fs::read_to_string(target_app.join("public/app.js")).unwrap(),
        initial_app_js,
        "Shadow Scratch First: --repo must remain unmodified when --apply is not passed"
    );

    // 2. Attached Mode with --apply --yes applies verified patch to --repo
    let out_apply = run_cmd(
        &dir,
        &[
            "repro",
            "--video",
            "bug.mp4",
            "--url",
            &attached_url,
            "--repo",
            "target-app",
            "--apply",
            "--yes",
            "--output-dir",
            "out-attached-apply",
        ],
    );
    assert_eq!(code(&out_apply), 0);
    let patched_app_js = fs::read_to_string(target_app.join("public/app.js")).unwrap();
    assert!(
        patched_app_js.contains("let session = { currentUser: { profileId: 'default' } };"),
        "--apply --yes must apply verified patch to --repo target-app/public/app.js"
    );
}

#[test]
fn test_heal_macro_subcommand_deterministic_rf01() {
    let dir = scratch("heal-macro-rf01");
    let out = run_cmd(
        &dir,
        &[
            "heal", "--bug", "RF-01", "--repo", ".", "--output", "out-heal", "--json",
        ],
    );
    assert_eq!(
        code(&out),
        0,
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    let parsed: serde_json::Value = serde_json::from_slice(&out.stdout).expect("valid json output");
    assert_eq!(parsed["status"], "VERIFIED_REPAIRED");
    assert_eq!(parsed["task_id"], "RF-01");
    assert_eq!(parsed["sla_compliant_sub_60s"], true);
    assert!(parsed["deliberation_turns"].as_u64().unwrap() >= 4);

    let receipt_path = dir.join("out-heal/audit_receipt.json");
    assert!(receipt_path.exists());
    let receipt = json(&receipt_path);
    assert_eq!(receipt["status"], "VERIFIED_REPAIRED");
    assert!(receipt["merkle_root"].is_string());
    assert_eq!(receipt["base_repo_unmodified_verified"], true);
    assert_eq!(receipt["base_tree_sha256"].as_str().unwrap().len(), 64);
    assert_eq!(receipt["sandbox_tree_sha256"].as_str().unwrap().len(), 64);
    assert_ne!(receipt["base_tree_sha256"], receipt["sandbox_tree_sha256"]);
    assert!(receipt["deliberation"].is_object());
    assert_eq!(
        receipt["deliberation"]["prompt_cache_key"],
        "replayfix-v1.6-spark-agent-loop"
    );
    assert!(dir.join("tests/repro/rf_01.spec.ts").exists());
}

#[test]
fn test_heal_macro_subcommand_syntax_retry_backtracking() {
    let dir = scratch("heal-macro-retry");
    let out = run_cmd(
        &dir,
        &[
            "heal",
            "--bug",
            "RF-01",
            "--mode",
            "syntax-retry",
            "--repo",
            ".",
            "--output",
            "out-heal-retry",
            "--json",
        ],
    );
    assert_eq!(
        code(&out),
        0,
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    let parsed: serde_json::Value = serde_json::from_slice(&out.stdout).expect("valid json output");
    assert_eq!(parsed["lineage_attempts"], 2);

    let receipt = json(&dir.join("out-heal-retry/audit_receipt.json"));
    let turns = receipt["deliberation"]["turns"].as_array().unwrap();
    assert!(turns.len() >= 5);
}

#[test]
fn test_heal_macro_subcommand_flaky_retry_and_quarantine() {
    let dir_flaky_retry = scratch("heal-macro-flaky-retry");
    let out_retry = run_cmd(
        &dir_flaky_retry,
        &[
            "heal",
            "--bug",
            "RF-01",
            "--mode",
            "flaky-retry",
            "--repo",
            ".",
            "--output",
            "out-flaky-retry",
            "--json",
        ],
    );
    assert_eq!(code(&out_retry), 0);
    let parsed: serde_json::Value =
        serde_json::from_slice(&out_retry.stdout).expect("valid json output");
    assert_eq!(parsed["lineage_attempts"], 2);

    // Exhausted attempts must fail closed with Exit Code 21 and triage_report.json
    let dir_exhausted = scratch("heal-macro-exhausted");
    let out_exhausted = run_cmd(
        &dir_exhausted,
        &[
            "heal",
            "--bug",
            "RF-01",
            "--mode",
            "exhausted",
            "--repo",
            ".",
            "--output",
            "out-exhausted",
        ],
    );
    assert_eq!(code(&out_exhausted), 21);
    let triage_path = dir_exhausted.join("out-exhausted/triage_report.json");
    assert!(triage_path.exists());
    let triage = json(&triage_path);
    assert_eq!(triage["status"], "QUARANTINE_REPAIR_FAILED");
}

#[test]
fn test_heal_macro_subcommand_package_pr() {
    let dir = scratch("heal-macro-pr");
    let out = run_cmd(
        &dir,
        &[
            "heal",
            "--bug",
            "RF-01",
            "--repo",
            ".",
            "--output",
            "out-heal-pr",
            "--package-pr",
        ],
    );
    assert_eq!(code(&out), 0);
    assert!(dir.join("out-heal-pr/pr_description.md").exists());
    assert!(dir.join("out-heal-pr/merkle_receipt.json").exists());
}
