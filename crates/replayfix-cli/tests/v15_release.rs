use serde_json::json;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("replayfix-v15-{}-{}", name, std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn test_mcp_stdio_initialize_tools_resources_and_error_handling() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_replayfix"))
        .args(["mcp", "--transport", "stdio"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn replayfix mcp");

    let mut stdin = child.stdin.take().expect("stdin piped");
    let stdout = child.stdout.take().expect("stdout piped");
    let mut reader = BufReader::new(stdout);

    // 1. Initialize
    let init_req = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "test-client",
                "version": "1.0.0"
            }
        }
    });
    writeln!(&mut stdin, "{}", serde_json::to_string(&init_req).unwrap()).unwrap();
    stdin.flush().unwrap();

    let mut line = String::new();
    reader.read_line(&mut line).unwrap();
    let init_resp: serde_json::Value = serde_json::from_str(&line).expect("valid JSON response");
    assert_eq!(init_resp["id"], 1);
    assert_eq!(
        init_resp["result"]["serverInfo"]["name"],
        "replayfix-mcp-server"
    );

    // 2. tools/list
    line.clear();
    let tools_req = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list",
        "params": {}
    });
    writeln!(stdin, "{}", serde_json::to_string(&tools_req).unwrap()).unwrap();
    stdin.flush().unwrap();

    reader.read_line(&mut line).unwrap();
    let tools_resp: serde_json::Value = serde_json::from_str(&line).expect("valid tools response");
    let tools = tools_resp["result"]["tools"]
        .as_array()
        .expect("tools array");
    let tool_names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert!(tool_names.contains(&"replayfix_reproduce"));
    assert!(tool_names.contains(&"replayfix_verify_patch"));
    assert!(tool_names.contains(&"replayfix_inoculate"));
    assert!(tool_names.contains(&"replayfix_inspect_trace"));
    assert!(tool_names.contains(&"replayfix_doctor"));
    assert_eq!(tools.len(), 5);

    // 3. resources/list and resources/templates/list
    line.clear();
    let res_req = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "resources/list",
        "params": {}
    });
    writeln!(stdin, "{}", serde_json::to_string(&res_req).unwrap()).unwrap();
    stdin.flush().unwrap();

    reader.read_line(&mut line).unwrap();
    let res_resp: serde_json::Value =
        serde_json::from_str(&line).expect("valid resources response");
    let resources = res_resp["result"]["resources"]
        .as_array()
        .expect("resources array");
    assert_eq!(resources.len(), 1);

    line.clear();
    let tmpl_req = json!({
        "jsonrpc": "2.0",
        "id": 31,
        "method": "resources/templates/list",
        "params": {}
    });
    writeln!(stdin, "{}", serde_json::to_string(&tmpl_req).unwrap()).unwrap();
    stdin.flush().unwrap();

    reader.read_line(&mut line).unwrap();
    let tmpl_resp: serde_json::Value =
        serde_json::from_str(&line).expect("valid templates response");
    let templates = tmpl_resp["result"]["resourceTemplates"]
        .as_array()
        .expect("templates array");
    assert_eq!(templates.len(), 2);

    // 4. prompts/get for triage_bug
    line.clear();
    let prompt_req = json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "prompts/get",
        "params": {
            "name": "triage_bug",
            "arguments": {
                "bug_id": "RF-01"
            }
        }
    });
    writeln!(stdin, "{}", serde_json::to_string(&prompt_req).unwrap()).unwrap();
    stdin.flush().unwrap();

    reader.read_line(&mut line).unwrap();
    let prompt_resp: serde_json::Value =
        serde_json::from_str(&line).expect("valid prompt response");
    assert_eq!(prompt_resp["id"], 4);
    let messages = prompt_resp["result"]["messages"]
        .as_array()
        .expect("messages array");
    assert!(!messages.is_empty());

    // 5. Malformed payload error handling (-32600 or -32700)
    line.clear();
    writeln!(stdin, "{{ malformed json !!").unwrap();
    stdin.flush().unwrap();

    reader.read_line(&mut line).unwrap();
    let err_resp: serde_json::Value =
        serde_json::from_str(&line).expect("valid error json response");
    assert!(err_resp["error"]["code"].as_i64().is_some());

    // 6. Unknown method handling (-32601)
    line.clear();
    let unknown_req = json!({
        "jsonrpc": "2.0",
        "id": 5,
        "method": "unknown/nonexistent_method",
        "params": {}
    });
    writeln!(stdin, "{}", serde_json::to_string(&unknown_req).unwrap()).unwrap();
    stdin.flush().unwrap();

    reader.read_line(&mut line).unwrap();
    let unk_resp: serde_json::Value = serde_json::from_str(&line).expect("valid unk json response");
    assert_eq!(unk_resp["error"]["code"], -32601);

    // Clean shutdown
    drop(stdin);
    let status = child.wait().expect("child finished");
    assert!(status.success());
}

