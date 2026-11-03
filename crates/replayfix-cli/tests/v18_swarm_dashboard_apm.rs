//! Release 1.8.0 End-to-End Integration Test Battery:
//! 1. Production APM & RRWeb Session Replay Ingestion (`replayfix ingest`)
//! 2. Distributed Work-Stealing Triage Swarm (`replayfix swarm`)
//! 3. Live Streaming Observability Dashboard (`replayfix dashboard`)
//! 4. Cross-Browser & Multi-Device Inoculation Matrix (`replayfix inoculate --matrix`)
//! 5. RFC 6962 Merkle Receipt Transparency Log & Key Notary (`replayfix notary`)

use replayfix_core::fixtures::Fixture;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("replayfix-v18-{}-{}", name, std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn init_test_git_repo(path: &Path) {
    let _ = Command::new("git")
        .args(["init"])
        .current_dir(path)
        .output();
    let _ = Command::new("git")
        .args(["config", "user.name", "Test Engineer"])
        .current_dir(path)
        .output();
    let _ = Command::new("git")
        .args(["config", "user.email", "test@replayfix.dev"])
        .current_dir(path)
        .output();

    for bug in ["RF-01", "RF-02"] {
        if let Some(fixture) = Fixture::for_task(bug) {
            let target_file = path.join(fixture.target_path);
            if let Some(p) = target_file.parent() {
                let _ = fs::create_dir_all(p);
            }
            let _ = fs::write(&target_file, fixture.buggy_source);
        }
    }

    let pkg_file = path.join("package.json");
    let _ = fs::write(
        &pkg_file,
        r#"{"name":"v18-test-repo","version":"1.8.0","scripts":{"test":"echo ok"}}"#,
    );

    let _ = Command::new("git")
        .args(["add", "."])
        .current_dir(path)
        .output();
    let _ = Command::new("git")
        .args(["commit", "-m", "chore: initial v18 commit"])
        .current_dir(path)
        .output();
}

fn run_cli(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_replayfix"))
        .args(args)
        .output()
        .expect("failed to execute replayfix binary")
}

#[test]
fn test_apm_ingest_rrweb_and_sentry_synthesizes_manifest() {
    let temp = scratch("ingest-apm");
    let repo = temp.join("repo");
    fs::create_dir_all(&repo).unwrap();
    init_test_git_repo(&repo);

    let rrweb_file = temp.join("session.json");
    let rrweb_content = serde_json::json!([
        {
            "type": 4,
            "data": {
                "href": "http://localhost:3000/cart",
                "width": 1440,
                "height": 900
            },
            "timestamp": 100
        },
        {
            "type": 3,
            "data": {
                "source": 2,
                "type": 2,
                "id": 14,
                "x": 320,
                "y": 480
            },
            "timestamp": 250
        }
    ]);
    fs::write(&rrweb_file, rrweb_content.to_string()).unwrap();

    let sentry_file = temp.join("crash.json");
    let sentry_content = serde_json::json!({
        "event_id": "c1a2b3c4d5e6f708192a3b4c5d6e7f80",
        "timestamp": "2026-11-02T12:00:00Z",
        "level": "error",
        "title": "TypeError: Cannot read properties of undefined (reading 'cart_total')",
        "culprit": "src/Cart.tsx in calculateTotal",
        "exception": {
            "values": [
                {
                    "type": "TypeError",
                    "value": "Cannot read properties of undefined (reading 'cart_total')",
                    "stacktrace": {
                        "frames": [
                            {
                                "filename": "src/Cart.tsx",
                                "function": "calculateTotal",
                                "lineno": 42
                            }
                        ]
                    }
                }
            ]
        }
    });
    fs::write(&sentry_file, sentry_content.to_string()).unwrap();

    let out_manifest = temp.join("synthesized_manifest.json");

    let out = run_cli(&[
        "ingest",
        "--rrweb",
        rrweb_file.to_str().unwrap(),
        "--sentry",
        sentry_file.to_str().unwrap(),
        "--repo",
        repo.to_str().unwrap(),
        "--output",
        out_manifest.to_str().unwrap(),
        "--json",
    ]);

    assert_eq!(
        out.status.code().unwrap_or(-1),
        0,
        "CLI stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out_manifest.exists(), "Synthesized manifest must exist");

    let manifest_raw = fs::read_to_string(&out_manifest).unwrap();
    let manifest_val: serde_json::Value = serde_json::from_str(&manifest_raw).unwrap();
    assert_eq!(manifest_val["clean_ref"], "main");
    assert!(manifest_val["task_id"]
        .as_str()
        .unwrap()
        .starts_with("APM-"));

    let _ = fs::remove_dir_all(&temp);
}

#[test]
fn test_swarm_distributed_work_stealing_mock_heal_cluster() {
    let temp = scratch("swarm-cluster");
    let repo = temp.join("repo");
    fs::create_dir_all(&repo).unwrap();
    init_test_git_repo(&repo);

    let artifacts = temp.join("artifacts");

    let out = run_cli(&[
        "swarm",
        "--workers",
        "2",
        "--bugs",
        "RF-01,RF-02",
        "--mock-heal",
        "--repo",
        repo.to_str().unwrap(),
        "--output",
        artifacts.to_str().unwrap(),
        "--json",
    ]);

    assert_eq!(
        out.status.code().unwrap_or(-1),
        0,
        "CLI stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    let summary_file = artifacts.join("swarm_summary.json");
    assert!(summary_file.exists(), "Swarm summary JSON must exist");

    let summary_val: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&summary_file).unwrap()).unwrap();
    assert_eq!(summary_val["total_tasks"], 2);
    assert_eq!(summary_val["completed_tasks"], 2);
    assert_eq!(summary_val["failed_tasks"], 0);

    // Verify delta repo == 0
    let status_out = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(&repo)
        .output()
        .unwrap();
    assert!(
        status_out.stdout.is_empty(),
        "Base repository must retain delta repo = 0"
    );

    let _ = fs::remove_dir_all(&temp);
}

#[test]
fn test_dashboard_server_cli_lifecycle_and_api() {
    let temp = scratch("dashboard-cli");
    let artifacts = temp.join("artifacts");
    fs::create_dir_all(&artifacts).unwrap();

    let out = run_cli(&[
        "dashboard",
        "--port",
        "0",
        "--duration",
        "1",
        "--artifacts",
        artifacts.to_str().unwrap(),
        "--json",
    ]);

    assert_eq!(
        out.status.code().unwrap_or(-1),
        0,
        "CLI stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout_str = String::from_utf8_lossy(&out.stdout);
    let json_val: serde_json::Value = serde_json::from_str(&stdout_str).unwrap();
    assert_eq!(json_val["status"], "ONLINE");
    assert!(json_val["port"].as_u64().unwrap() > 0);
    assert!(json_val["url"]
        .as_str()
        .unwrap()
        .starts_with("http://127.0.0.1:"));
    assert!(json_val["websocket_endpoint"]
        .as_str()
        .unwrap()
        .starts_with("ws://127.0.0.1:"));
    assert!(json_val["sse_endpoint"]
        .as_str()
        .unwrap()
        .starts_with("http://127.0.0.1:"));

    let _ = fs::remove_dir_all(&temp);
}

#[test]
fn test_inoculate_cross_browser_multi_device_matrix() {
    let temp = scratch("inoculate-matrix");
    let repo = temp.join("repo");
    fs::create_dir_all(&repo).unwrap();
    init_test_git_repo(&repo);

    let out_dir = temp.join("out");

    let out = run_cli(&[
        "inoculate",
        "--bug",
        "RF-01",
        "--repo",
        repo.to_str().unwrap(),
        "--output",
        out_dir.to_str().unwrap(),
        "--matrix",
        "chromium,firefox,webkit,mobile,desktop",
        "--json",
    ]);

    assert_eq!(
        out.status.code().unwrap_or(-1),
        0,
        "CLI stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    let matrix_spec_path = repo.join("tests/repro/rf_rf_01_matrix.spec.ts");
    assert!(
        matrix_spec_path.exists(),
        "Matrix specification file must be synthesized in workspace"
    );

    let spec_content = fs::read_to_string(&matrix_spec_path).unwrap();
    assert!(spec_content.contains("// REPLAYFIX-INOCULATION-MATRIX: v1.8"));
    assert!(spec_content.contains("const VIEWPORT_PRESETS"));
    assert!(spec_content.contains("Desktop Wide"));
    assert!(spec_content.contains("Mobile Portrait (iPhone 14)"));
    assert!(spec_content.contains("deviceScaleFactor: vp.dpr"));

    let inoc_ledger = out_dir.join("inoculations.json");
    assert!(inoc_ledger.exists(), "Inoculation ledger must be updated");

    let _ = fs::remove_dir_all(&temp);
}

#[test]
fn test_notary_append_and_verify_log_consistency() {
    let temp = scratch("notary-log");
    let log_file = temp.join("transparency_log.json");

    // Create a mock receipt
    let receipt1_file = temp.join("receipt1.json");
    let receipt1_content = serde_json::json!({
        "schema_version": "0.9",
        "status": "VERIFIED_REPAIRED",
        "task_id": "RF-01",
        "substrate": "test",
        "disclaimer": "test disclaimer",
        "contract_id": "RF-01",
        "contract_sha256": "0000000000000000000000000000000000000000000000000000000000000000",
        "gate1": {
            "observed_at": "2026-11-02T12:00:01Z",
            "verdict": "RED",
            "contract_sha256": "0000000000000000000000000000000000000000000000000000000000000000",
            "events": []
        },
        "patch": {
            "target_path": "src/Cart.tsx",
            "sha256": "0000000000000000000000000000000000000000000000000000000000000000",
            "insertions": 5,
            "deletions": 1,
            "files_changed": 1
        },
        "gate2": {
            "observed_at": "2026-11-02T12:00:03Z",
            "verdict": "GREEN",
            "contract_sha256": "0000000000000000000000000000000000000000000000000000000000000000",
            "events": []
        },
        "phase_trace": ["INIT"],
        "base_tree_sha256": "0000000000000000000000000000000000000000000000000000000000000000",
        "sandbox_tree_sha256": "0000000000000000000000000000000000000000000000000000000000000000",
        "base_repo_unmodified_verified": true,
        "ephemeral_port": 3000,
        "created_at": "2026-11-02T12:00:00Z"
    });
    fs::write(&receipt1_file, receipt1_content.to_string()).unwrap();

    // 1. Append receipt 1
    let out1 = run_cli(&[
        "notary",
        "append",
        "--receipt",
        receipt1_file.to_str().unwrap(),
        "--log",
        log_file.to_str().unwrap(),
        "--json",
    ]);
    assert_eq!(
        out1.status.code().unwrap_or(-1),
        0,
        "Notary append 1 failed: {}",
        String::from_utf8_lossy(&out1.stderr)
    );

    let json_val1: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&out1.stdout)).unwrap();
    assert_eq!(json_val1["tree_size"], 1);

    // 2. Append receipt 2
    let receipt2_file = temp.join("receipt2.json");
    let mut receipt2_content = receipt1_content.clone();
    receipt2_content["task_id"] = serde_json::json!("RF-02");
    fs::write(&receipt2_file, receipt2_content.to_string()).unwrap();

    let out2 = run_cli(&[
        "notary",
        "append",
        "--receipt",
        receipt2_file.to_str().unwrap(),
        "--log",
        log_file.to_str().unwrap(),
        "--json",
    ]);
    assert_eq!(
        out2.status.code().unwrap_or(-1),
        0,
        "Notary append 2 failed: {}",
        String::from_utf8_lossy(&out2.stderr)
    );

    let json_val2: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&out2.stdout)).unwrap();
    assert_eq!(json_val2["tree_size"], 2);

    // 3. Verify log
    let verify_out = run_cli(&[
        "notary",
        "verify-log",
        "--log",
        log_file.to_str().unwrap(),
        "--json",
    ]);
    assert_eq!(
        verify_out.status.code().unwrap_or(-1),
        0,
        "Notary verify log failed: {}",
        String::from_utf8_lossy(&verify_out.stderr)
    );
    let verify_val: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&verify_out.stdout)).unwrap();
    assert_eq!(verify_val["valid"], true);
    assert_eq!(verify_val["tree_size"], 2);

    // 4. Verify consistency proof between size 1 and size 2
    let proof_out = run_cli(&[
        "notary",
        "consistency-proof",
        "--first",
        "1",
        "--second",
        "2",
        "--log",
        log_file.to_str().unwrap(),
        "--json",
    ]);
    assert_eq!(
        proof_out.status.code().unwrap_or(-1),
        0,
        "Notary consistency proof failed: {}",
        String::from_utf8_lossy(&proof_out.stderr)
    );
    let proof_val: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&proof_out.stdout)).unwrap();
    assert_eq!(proof_val["verified"], true);
    assert!(proof_val["first_root"].as_str().unwrap().len() == 64);
    assert!(proof_val["second_root"].as_str().unwrap().len() == 64);

    // 5. Tamper with transparency_log.json and verify fail-closed exit code 40 even with --json
    let mut tampered: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&log_file).unwrap()).unwrap();
    tampered["entries"][0]["task_id"] = serde_json::json!("TAMPERED-TASK");
    fs::write(&log_file, serde_json::to_string_pretty(&tampered).unwrap()).unwrap();

    let tampered_out = run_cli(&[
        "notary",
        "verify-log",
        "--log",
        log_file.to_str().unwrap(),
        "--json",
    ]);
    assert_eq!(
        tampered_out.status.code().unwrap_or(-1),
        40,
        "Tampered log must fail closed with exit code 40"
    );
    let tampered_val: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&tampered_out.stdout)).unwrap();
    assert_eq!(tampered_val["valid"], false);
    assert_eq!(tampered_val["status"], "INTEGRITY_VIOLATION");

    let _ = fs::remove_dir_all(&temp);
}

