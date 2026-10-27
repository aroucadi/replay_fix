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
    assert!(receipt["disclaimer"]
        .as_str()
        .unwrap()
        .contains("No browser"));
    assert!(!dir.join(".replayfix/triage_report.json").exists());
}

#[test]
fn noop_patch_is_rejected_with_triage_report() {
    let dir = scratch("noop");
    let out = run(&dir, &["--mock-heal", "--mock-patch", "noop"]);
    assert_eq!(code(&out), 30);

    let triage = json(&dir.join(".replayfix/triage_report.json"));
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
