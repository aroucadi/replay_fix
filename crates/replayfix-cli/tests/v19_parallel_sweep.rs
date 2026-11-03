use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("replayfix-v19-sweep-{}", name));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_replayfix"))
}

#[test]
fn test_sweep_parallel_swarm_generates_valid_telemetry_and_receipt() {
    let dir = scratch("parallel-sweep");

    // 1. Create a synthetic 3-bug benchmark suite in MANIFEST.json
    let bug1_dir = dir.join("bug1");
    let bug2_dir = dir.join("bug2");
    let bug3_dir = dir.join("bug3");
    fs::create_dir_all(&bug1_dir).unwrap();
    fs::create_dir_all(&bug2_dir).unwrap();
    fs::create_dir_all(&bug3_dir).unwrap();

    let manifest_json = r#"{
  "benchmark_suite": "MetaPulse-20",
  "version": "1.9.0",
  "tasks": [
    {
      "task_id": "RF-01",
      "title": "Bug 1",
      "repo_path": "bug1",
      "bug_ref": "bug/RF-01",
      "video_path": "bug1.mp4"
    },
    {
      "task_id": "RF-02",
      "title": "Bug 2",
      "repo_path": "bug2",
      "bug_ref": "bug/RF-02",
      "video_path": "bug2.mp4"
    },
    {
      "task_id": "RF-03",
      "title": "Bug 3",
      "repo_path": "bug3",
      "bug_ref": "bug/RF-03",
      "video_path": "bug3.mp4"
    }
  ]
}"#;

    let manifest_path = dir.join("MANIFEST.json");
    fs::write(&manifest_path, manifest_json).unwrap();

    let output_dir = dir.join("sweep_out");

    // Run `replayfix sweep --dataset MANIFEST.json --parallel 2 --output sweep_out --json`
    let out = Command::new(bin())
        .current_dir(&dir)
        .args([
            "sweep",
            "--dataset",
            manifest_path.to_str().unwrap(),
            "--parallel",
            "2",
            "--output",
            output_dir.to_str().unwrap(),
            "--json",
        ])
        .output()
        .unwrap();

    assert_eq!(
        out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let summary_file = output_dir.join("benchmark_summary.json");
    let telemetry_file = output_dir.join("telemetry_matrix.json");

    assert!(summary_file.exists(), "benchmark_summary.json must exist");
    assert!(telemetry_file.exists(), "telemetry_matrix.json must exist");

    let summary_content = fs::read_to_string(&summary_file).unwrap();
    let summary: serde_json::Value = serde_json::from_str(&summary_content).unwrap();

    assert_eq!(summary["schema_version"], "1.9.0");
    assert_eq!(summary["total_bugs"], 3);
    assert_eq!(summary["success_count"], 3);
    assert_eq!(summary["failure_count"], 0);
    assert!(summary["batch_merkle_root"].as_str().is_some());
    assert!(summary["ed25519_signature"].as_str().is_some());

    let telemetry_content = fs::read_to_string(&telemetry_file).unwrap();
    let telemetry: serde_json::Value = serde_json::from_str(&telemetry_content).unwrap();

    assert_eq!(telemetry["schema_version"], "1.9.0");
    assert_eq!(summary["suite_name"], "MetaPulse-20");
    assert_eq!(telemetry["parallel_workers"], 2);

    // Ensure Δrepo = 0 on base directory
    let repo_dir = Path::new(&dir);
    assert!(!repo_dir.join(".git").exists());
}
