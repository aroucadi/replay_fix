//! Release 2.0.0 Sovereign Enterprise Final MMP Integration Test Suite:
//! 1. Pillar 1: Production APM, OpenTelemetry Span Alignment, SourceMap v3 & Secret Scrubbing (`replayfix ingest`)
//! 2. Pillar 2: Distributed Work-Stealing Swarm (`--tasks`, `--notary-log`, live broadcaster hooks, Δrepo = 0)
//! 3. Pillar 3: Real-Time Live Streaming Dashboard (`/api/notary`, `/api/matrix`, DOM diffs, Gate verdicts)
//! 4. Pillar 4: Cross-Browser & Multi-Device Inoculation Matrix (letterbox calibration, matrix parity, `--matrix`)
//! 5. Pillar 5 & Audit Convergence: Enterprise Multi-Party Attestation v2.0 (`FR-066`), 7-Leaf Canonical Merkle Receipts (`FR-026`), `replayfix verify` CLI alias (`FR-028`), `muse-spark-1.3` Web Search & `muse-image-1.0` Badge (`FR-036`), and Sovereign Notary Vault (`proof_viewer.html`).

use replayfix_core::{
    calibrate_viewport_with_letterbox, parse_matrix_arg, sanitize_untrusted_telemetry_text,
    verify_matrix_parity, AttestationRole, AuditReceipt, DashboardServerHandle, Fixture,
    HealOutcome, KeypairManager, MerkleReceipt, MetaGatewayClient, MockHealer,
    MultiPartyAttestation, OtelIngestion, SourceMapResolver, TransparencyLog, TrustPolicy,
};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(1);

fn create_temp_dir(prefix: &str) -> PathBuf {
    let count = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "replayfix_v20_{}_{}_{}",
        prefix,
        std::process::id(),
        count
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn cli_cmd() -> Command {
    Command::new(env!("CARGO_BIN_EXE_replayfix"))
}

// =========================================================================
// PILLAR 1: APM, OPENTELEMETRY SPAN ALIGNMENT, SOURCEMAP & SECRET SCRUBBING
// =========================================================================

#[test]
fn test_v20_pillar1_ingest_otel_sourcemap_and_secret_scrubbing() {
    let temp = create_temp_dir("pillar1_apm");

    // Verify ANSI escape stripping and GitHub/API secret scrubbing
    let dirty = "\x1b[31mLeak: ghp_1234567890abcdefghijklmnopqrstuvwxyzABCD\x1b[0m";
    let cleaned = sanitize_untrusted_telemetry_text(dirty);
    assert!(!cleaned.contains("\x1b["));
    assert!(!cleaned.contains("ghp_1234567890"));
    assert!(cleaned.contains("[REDACTED"));

    let rrweb_file = temp.join("session.rrweb.json");
    let sentry_file = temp.join("crash.sentry.json");
    let otel_file = temp.join("spans.otel.json");
    let sourcemap_file = temp.join("bundle.js.map");
    let out_manifest = temp.join("synthesized.manifest.json");

    fs::write(
        &rrweb_file,
        r#"[
            {"type": 4, "timestamp": 1000, "data": {"href": "http://localhost:3000/checkout", "width": 1440, "height": 900}},
            {"type": 3, "timestamp": 1500, "data": {"source": 2, "type": 2, "id": 10, "x": 420.0, "y": 310.0}}
        ]"#,
    )
    .unwrap();

    fs::write(
        &sentry_file,
        r#"{
            "event_id": "2000beefcafe1234",
            "exception": {
                "values": [{
                    "type": "TypeError",
                    "value": "Cannot read properties of undefined (reading 'total')",
                    "stacktrace": {
                        "frames": [
                            {"filename": "webpack:///src/components/CartSummary.tsx", "in_app": true, "lineno": 42}
                        ]
                    }
                }]
            }
        }"#,
    )
    .unwrap();

    fs::write(
        &otel_file,
        r#"{
            "spans": [
                {
                    "trace_id": "4bf92f3577b34da6a3ce929d0e0e4736",
                    "span_id": "00f067aa0ba902b7",
                    "name": "POST /api/checkout",
                    "start_time_ms": 1525,
                    "end_time_ms": 1610,
                    "status_code": "ERROR"
                }
            ]
        }"#,
    )
    .unwrap();

    fs::write(
        &sourcemap_file,
        r#"{
            "version": 3,
            "file": "bundle.min.js",
            "sources": ["webpack:///src/state/checkoutSlice.ts", "node_modules/react/index.js"],
            "names": ["calculateTotal"]
        }"#,
    )
    .unwrap();

    let spans = OtelIngestion::parse_json(&fs::read_to_string(&otel_file).unwrap()).unwrap();
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].name, "POST /api/checkout");

    let sm = SourceMapResolver::parse_json(&fs::read_to_string(&sourcemap_file).unwrap()).unwrap();
    let sources = sm.resolve_candidate_sources().unwrap();
    assert_eq!(sources, vec!["src/state/checkoutSlice.ts"]);

    let output = cli_cmd()
        .args([
            "ingest",
            "--rrweb",
            rrweb_file.to_str().unwrap(),
            "--sentry",
            sentry_file.to_str().unwrap(),
            "--otel",
            otel_file.to_str().unwrap(),
            "--sourcemap",
            sourcemap_file.to_str().unwrap(),
            "--output",
            out_manifest.to_str().unwrap(),
            "--json",
        ])
        .output()
        .expect("Failed to run replayfix ingest");

    assert!(output.status.success());
    let stdout_json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(stdout_json["status"], "INGESTED");
    assert_eq!(stdout_json["otel_spans_count"], 1);
    assert_eq!(stdout_json["aligned_spans_count"], 1);
    assert_eq!(stdout_json["has_sourcemap"], true);

    let manifest_raw = fs::read_to_string(&out_manifest).unwrap();
    assert!(manifest_raw.contains("src/state/checkoutSlice.ts"));
    assert!(manifest_raw.contains("POST /api/checkout[ERROR]"));

    let _ = fs::remove_dir_all(&temp);
}