#[test]
fn test_permanent_inoculation_dreaming_synthesis_and_registry() {
    let dir = scratch("inoculate-test");

    let status = Command::new(env!("CARGO_BIN_EXE_replayfix"))
        .args([
            "inoculate",
            "--bug",
            "RF-01",
            "--output",
            &dir.display().to_string(),
        ])
        .status()
        .expect("run inoculate");
    assert!(status.success());

    // 1. Verify tests/repro/rf_01.spec.ts was generated with resilient locators
    let spec_file = dir.join("tests/repro/rf_01.spec.ts");
    assert!(
        spec_file.exists(),
        "tests/repro/rf_01.spec.ts must be synthesized"
    );

    let spec_content = fs::read_to_string(&spec_file).unwrap();
    assert!(spec_content.contains("import { test, expect } from '@playwright/test';"));
    assert!(spec_content.contains("getByRole("));
    assert!(!spec_content.contains(".nth("));
    assert!(!spec_content.contains(":nth-child"));

    // 2. Verify .replayfix/inoculations.json registry
    let reg_file = dir.join("inoculations.json");
    assert!(reg_file.exists(), ".replayfix/inoculations.json must exist");

    let reg_val: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&reg_file).unwrap()).unwrap();
    let entries = reg_val["entries"].as_array().expect("entries array");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["bug_id"], "RF-01");
    assert!(entries[0]["spec_sha256"].as_str().is_some());
    assert!(entries[0]["patch_sha256"].as_str().is_some());

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_gate3_stress_quorum_passes_deterministic_and_rejects_flaky() {
    let dir_det = scratch("stress-det");
    // Deterministic pass under Gate 3
    let status_det = Command::new(env!("CARGO_BIN_EXE_replayfix"))
        .args([
            "repro",
            "--bug",
            "RF-01",
            "--mock-heal",
            "--stress-quorum",
            "--output-dir",
            &dir_det.display().to_string(),
        ])
        .status()
        .expect("run repro with stress-quorum");
    assert!(status_det.success());

    let receipt_json: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(dir_det.join("audit_receipt.json")).unwrap())
            .unwrap();
    assert_eq!(receipt_json["status"], "VERIFIED_REPAIRED");
    assert!(receipt_json["stress_quorum"].is_object());
    assert_eq!(receipt_json["stress_quorum"]["quorum_passed"], true);
    assert_eq!(receipt_json["stress_quorum"]["flaky_detected"], false);
    assert_eq!(
        receipt_json["stress_quorum"]["concurrency_passes"]
            .as_array()
            .unwrap()
            .len(),
        3
    );

    // Flaky patch fails closed with Exit Code 21 (FLAKY_NON_DETERMINISTIC)
    let dir_flaky = scratch("stress-flaky");
    let output_flaky = Command::new(env!("CARGO_BIN_EXE_replayfix"))
        .args([
            "repro",
            "--bug",
            "RF-01",
            "--mock-heal",
            "--stress-quorum",
            "--mock-flaky",
            "--output-dir",
            &dir_flaky.display().to_string(),
        ])
        .output()
        .expect("run repro with mock-flaky");
    assert_eq!(output_flaky.status.code(), Some(21));

    let triage_json: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(dir_flaky.join("triage_report.json")).unwrap())
            .unwrap();
    assert_eq!(triage_json["status"], "FLAKY_NON_DETERMINISTIC");

    let _ = fs::remove_dir_all(&dir_det);
    let _ = fs::remove_dir_all(&dir_flaky);
}

