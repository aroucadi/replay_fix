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
    out.status
        .code()
        .expect("process was terminated by a signal")
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
    assert_eq!(receipt["schema_version"], "0.6");
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
    assert!(receipt["disclaimer"]
        .as_str()
        .unwrap()
        .contains("No browser"));
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
    assert_eq!(receipt["schema_version"], "0.6");
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
    assert_eq!(triage["schema_version"], "0.6");
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
    assert_eq!(triage["schema_version"], "0.6");
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
    assert_eq!(receipt["schema_version"], "0.6");
    assert_eq!(receipt["status"], "VERIFIED_REPAIRED");
    assert_eq!(receipt["substrate"], "playwright-cdp");
    assert_eq!(receipt["video_telemetry"]["sam_snap"]["model"], "sam-3.1");
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
        "muse-voice-transcribe-1.0"
    );
    let keyframes = receipt["video_telemetry"]["keyframe_paths"]
        .as_array()
        .expect("keyframe_paths array");
    assert_eq!(keyframes.len(), 1);
    assert!(dir.join(".replayfix/keyframes").exists());
}

#[test]
fn model_autonomous_repair_and_syntax_retry_write_v06_receipt() {
    let dir = scratch("model-autonomous-v06");
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
    assert_eq!(receipt["schema_version"], "0.6");
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
        receipt["token_accounting"]["total_cost_micros_usd"]
            .as_u64()
            .unwrap()
            > 0
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
    assert_eq!(triage["schema_version"], "0.6");
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