// =========================================================================
// PILLAR 2: DISTRIBUTED SWARM WITH --tasks AND AUTOMATIC NOTARY LEDGER
// =========================================================================

#[test]
fn test_v20_pillar2_swarm_tasks_and_notary_ledger() {
    let temp = create_temp_dir("pillar2_swarm");
    let repo_dir = temp.join("repo");
    let out_dir = temp.join("out");
    let notary_log = temp.join("transparency_log.json");
    fs::create_dir_all(&repo_dir).unwrap();
    fs::write(repo_dir.join("README.md"), "# Base Repo Immutable").unwrap();

    let task1_path = temp.join("RF-01.json");
    let task2_path = temp.join("RF-02.json");
    fs::write(
        &task1_path,
        serde_json::json!({
            "task_id": "RF-01",
            "title": "Cart Quantity NaN",
            "repo_path": repo_dir.to_string_lossy(),
            "bug_ref": "bug/RF-01",
            "video_path": "recordings/RF-01.mp4"
        })
        .to_string(),
    )
    .unwrap();
    fs::write(
        &task2_path,
        serde_json::json!({
            "task_id": "RF-02",
            "title": "Coupon Stale Total",
            "repo_path": repo_dir.to_string_lossy(),
            "bug_ref": "bug/RF-02",
            "video_path": "recordings/RF-02.mp4"
        })
        .to_string(),
    )
    .unwrap();

    let output = cli_cmd()
        .args([
            "swarm",
            "--workers",
            "2",
            "--tasks",
            task1_path.to_str().unwrap(),
            task2_path.to_str().unwrap(),
            "--repo",
            repo_dir.to_str().unwrap(),
            "--output",
            out_dir.to_str().unwrap(),
            "--notary-log",
            notary_log.to_str().unwrap(),
            "--mock-heal",
            "--json",
        ])
        .output()
        .expect("Failed to run replayfix swarm --tasks");

    assert!(output.status.success());
    let summary: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(summary["total_tasks"], 2);
    assert_eq!(summary["completed_tasks"], 2);
    assert_eq!(summary["failed_tasks"], 0);

    // Verify TransparencyLog automatically notarized both verified swarm receipts
    assert!(notary_log.exists());
    let kp = KeypairManager::load_or_generate(Some(&out_dir)).unwrap();
    let log = TransparencyLog::load_or_new(&notary_log, &kp).unwrap();
    assert_eq!(log.tree_size, 2);
    assert!(log.verify_ledger_integrity(None).unwrap());

    let _ = fs::remove_dir_all(&temp);
}