#[test]
fn test_autonomous_git_packaging_trailers_and_evidence_matrix() {
    let dir = scratch("git-pkg-test");

    let status = Command::new(env!("CARGO_BIN_EXE_replayfix"))
        .args([
            "repro",
            "--bug",
            "RF-01",
            "--mock-heal",
            "--package-pr",
            "--output-dir",
            &dir.display().to_string(),
        ])
        .status()
        .expect("run repro with package-pr");
    assert!(status.success());

    // 1. Verify pr_description.md exists and contains Evidence Matrix
    let pr_md = dir.join("pr_description.md");
    assert!(pr_md.exists(), "pr_description.md must exist");
    let content = fs::read_to_string(&pr_md).unwrap();
    assert!(content.contains("Evidence Matrix Summary"));
    assert!(content.contains("Gate 1 (Causal Red)"));
    assert!(content.contains("Gate 2 (2-Run Stability Pass)"));
    assert!(content.contains("Cryptographic Commit Trailers"));
    assert!(content.contains("ReplayFix-Signature:"));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_sarif_and_junit_reporting() {
    let dir = scratch("reporters-test");
    let sarif_path = dir.join("results.sarif");
    let junit_path = dir.join("junit.xml");

    let status = Command::new(env!("CARGO_BIN_EXE_replayfix"))
        .args([
            "repro",
            "--bug",
            "RF-01",
            "--mock-heal",
            "--sarif",
            &sarif_path.display().to_string(),
            "--junit",
            &junit_path.display().to_string(),
            "--output-dir",
            &dir.display().to_string(),
        ])
        .status()
        .expect("run repro with sarif and junit");
    assert!(status.success());

    // 1. SARIF v2.1.0 validation
    assert!(sarif_path.exists());
    let sarif_val: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&sarif_path).unwrap()).unwrap();
    assert_eq!(sarif_val["version"], "2.1.0");
    assert_eq!(sarif_val["runs"][0]["tool"]["driver"]["name"], "ReplayFix");

    // 2. JUnit XML validation
    assert!(junit_path.exists());
    let junit_str = fs::read_to_string(&junit_path).unwrap();
    assert!(junit_str.contains("<testsuites"));
    assert!(junit_str.contains("<testcase name=\"RF-01::Gate1_CausalRed\""));
    assert!(junit_str.contains("<testcase name=\"RF-01::Gate2_StabilityGreen\""));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_path_traversal_defense_in_inoculate() {
    let output = Command::new(env!("CARGO_BIN_EXE_replayfix"))
        .args([
            "inoculate",
            "--bug",
            "RF-01",
            "--output",
            "../../../etc/shadow_out",
        ])
        .output()
        .expect("run inoculate with traversal path");
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn test_ambiguity_tie_detector_and_interactive_picker_resolution() {
    use replayfix_core::{BoundingBox, DomCandidate, LocatorResolver};

    let candidates = vec![
        DomCandidate {
            tag: "button".to_string(),
            role: "button".to_string(),
            accessible_name: "Refresh Cart".to_string(),
            test_id: Some("cart-refresh-primary".to_string()),
            bbox: BoundingBox {
                x: 100,
                y: 100,
                width: 120,
                height: 40,
            },
            is_element_from_point: true,
        },
        DomCandidate {
            tag: "button".to_string(),
            role: "button".to_string(),
            accessible_name: "Update Cart".to_string(),
            test_id: Some("cart-refresh-secondary".to_string()),
            bbox: BoundingBox {
                x: 102,
                y: 100,
                width: 120,
                height: 40,
            },
            is_element_from_point: true,
        },
    ];
    let aria_snapshot = r#"- main: - button "Refresh Cart" - button "Update Cart""#;

    let tied = LocatorResolver::detect_ambiguous_tie(160, 120, aria_snapshot, &candidates);
    assert_eq!(tied.len(), 2);
    assert!(tied[0].score_bps.saturating_sub(tied[1].score_bps) < 500);

    // Non-interactive tie-breaker fails closed with AMBIGUOUS_GROUNDING
    assert!(LocatorResolver::resolve_with_tie_breaker(
        160,
        120,
        1280,
        720,
        aria_snapshot,
        &candidates,
        |_| None
    )
    .is_err());

    // Interactive picker selects candidate #2 (index 1)
    let resolved = LocatorResolver::resolve_with_tie_breaker(
        160,
        120,
        1280,
        720,
        aria_snapshot,
        &candidates,
        |_| Some(1),
    )
    .expect("resolve picked candidate");
    assert_eq!(
        resolved.telemetry.matched_selector,
        "page.getByRole('button', { name: 'Update Cart', exact: true })"
    );
}