#[test]
fn test_ingest_rejects_sentry_path_traversal_with_exit_11() {
    let temp = scratch("ingest-traversal");
    let repo = temp.join("repo");
    fs::create_dir_all(&repo).unwrap();
    init_test_git_repo(&repo);

    let sentry_file = temp.join("malicious_crash.json");
    let sentry_content = serde_json::json!({
        "event_id": "bad1234567890",
        "title": "Error: traversal",
        "exception": {
            "values": [
                {
                    "type": "Error",
                    "value": "traversal",
                    "stacktrace": {
                        "frames": [
                            {
                                "filename": "../../../etc/passwd",
                                "function": "exploit",
                                "lineno": 1
                            }
                        ]
                    }
                }
            ]
        }
    });
    fs::write(&sentry_file, sentry_content.to_string()).unwrap();

    let out = run_cli(&[
        "ingest",
        "--sentry",
        sentry_file.to_str().unwrap(),
        "--repo",
        repo.to_str().unwrap(),
    ]);

    assert_eq!(
        out.status.code().unwrap_or(-1),
        11,
        "Path traversal in Sentry stack frame must fail closed with exit code 11 (SECURITY_VIOLATION)"
    );

    let _ = fs::remove_dir_all(&temp);
}

#[test]
fn test_inoculate_matrix_includes_pixel7_and_calibrated_dpr_coordinates() {
    let temp = scratch("inoculate-pixel7");
    let repo = temp.join("repo");
    fs::create_dir_all(&repo).unwrap();
    init_test_git_repo(&repo);

    let out_dir = temp.join("out");

    let out = run_cli(&[
        "inoculate",
        "--bug",
        "RF-01",
        "--repo",
        repo.to_str().unwrap(),
        "--output",
        out_dir.to_str().unwrap(),
        "--matrix",
        "chromium,mobile,pixel7,tablet",
        "--json",
    ]);

    assert_eq!(out.status.code().unwrap_or(-1), 0);

    let matrix_spec_path = repo.join("tests/repro/rf_rf_01_matrix.spec.ts");
    let spec_content = fs::read_to_string(&matrix_spec_path).unwrap();
    assert!(spec_content.contains("Mobile Portrait (iPhone 14)"));
    assert!(spec_content.contains("Mobile Portrait (Pixel 7)"));
    assert!(spec_content.contains("dpr: 2.625"));
    assert!(spec_content.contains("calibratedX:"));
    assert!(spec_content.contains("calibratedY:"));

    let _ = fs::remove_dir_all(&temp);
}