fn http_get_body(port: u16, path: &str) -> String {
    use std::io::{Read, Write};
    use std::net::TcpStream;
    let mut stream = TcpStream::connect(format!("127.0.0.1:{port}")).unwrap();
    let req = format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n");
    stream.write_all(req.as_bytes()).unwrap();
    let mut buf = String::new();
    stream.read_to_string(&mut buf).unwrap();
    buf.split_once("\r\n\r\n")
        .map(|(_, body)| body.to_string())
        .unwrap_or(buf)
}

// =========================================================================
// PILLAR 3: LIVE STREAMING DASHBOARD (/api/notary, /api/matrix, DOM DIFFS)
// =========================================================================

#[test]
fn test_v20_pillar3_dashboard_notary_and_matrix_endpoints() {
    let temp = create_temp_dir("pillar3_dashboard");
    let mut handle = DashboardServerHandle::start(0, temp.clone()).unwrap();
    let port = handle.port();

    let bcast = &handle.broadcaster;
    bcast.broadcast_dom_diff(
        "RF-01",
        r#"- button "Refresh Cart" - text "Total: $0.00""#,
        r#"- button "Refresh Cart" - text "Total: $25.00""#,
    );
    bcast.broadcast_gate_verdict("RF-01", "Gate2", "GREEN");

    let status_resp: Value = serde_json::from_str(&http_get_body(port, "/api/status")).unwrap();
    assert_eq!(status_resp["status"], "online");
    assert_eq!(status_resp["release_version"], "2.0.0");

    let notary_resp: Value = serde_json::from_str(&http_get_body(port, "/api/notary")).unwrap();
    assert_eq!(notary_resp["tree_size"], 0);

    let matrix_resp: Value = serde_json::from_str(&http_get_body(port, "/api/matrix")).unwrap();
    assert_eq!(
        matrix_resp["engines"],
        serde_json::json!(["chromium", "firefox", "webkit"])
    );

    let sse_text = http_get_body(port, "/events/reasoning/RF-01");
    assert!(sse_text.contains("dom_diff"));
    assert!(sse_text.contains("gate_verdict"));

    handle.shutdown();
    let _ = fs::remove_dir_all(&temp);
}

// =========================================================================
// PILLAR 4: CROSS-BROWSER & MULTI-DEVICE INOCULATION MATRIX
// =========================================================================

#[test]
fn test_v20_pillar4_inoculation_matrix_and_letterbox_calibration() {
    let temp = create_temp_dir("pillar4_matrix");
    let repo_dir = temp.join("repo");
    let out_dir = temp.join("out");
    fs::create_dir_all(&repo_dir).unwrap();

    // Verify letterbox coordinate calibration formula
    let (x_css, y_css) = calibrate_viewport_with_letterbox(
        (680.0, 400.0),
        (40.0, 40.0),
        (1280.0, 720.0),
        (1280.0, 720.0),
        2.0,
    );
    assert_eq!(x_css, 320.0);
    assert_eq!(y_css, 180.0);

    let fixture = Fixture::for_task("RF-01").unwrap();
    let (engines, presets) =
        parse_matrix_arg("chromium,firefox,webkit,mobile-portrait,tablet,desktop");
    let report = verify_matrix_parity(&fixture, &engines, &presets);
    assert_eq!(report.total_cells, 9); // 3 engines × 3 viewports
    assert!(report.all_green);

    // Run `replayfix inoculate --repo <path> --output <out> --matrix --json` (defaulting --bug RF-01 and standalone --matrix)
    let output = cli_cmd()
        .args([
            "inoculate",
            "--repo",
            repo_dir.to_str().unwrap(),
            "--output",
            out_dir.to_str().unwrap(),
            "--matrix",
            "--json",
        ])
        .output()
        .expect("Failed to run replayfix inoculate --matrix");

    assert!(output.status.success());
    let json_out: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json_out["bug_id"], "RF-01");
    assert_eq!(json_out["matrix_enabled"], true);

    let _ = fs::remove_dir_all(&temp);
}

// =========================================================================
// PILLAR 5 & AUDIT CONVERGENCE: MULTI-PARTY ATTESTATION v2.0, 7-LEAF MERKLE
// RECEIPTS, `replayfix verify` ALIAS, GATEWAY GROUNDING & PROOF PORTAL
// =========================================================================

#[test]
fn test_v20_pillar5_multiparty_attestation_7leaf_receipt_and_verify_alias() {
    let temp = create_temp_dir("pillar5_notary");
    let repo_dir = temp.join("repo");
    let out_dir = temp.join("out");
    fs::create_dir_all(&repo_dir).unwrap();

    let fixture = Fixture::for_task("RF-01").unwrap();
    let outcome = MockHealer::run_in_repo(
        &fixture,
        fixture.builtin_diff,
        &repo_dir,
        std::slice::from_ref(&out_dir),
    )
    .unwrap();

    let mut receipt = match outcome {
        HealOutcome::Verified(r) => r,
        HealOutcome::Rejected(rep) => panic!("Expected verified receipt, got {:?}", rep),
    };

    // Write canonical 7-leaf disaggregated artifacts (FR-026 / NFR-009)
    receipt
        .write_canonical_7leaf_artifacts(&out_dir, Some(fixture.builtin_diff), None)
        .unwrap();

    let merkle_raw = fs::read_to_string(out_dir.join("merkle_receipt.json")).unwrap();
    let merkle: MerkleReceipt = serde_json::from_str(&merkle_raw).unwrap();
    assert!(
        merkle.tree_size >= 7,
        "Expected >= 7 canonical RFC 6962 leaves, got {}",
        merkle.tree_size
    );
    merkle.verify_all().unwrap();

    // Test Enterprise Multi-Party Attestation Chain v2.0 (Developer + Harness + CiJudge) (FR-066)
    let key_dir = out_dir.join("keys");
    let dev_kp = KeypairManager::generate();
    let harness_kp = KeypairManager::load_or_generate(Some(&key_dir)).unwrap();
    let judge_kp = KeypairManager::generate();

    let mp_chain = MultiPartyAttestation::sign_chain(
        &merkle.merkle_root,
        merkle.tree_size,
        &receipt.created_at,
        &[
            (AttestationRole::Developer, &dev_kp),
            (AttestationRole::Harness, &harness_kp),
            (AttestationRole::CiJudge, &judge_kp),
        ],
    )
    .unwrap();
    assert_eq!(mp_chain.signatures.len(), 3);
    assert_eq!(
        mp_chain
            .verify_chain(&merkle.merkle_root, merkle.tree_size)
            .unwrap(),
        3
    );

    // Tampered root must fail multi-party verification
    assert!(mp_chain
        .verify_chain(
            "0000000000000000000000000000000000000000000000000000000000000000",
            merkle.tree_size
        )
        .is_err());

    // Test `replayfix verify --receipt <dir> --public-key <pub_file> --json` (FR-028)
    let pub_key_file = key_dir.join("ed25519_signing.pub");
    let verify_out = cli_cmd()
        .args([
            "verify",
            "--receipt",
            out_dir.to_str().unwrap(),
            "--public-key",
            pub_key_file.to_str().unwrap(),
            "--json",
        ])
        .output()
        .expect("Failed to run replayfix verify");

    assert!(verify_out.status.success());
    let verify_json: Value = serde_json::from_slice(&verify_out.stdout).unwrap();
    assert_eq!(verify_json["status"], "SUCCESS");
    assert_eq!(verify_json["verification_outcome"], "VERIFIED_TRUSTED");
    assert!(verify_json["tree_size"].as_u64().unwrap() >= 7);

    // Verify AuditReceipt API round-trip
    let (_, _, _, verified_size) =
        AuditReceipt::verify_receipt_and_attestation(&out_dir, &TrustPolicy::AllowAny).unwrap();
    assert!(verified_size >= 7);

    // Test Gateway web search grounding (`muse-spark-1.3`) and badge synthesis (`muse-image-1.0`) (FR-036)
    let gw = MetaGatewayClient::default();
    let search_res = gw
        .ground_with_web_search("Array.prototype.reduce TypeError empty array")
        .unwrap();
    assert_eq!(search_res.model, "muse-spark-1.3");
    assert!(!search_res.citations.is_empty());

    let badge_res = gw
        .synthesize_verification_badge("RF-01", &merkle.merkle_root)
        .unwrap();
    assert_eq!(badge_res.model, "muse-image-1.0");
    assert!(badge_res.svg_markup.contains("<svg"));

    // Verify proof_viewer.html contains the Multi-Party Attestation Chain v2.0 visual panel
    let portal_html = include_str!("../../../proof_viewer.html");
    assert!(portal_html.contains(r#"id="multiparty-chain-list""#));
    assert!(portal_html.contains("Enterprise Multi-Party Attestation Chain (v2.0)"));
    assert!(portal_html.contains("default-src 'none'"));

    let _ = fs::remove_dir_all(&temp);
}
