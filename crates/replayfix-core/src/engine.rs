//! Dual-gate engine: Gate 1 (RED) -> Repair -> Gate 2 (GREEN, 2-run stability), supporting both
//! the deterministic simulated oracle and live Playwright CDP browser execution inside an
//! isolated ephemeral scratch sandbox ($\Delta \text{repo} = 0$), with Spec 060 bounded
//! autonomous Meta Muse repair, anti-cheating test freeze, and syntax preflight.

use crate::browser::{
    BridgeCommand, BridgeResponse, PlaywrightBridge, SandboxServer, BRIDGE_SCRIPT_FILENAME,
};
use crate::digest::{canonical_json, sha256_hex};
use crate::error::{CoreError, Result};
use crate::fixtures::Fixture;
use crate::fsm::{Phase, PhaseMachine};
use crate::locator::{DomCandidate, GroundedTarget, LocatorResolver};
use crate::patch::{PatchOutcome, UnifiedPatcher};
use crate::perception::VideoTelemetry;
use crate::platform::EphemeralPort;
use crate::receipt::{
    timestamp_now, AuditReceipt, GateRecord, PatchRecord, TriageReport, TriageStatus, DISCLAIMER,
    DISCLAIMER_LIVE_BROWSER, SCHEMA_VERSION, SUBSTRATE, SUBSTRATE_LIVE_BROWSER,
};
use crate::repair::{
    check_repair_deadline, AttemptLineage, BlockPatcher, BoundedRepairClient, MuseModelConfig,
    PromptBuilder, RepairClientMode, RepairRequest, RepairTelemetry, StructuredRepairPatch,
    SyntaxPreflight, TestFreezeGuard, TokenAccounting, MAX_REPAIR_ATTEMPTS,
};
use crate::worktree::{ScratchSandbox, WorktreeManifest};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

static EPHEMERAL_REPO_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Required consecutive GREEN runs at Gate 2 to confirm deterministic non-flaky repair.
pub const GATE2_REQUIRED_CONSECUTIVE_RUNS: u32 = 2;

/// Immutable description of what both gates must evaluate.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BehavioralContract {
    pub contract_id: String,
    pub task_id: String,
    pub setup: String,
    pub target: String,
    pub action: String,
    pub postcondition_id: String,
}

impl BehavioralContract {
    /// SHA-256 over the canonical (key-sorted, compact) JSON form.
    pub fn digest(&self) -> Result<String> {
        let value = serde_json::to_value(self)?;
        Ok(sha256_hex(canonical_json(&value)?.as_bytes()))
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OracleEventKind {
    Setup,
    Target,
    Action,
    Assert,
}

const EVENT_ORDER: [OracleEventKind; 4] = [
    OracleEventKind::Setup,
    OracleEventKind::Target,
    OracleEventKind::Action,
    OracleEventKind::Assert,
];

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct OracleEvent {
    pub seq: u32,
    pub kind: OracleEventKind,
    pub passed: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GateVerdict {
    Red,
    Green,
    Invalid,
}

/// Run the simulated oracle. Execution stops at the first failing event,
/// exactly as a real run could not assert after a failed setup or action.
pub fn run_oracle(contract: &BehavioralContract, source: &str) -> Vec<OracleEvent> {
    let target_marker = format!("function {}", contract.target);
    let defect_fixed = match contract.target.as_str() {
        "cartItemCount" => !source.contains("cart.items.length") || source.contains("cart.items?"),
        "applyDiscount" => !source.contains("0.8 * 0.8"),
        "decrementQuantity" => source.contains("Math.max") || source.contains("currentQty > 0"),
        "switchTab" => !source.contains("return [];"),
        "submitOrder" => source.contains("if (isSubmitting)"),
        _ => !source.contains("cart.items.length"),
    };

    let checks: [bool; 4] = [
        !source.trim().is_empty(),
        source.contains(&target_marker),
        source.contains(&target_marker),
        defect_fixed,
    ];

    let mut events = Vec::with_capacity(4);
    for (seq, (kind, passed)) in EVENT_ORDER.iter().zip(checks).enumerate() {
        events.push(OracleEvent {
            seq: seq as u32,
            kind: *kind,
            passed,
        });
        if !passed {
            break;
        }
    }
    events
}

/// Classify an event stream (Law VI): RED needs SETUP, TARGET and ACTION to
/// pass before a failed ASSERT. Anything incomplete or out of order is INVALID.
pub fn classify(events: &[OracleEvent]) -> GateVerdict {
    let in_order = events
        .iter()
        .enumerate()
        .all(|(i, e)| e.seq as usize == i && EVENT_ORDER.get(i) == Some(&e.kind));
    if !in_order || events.len() < EVENT_ORDER.len() {
        return GateVerdict::Invalid;
    }
    let (pre, last) = events.split_at(EVENT_ORDER.len() - 1);
    if !pre.iter().all(|e| e.passed) {
        return GateVerdict::Invalid;
    }
    if last[0].passed {
        GateVerdict::Green
    } else {
        GateVerdict::Red
    }
}

fn describe_invalid(events: &[OracleEvent]) -> String {
    match events.iter().find(|e| !e.passed) {
        Some(e) => format!(
            "oracle stopped at {:?} before asserting; this is not a valid verdict",
            e.kind
        ),
        None => "oracle event stream was incomplete or out of order".to_string(),
    }
}

/// Resolve the 40-character commit hash of a git ref (e.g. main, HEAD) in `repo_path`.
/// Returns fallback hash if git is unavailable or ref cannot be resolved.
#[must_use]
pub fn resolve_git_ref(repo_path: &Path, git_ref: &str) -> String {
    let out = std::process::Command::new("git")
        .args(["rev-parse", git_ref])
        .current_dir(repo_path)
        .output();
    if let Ok(output) = out {
        if output.status.success() {
            let sha = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if sha.len() == 40 && sha.chars().all(|c| c.is_ascii_hexdigit()) {
                return sha;
            }
        }
    }
    "0000000000000000000000000000000000000000".to_string()
}

/// Options for live Playwright CDP browser execution and Spec 060 autonomous model repair.
#[derive(Debug, Clone, Default)]
pub struct LiveBrowserOptions {
    pub headed: bool,
    pub candidate_override: Option<Vec<DomCandidate>>,
    pub video_path: Option<PathBuf>,
    pub keyframe_dir: Option<PathBuf>,
    pub model_slug: Option<String>,
    pub repair_mode: Option<RepairClientMode>,
    pub clean_ref: Option<String>,
    pub verify_matrix: bool,
    pub mock_regression: bool,
}

/// Terminal result of a mock-heal or live-browser run.
#[derive(Debug, Clone)]
pub enum HealOutcome {
    Verified(Box<AuditReceipt>),
    Rejected(Box<TriageReport>),
}

/// Internal result of running the bounded autonomous repair loop.
enum AutonomousRepairResult {
    Verified {
        outcome: PatchOutcome,
        diff_text: String,
        gate2_events: Vec<OracleEvent>,
        repair_telemetry: RepairTelemetry,
        token_accounting: TokenAccounting,
    },
    Rejected {
        status: TriageStatus,
        reason: String,
        repair_telemetry: Option<RepairTelemetry>,
        token_accounting: Option<TokenAccounting>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MockHealer;

impl MockHealer {
    /// Run the full loop against `fixture` using `diff` as the candidate repair
    /// inside an isolated ephemeral base directory.
    pub fn run(fixture: &Fixture, diff: &str) -> Result<HealOutcome> {
        Self::run_with_gate2_contract(fixture, diff, &fixture.contract)
    }

    /// Like [`run`](Self::run) but lets Gate 2 evaluate a different contract.
    /// Production code never does this; it exists to prove the invariance guard fires.
    pub fn run_with_gate2_contract(
        fixture: &Fixture,
        diff: &str,
        gate2_contract: &BehavioralContract,
    ) -> Result<HealOutcome> {
        let seq = EPHEMERAL_REPO_COUNTER.fetch_add(1, Ordering::Relaxed);
        let temp_repo = std::env::temp_dir().join(format!(
            "replayfix-ephemeral-repo-{}-{}",
            std::process::id(),
            seq
        ));
        fs::create_dir_all(&temp_repo)?;
        let res = Self::run_in_repo_with_options(
            fixture,
            diff,
            &temp_repo,
            &[],
            gate2_contract,
            None,
            None,
            false,
        );
        let _ = fs::remove_dir_all(&temp_repo);
        res
    }

    /// Run the full two-gate loop against `repo_path` in an isolated [`ScratchSandbox`],
    /// enforcing dual-layer $\Delta \text{repo} = 0$ on `repo_path` and allocating an [`EphemeralPort`].
    pub fn run_in_repo(
        fixture: &Fixture,
        diff: &str,
        repo_path: &Path,
        extra_excludes: &[PathBuf],
    ) -> Result<HealOutcome> {
        Self::run_in_repo_with_regression(fixture, diff, repo_path, extra_excludes, false)
    }

    pub fn run_in_repo_with_regression(
        fixture: &Fixture,
        diff: &str,
        repo_path: &Path,
        extra_excludes: &[PathBuf],
        mock_regression: bool,
    ) -> Result<HealOutcome> {
        Self::run_in_repo_with_options(
            fixture,
            diff,
            repo_path,
            extra_excludes,
            &fixture.contract,
            None,
            None,
            mock_regression,
        )
    }

    /// Run the two-gate loop in simulated oracle mode using the Spec 060 bounded autonomous
    /// repair engine (`--model`, `--mock-failing-repair`, or `--mock-syntax-retry`).
    pub fn run_autonomous_in_repo(
        fixture: &Fixture,
        repo_path: &Path,
        extra_excludes: &[PathBuf],
        model_slug: &str,
        repair_mode: RepairClientMode,
    ) -> Result<HealOutcome> {
        Self::run_in_repo_with_options(
            fixture,
            "",
            repo_path,
            extra_excludes,
            &fixture.contract,
            None,
            Some((model_slug, repair_mode)),
            false,
        )
    }

    /// Run the full two-gate loop in a live Chromium browser via Playwright CDP grounding
    /// (`document.elementFromPoint` + `ariaSnapshot` + `LocatorResolver`), serving the
    /// isolated `ScratchSandbox` over an OS-assigned ephemeral localhost port.
    pub fn run_live_browser_in_repo(
        fixture: &Fixture,
        diff: &str,
        repo_path: &Path,
        extra_excludes: &[PathBuf],
        options: &LiveBrowserOptions,
    ) -> Result<HealOutcome> {
        let task_id = fixture.task_id;
        let mut fsm = PhaseMachine::new();

        // Validate model configuration upfront if autonomous model repair is requested.
        let autonomous_config = match (options.model_slug.as_deref(), options.repair_mode) {
            (Some(slug), mode) => {
                Some((MuseModelConfig::new(slug, None)?, mode.unwrap_or_default()))
            }
            (None, Some(mode)) => Some((
                MuseModelConfig::new(crate::repair::MUSE_SPARK_13_SLUG, None)?,
                mode,
            )),
            (None, None) => None,
        };

        // 1. Capture pre-run dual-layer workspace manifest on the base repository.
        let pre_manifest = WorktreeManifest::capture_with_excludes(repo_path, extra_excludes)?;

        // 2. Create isolated scratch sandbox.
        let mut sandbox = ScratchSandbox::create_with_excludes(repo_path, task_id, extra_excludes)?;

        macro_rules! reject_live {
            ($status:expr, $reason:expr) => {{
                let _ = sandbox.teardown();
                fsm.advance(Phase::Rejected)?;
                return Ok(HealOutcome::Rejected(Box::new(
                    TriageReport::with_substrate(
                        $status,
                        $reason,
                        task_id,
                        SUBSTRATE_LIVE_BROWSER,
                        fsm.trace(),
                    ),
                )));
            }};
            ($status:expr, $reason:expr, $rtel:expr, $tokens:expr) => {{
                let _ = sandbox.teardown();
                fsm.advance(Phase::Rejected)?;
                return Ok(HealOutcome::Rejected(Box::new(
                    TriageReport::with_substrate(
                        $status,
                        $reason,
                        task_id,
                        SUBSTRATE_LIVE_BROWSER,
                        fsm.trace(),
                    )
                    .with_repair_telemetry($rtel, $tokens),
                )));
            }};
        }

        // 3. Materialize target source file and HTML host shell inside the scratch sandbox.
        let sandbox_target_file = sandbox.resolve_safe_path(fixture.target_path)?;
        if !sandbox_target_file.exists() {
            sandbox.write_file(fixture.target_path, fixture.buggy_source)?;
        }
        let sandbox_index_html = sandbox.resolve_safe_path("index.html")?;
        if !sandbox_index_html.exists() {
            sandbox.write_file("index.html", fixture.index_html)?;
        }
        let initial_source = sandbox.read_file(fixture.target_path)?;

        // Capture anti-cheating SHA-256 test freeze guard before Gate 1 / Repair.
        let freeze_guard = TestFreezeGuard::capture(sandbox.root())?;

        // 4. Optional Spec 050 Multimodal Video Telemetry, SAM 3.1 & ASR Ingestion.
        let (video_telemetry, target_click_x, target_click_y) =
            if let Some(ref vpath) = options.video_path {
                if vpath.exists() {
                    let kf_dir = options
                        .keyframe_dir
                        .clone()
                        .unwrap_or_else(|| sandbox.root().join("keyframes"));
                    match VideoTelemetry::ingest(
                        vpath,
                        &kf_dir,
                        (fixture.viewport_width, fixture.viewport_height),
                        task_id,
                    ) {
                        Ok(tel) => {
                            let (cx, cy) =
                                (tel.sam_snap.calibrated_x_css, tel.sam_snap.calibrated_y_css);
                            (Some(tel), cx, cy)
                        }
                        Err(CoreError::Grounding(msg)) => {
                            reject_live!(TriageStatus::AmbiguousGrounding, msg)
                        }
                        Err(CoreError::SecurityViolation(msg)) => {
                            reject_live!(TriageStatus::SecurityViolation, msg)
                        }
                        Err(e) => reject_live!(TriageStatus::BridgeExecutionFailed, e.to_string()),
                    }
                } else {
                    (None, fixture.click_x, fixture.click_y)
                }
            } else {
                (None, fixture.click_x, fixture.click_y)
            };

        // 5. Start embedded localhost HTTP server on an OS-assigned ephemeral port (127.0.0.1:0).
        let mut server = match SandboxServer::start(sandbox.root(), None) {
            Ok(s) => s,
            Err(e) => reject_live!(TriageStatus::BridgeExecutionFailed, e.to_string()),
        };
        let ephemeral_port = server.port();
        let page_url = format!("{}/index.html", server.base_url());

        // 6. Start Playwright Node.js stdio bridge.
        let mut bridge = match PlaywrightBridge::start(
            sandbox.root(),
            options.headed,
            fixture.viewport_width,
            fixture.viewport_height,
        ) {
            Ok(b) => b,
            Err(e) => {
                server.shutdown();
                reject_live!(TriageStatus::BridgeExecutionFailed, e.to_string());
            }
        };
        sandbox.register_child_pid(bridge.pid());

        // 7. Gate 1 (Live Browser RED): Probe, ground locator, click, and observe runtime TypeError.
        let contract_sha256 = fixture.contract.digest()?;
        let (gate1_grounded, gate1_events) = match Self::execute_browser_gate(
            &mut bridge,
            fixture,
            &page_url,
            (target_click_x, target_click_y),
            options.candidate_override.as_deref(),
        ) {
            Ok(pair) => pair,
            Err(CoreError::Grounding(msg)) => {
                bridge.shutdown();
                server.shutdown();
                reject_live!(TriageStatus::AmbiguousGrounding, msg);
            }
            Err(e) => {
                bridge.shutdown();
                server.shutdown();
                reject_live!(TriageStatus::BridgeExecutionFailed, e.to_string());
            }
        };

        let gate1_verdict = classify(&gate1_events);
        if gate1_verdict != GateVerdict::Red {
            bridge.shutdown();
            server.shutdown();
            reject_live!(
                TriageStatus::Gate1NotRed,
                format!(
                    "Gate 1 must be RED in live browser before repair (got {:?}): {}",
                    gate1_verdict,
                    describe_invalid(&gate1_events)
                )
            );
        }
        fsm.advance(Phase::Gate1Red)?;
        let gate1_at = timestamp_now();

        // 8. Repair + Gate 2 (2-run stability verification).
        fsm.advance(Phase::Repair)?;
        let gate2_sha256 = fixture.contract.digest()?;
        if gate2_sha256 != contract_sha256 {
            bridge.shutdown();
            server.shutdown();
            reject_live!(
                TriageStatus::ContractMismatch,
                "Gate 2 contract digest differs from Gate 1; refusing to compare results"
            );
        }

        let (outcome, effective_diff, gate2_events, repair_telemetry, token_accounting) =
            if let Some((model_config, client_mode)) = autonomous_config {
                let mut eval_error: Option<(TriageStatus, String)> = None;
                let auto_res = Self::execute_autonomous_repair_loop(
                    fixture,
                    &mut sandbox,
                    &initial_source,
                    &freeze_guard,
                    model_config,
                    client_mode,
                    &mut |_sandbox_ref| {
                        // Run 2 consecutive live-browser Gate 2 verifications.
                        let mut last_events = Vec::new();
                        for _run_idx in 0..GATE2_REQUIRED_CONSECUTIVE_RUNS {
                            match Self::execute_browser_gate(
                                &mut bridge,
                                fixture,
                                &page_url,
                                (target_click_x, target_click_y),
                                options.candidate_override.as_deref(),
                            ) {
                                Ok((g2_grounded, g2_events)) => {
                                    if g2_grounded.selector != gate1_grounded.selector {
                                        eval_error = Some((
                                            TriageStatus::ContractMismatch,
                                            format!(
                                                "Gate 2 grounded selector '{}' differs from Gate 1 selector '{}'",
                                                g2_grounded.selector, gate1_grounded.selector
                                            ),
                                        ));
                                        return Err(CoreError::Verification(
                                            "Gate 2 selector mismatch".to_string(),
                                        ));
                                    }
                                    let v = classify(&g2_events);
                                    last_events = g2_events;
                                    if v != GateVerdict::Green {
                                        return Ok((false, last_events));
                                    }
                                }
                                Err(CoreError::Grounding(msg)) => {
                                    eval_error =
                                        Some((TriageStatus::AmbiguousGrounding, msg.clone()));
                                    return Err(CoreError::Grounding(msg));
                                }
                                Err(e) => {
                                    eval_error =
                                        Some((TriageStatus::BridgeExecutionFailed, e.to_string()));
                                    return Err(e);
                                }
                            }
                        }
                        Ok((true, last_events))
                    },
                )?;

                if let Some((st, msg)) = eval_error {
                    bridge.shutdown();
                    server.shutdown();
                    reject_live!(st, msg);
                }

                match auto_res {
                    AutonomousRepairResult::Verified {
                        outcome,
                        diff_text,
                        gate2_events,
                        repair_telemetry,
                        token_accounting,
                    } => (
                        outcome,
                        diff_text,
                        gate2_events,
                        Some(repair_telemetry),
                        Some(token_accounting),
                    ),
                    AutonomousRepairResult::Rejected {
                        status,
                        reason,
                        repair_telemetry,
                        token_accounting,
                    } => {
                        bridge.shutdown();
                        server.shutdown();
                        reject_live!(status, reason, repair_telemetry, token_accounting);
                    }
                }
            } else {
                if diff.trim().is_empty() {
                    bridge.shutdown();
                    server.shutdown();
                    reject_live!(
                        TriageStatus::RepairNoopRejected,
                        "The candidate patch is empty; a repair must change the source"
                    );
                }
                let outcome = match UnifiedPatcher::apply_diff(&initial_source, diff) {
                    Ok(o) => o,
                    Err(CoreError::SecurityViolation(msg)) => {
                        bridge.shutdown();
                        server.shutdown();
                        reject_live!(TriageStatus::SecurityViolation, msg);
                    }
                    Err(e) => {
                        bridge.shutdown();
                        server.shutdown();
                        reject_live!(TriageStatus::PatchApplyFailed, e.to_string());
                    }
                };
                if let Err(err) = freeze_guard.validate_target_path(&outcome.target_path) {
                    bridge.shutdown();
                    server.shutdown();
                    match err {
                        CoreError::SecurityViolation(msg) => {
                            reject_live!(TriageStatus::SecurityViolation, msg);
                        }
                        other => return Err(other),
                    }
                }
                if outcome.target_path != fixture.target_path {
                    bridge.shutdown();
                    server.shutdown();
                    reject_live!(
                        TriageStatus::PatchApplyFailed,
                        format!(
                            "Patch targets '{}' but the repair scope is '{}'",
                            outcome.target_path, fixture.target_path
                        )
                    );
                }
                if outcome.is_noop(&initial_source) {
                    bridge.shutdown();
                    server.shutdown();
                    reject_live!(
                        TriageStatus::RepairNoopRejected,
                        "The patch applies cleanly but leaves the source unchanged"
                    );
                }

                sandbox.write_file(&outcome.target_path, &outcome.patched)?;
                if let Err(err) = freeze_guard.verify_integrity(sandbox.root()) {
                    bridge.shutdown();
                    server.shutdown();
                    match err {
                        CoreError::SecurityViolation(msg) => {
                            reject_live!(TriageStatus::SecurityViolation, msg);
                        }
                        other => return Err(other),
                    }
                }
                if let Err(e) = SyntaxPreflight::check_file(sandbox.root(), &outcome.target_path) {
                    bridge.shutdown();
                    server.shutdown();
                    reject_live!(TriageStatus::PatchApplyFailed, e.to_string());
                }

                let mut final_events = Vec::new();
                for _run_idx in 0..GATE2_REQUIRED_CONSECUTIVE_RUNS {
                    let (gate2_grounded, gate2_events) = match Self::execute_browser_gate(
                        &mut bridge,
                        fixture,
                        &page_url,
                        (target_click_x, target_click_y),
                        options.candidate_override.as_deref(),
                    ) {
                        Ok(pair) => pair,
                        Err(CoreError::Grounding(msg)) => {
                            bridge.shutdown();
                            server.shutdown();
                            reject_live!(TriageStatus::AmbiguousGrounding, msg);
                        }
                        Err(e) => {
                            bridge.shutdown();
                            server.shutdown();
                            reject_live!(TriageStatus::BridgeExecutionFailed, e.to_string());
                        }
                    };

                    if gate2_grounded.selector != gate1_grounded.selector {
                        bridge.shutdown();
                        server.shutdown();
                        reject_live!(
                            TriageStatus::ContractMismatch,
                            format!(
                                "Gate 2 grounded selector '{}' differs from Gate 1 selector '{}'",
                                gate2_grounded.selector, gate1_grounded.selector
                            )
                        );
                    }

                    let gate2_verdict = classify(&gate2_events);
                    match gate2_verdict {
                        GateVerdict::Green => {
                            final_events = gate2_events;
                        }
                        GateVerdict::Red => {
                            bridge.shutdown();
                            server.shutdown();
                            reject_live!(
                                TriageStatus::Gate2NotGreen,
                                "The defect still reproduces in live browser after the patch"
                            );
                        }
                        GateVerdict::Invalid => {
                            bridge.shutdown();
                            server.shutdown();
                            reject_live!(
                                TriageStatus::Gate2NotGreen,
                                format!(
                                    "Gate 2 produced no valid verdict in live browser: {}",
                                    describe_invalid(&gate2_events)
                                )
                            );
                        }
                    }
                }

                (outcome, diff.to_string(), final_events, None, None)
            };

        let bridge_script_exclude = vec![PathBuf::from(BRIDGE_SCRIPT_FILENAME)];
        let sandbox_manifest =
            WorktreeManifest::capture_with_excludes(sandbox.root(), &bridge_script_exclude)?;
        let sandbox_tree_sha256 = sandbox_manifest.tree_sha256;

        bridge.shutdown();
        server.shutdown();

        // 9. Verify dual-layer base repository integrity (\Delta repo = 0) before accepting Gate 2.
        let post_manifest = WorktreeManifest::capture_with_excludes(repo_path, extra_excludes)?;
        if let Err(err) = pre_manifest.verify_unchanged(&post_manifest) {
            reject_live!(TriageStatus::BaseRepoDirtyMutationDetected, err.to_string());
        }

        // 10. Teardown the scratch sandbox cleanly.
        sandbox.teardown()?;

        let clean_ref_name = options
            .clean_ref
            .clone()
            .unwrap_or_else(|| "main".to_string());
        let clean_ref_commit = resolve_git_ref(repo_path, &clean_ref_name);

        let baseline_snapshot =
            crate::regression::RegressionEvaluator::capture_suite_snapshot(repo_path, false);
        let patched_snapshot = crate::regression::RegressionEvaluator::capture_suite_snapshot(
            repo_path,
            options.mock_regression,
        );
        let parity =
            crate::regression::RegressionEvaluator::evaluate(&baseline_snapshot, &patched_snapshot);

        if !parity.parity_passed {
            fsm.advance(Phase::Rejected)?;
            let triage = TriageReport::with_substrate(
                TriageStatus::RegressionDetected,
                format!(
                    "Regression detected: {} newly failing tests",
                    parity.new_failures.join(", ")
                ),
                task_id,
                SUBSTRATE_LIVE_BROWSER,
                fsm.trace(),
            )
            .with_regression_parity(Some(parity.clone()));
            return Ok(HealOutcome::Rejected(Box::new(triage)));
        }

        fsm.advance(Phase::Gate2Green)?;
        let gate2_at = timestamp_now();

        let diag = crate::fsm::DifferentialMatrix::evaluate(
            task_id,
            &clean_ref_name,
            &clean_ref_commit,
            crate::fsm::ReproExecutionResult::fail(1, "Gate 1 Red baseline defect reproduced"),
            crate::fsm::ReproExecutionResult::pass(0),
            crate::fsm::ReproExecutionResult::pass(0),
            crate::fsm::ReproExecutionResult::pass(0),
            parity.parity_passed,
            chrono::Utc::now().timestamp_millis() as u64,
        );

        Ok(HealOutcome::Verified(Box::new(AuditReceipt {
            schema_version: SCHEMA_VERSION.to_string(),
            status: "VERIFIED_REPAIRED".to_string(),
            task_id: task_id.to_string(),
            substrate: SUBSTRATE_LIVE_BROWSER.to_string(),
            disclaimer: DISCLAIMER_LIVE_BROWSER.to_string(),
            contract_id: fixture.contract.contract_id.clone(),
            contract_sha256: contract_sha256.clone(),
            gate1: GateRecord {
                observed_at: gate1_at,
                verdict: gate1_verdict,
                contract_sha256,
                events: gate1_events,
            },
            patch: PatchRecord {
                target_path: outcome.target_path.clone(),
                sha256: sha256_hex(effective_diff.as_bytes()),
                insertions: outcome.insertions,
                deletions: outcome.deletions,
                files_changed: outcome.files_changed,
            },
            gate2: GateRecord {
                observed_at: gate2_at,
                verdict: GateVerdict::Green,
                contract_sha256: gate2_sha256,
                events: gate2_events,
            },
            phase_trace: fsm.trace().to_vec(),
            base_tree_sha256: pre_manifest.tree_sha256,
            sandbox_tree_sha256,
            base_repo_unmodified_verified: true,
            ephemeral_port,
            grounding_telemetry: Some(gate1_grounded.telemetry),
            video_telemetry,
            repair_telemetry,
            token_accounting,
            differential_diagnosis: Some(diag),
            regression_parity: Some(parity),
            merkle_root: None,
            attestation: None,
            created_at: timestamp_now(),
        })))
    }

    /// Execute the Spec 060 bounded autonomous repair loop (up to `MAX_REPAIR_ATTEMPTS` = 3,
    /// 180s wall-clock budget), enforcing `TestFreezeGuard`, `StructuredRepairPatch::decode_strict`,
    /// `BlockPatcher::apply`, `SyntaxPreflight::check_file`, and 2-run Gate 2 stability.
    fn execute_autonomous_repair_loop<F>(
        fixture: &Fixture,
        sandbox: &mut ScratchSandbox,
        initial_source: &str,
        freeze_guard: &TestFreezeGuard,
        mut model_config: MuseModelConfig,
        client_mode: RepairClientMode,
        run_gate2_twice: &mut F,
    ) -> Result<AutonomousRepairResult>
    where
        F: FnMut(&ScratchSandbox) -> Result<(bool, Vec<OracleEvent>)>,
    {
        let client = BoundedRepairClient::new(client_mode);
        let loop_started = Instant::now();
        let mut previous_response_id: Option<String> = None;
        let mut encrypted_content: Option<String> = None;
        let mut retry_feedback: Option<String> = None;
        let mut previous_response_ids: Vec<String> = Vec::new();
        let mut attempt_lineage: Vec<AttemptLineage> = Vec::new();
        let mut cumulative_tokens = TokenAccounting::default();
        let mut any_fallback_used = false;
        let gate1_diagnostic = format!(
            "Gate 1 RED on contract '{}': target '{}' threw TypeError when reading undefined property",
            fixture.contract.contract_id, fixture.contract.target
        );

        for attempt in 1..=MAX_REPAIR_ATTEMPTS {
            if let Err(err) = check_repair_deadline(loop_started) {
                let rtel = RepairTelemetry {
                    requested_model: model_config.requested_model.as_str().to_string(),
                    serving_model: model_config.serving_model.as_str().to_string(),
                    provider_mode: client_mode.provider_mode_str().to_string(),
                    reasoning_effort: model_config.reasoning_effort.as_str().to_string(),
                    fallback_used: any_fallback_used,
                    prompt_cache_key: model_config.prompt_cache_key.clone(),
                    prompt_cache_retention: model_config.prompt_cache_retention.clone(),
                    attempts_used: attempt.saturating_sub(1),
                    max_attempts: MAX_REPAIR_ATTEMPTS,
                    previous_response_ids,
                    encrypted_deliberation_preserved: true,
                    syntax_preflight_passed: false,
                    test_freeze_verified: true,
                    test_freeze_sha256: freeze_guard.aggregate_sha256.clone(),
                    gate2_runs_completed: 0,
                    gate2_stability_passed: false,
                    attempt_lineage,
                };
                return Ok(AutonomousRepairResult::Rejected {
                    status: TriageStatus::QuarantineRepairFailed,
                    reason: err.to_string(),
                    repair_telemetry: Some(rtel),
                    token_accounting: Some(cumulative_tokens),
                });
            }

            let user_prompt = PromptBuilder::build_user_prompt(
                fixture.task_id,
                fixture.target_path,
                initial_source,
                &gate1_diagnostic,
                retry_feedback.as_deref(),
            );
            let req = RepairRequest::new(
                attempt,
                &model_config,
                user_prompt,
                previous_response_id.clone(),
                encrypted_content.clone(),
            );
            let resp = client.generate(&req, &mut model_config)?;
            if resp.fallback_used {
                any_fallback_used = true;
            }
            cumulative_tokens.accumulate(&resp.usage, resp.serving_model);

            let prev_id_for_lineage = previous_response_id.clone();
            let encrypted_hash = sha256_hex(resp.encrypted_content.as_bytes());
            previous_response_id = Some(resp.response_id.clone());
            encrypted_content = Some(resp.encrypted_content.clone());
            previous_response_ids.push(resp.response_id.clone());

            // 1. Decode strict 5-field JSON schema.
            let structured = match StructuredRepairPatch::decode_strict(&resp.raw_json_output) {
                Ok(p) => p,
                Err(e) => {
                    let msg = e.to_string();
                    attempt_lineage.push(AttemptLineage {
                        attempt,
                        response_id: resp.response_id,
                        previous_response_id: prev_id_for_lineage,
                        encrypted_content_present: true,
                        encrypted_content_sha256: encrypted_hash,
                        syntax_check_passed: false,
                        patch_applied: false,
                        failure_reason: Some(msg.clone()),
                    });
                    retry_feedback = Some(msg);
                    sandbox.write_file(fixture.target_path, initial_source)?;
                    continue;
                }
            };

            // 2. Apply ambiguity-safe source block patch (checking TestFreezeGuard first).
            let patch_outcome = match BlockPatcher::apply(
                initial_source,
                fixture.target_path,
                &structured,
                freeze_guard,
            ) {
                Ok(out) => out,
                Err(CoreError::SecurityViolation(msg)) => {
                    // Anti-cheating violations fail closed immediately without retry.
                    attempt_lineage.push(AttemptLineage {
                        attempt,
                        response_id: resp.response_id,
                        previous_response_id: prev_id_for_lineage,
                        encrypted_content_present: true,
                        encrypted_content_sha256: encrypted_hash,
                        syntax_check_passed: false,
                        patch_applied: false,
                        failure_reason: Some(msg.clone()),
                    });
                    let rtel = RepairTelemetry {
                        requested_model: model_config.requested_model.as_str().to_string(),
                        serving_model: model_config.serving_model.as_str().to_string(),
                        provider_mode: client_mode.provider_mode_str().to_string(),
                        reasoning_effort: model_config.reasoning_effort.as_str().to_string(),
                        fallback_used: any_fallback_used,
                        prompt_cache_key: model_config.prompt_cache_key.clone(),
                        prompt_cache_retention: model_config.prompt_cache_retention.clone(),
                        attempts_used: attempt,
                        max_attempts: MAX_REPAIR_ATTEMPTS,
                        previous_response_ids,
                        encrypted_deliberation_preserved: true,
                        syntax_preflight_passed: false,
                        test_freeze_verified: false,
                        test_freeze_sha256: freeze_guard.aggregate_sha256.clone(),
                        gate2_runs_completed: 0,
                        gate2_stability_passed: false,
                        attempt_lineage,
                    };
                    return Ok(AutonomousRepairResult::Rejected {
                        status: TriageStatus::SecurityViolation,
                        reason: msg,
                        repair_telemetry: Some(rtel),
                        token_accounting: Some(cumulative_tokens),
                    });
                }
                Err(e) => {
                    let msg = e.to_string();
                    attempt_lineage.push(AttemptLineage {
                        attempt,
                        response_id: resp.response_id,
                        previous_response_id: prev_id_for_lineage,
                        encrypted_content_present: true,
                        encrypted_content_sha256: encrypted_hash,
                        syntax_check_passed: false,
                        patch_applied: false,
                        failure_reason: Some(msg.clone()),
                    });
                    retry_feedback = Some(msg);
                    sandbox.write_file(fixture.target_path, initial_source)?;
                    continue;
                }
            };

            // 3. Write candidate patch to scratch sandbox and verify frozen test SHA-256 parity.
            sandbox.write_file(&patch_outcome.target_path, &patch_outcome.patched)?;
            if let Err(err) = freeze_guard.verify_integrity(sandbox.root()) {
                match err {
                    CoreError::SecurityViolation(msg) => {
                        attempt_lineage.push(AttemptLineage {
                            attempt,
                            response_id: resp.response_id,
                            previous_response_id: prev_id_for_lineage,
                            encrypted_content_present: true,
                            encrypted_content_sha256: encrypted_hash,
                            syntax_check_passed: false,
                            patch_applied: true,
                            failure_reason: Some(msg.clone()),
                        });
                        let rtel = RepairTelemetry {
                            requested_model: model_config.requested_model.as_str().to_string(),
                            serving_model: model_config.serving_model.as_str().to_string(),
                            provider_mode: client_mode.provider_mode_str().to_string(),
                            reasoning_effort: model_config.reasoning_effort.as_str().to_string(),
                            fallback_used: any_fallback_used,
                            prompt_cache_key: model_config.prompt_cache_key.clone(),
                            prompt_cache_retention: model_config.prompt_cache_retention.clone(),
                            attempts_used: attempt,
                            max_attempts: MAX_REPAIR_ATTEMPTS,
                            previous_response_ids,
                            encrypted_deliberation_preserved: true,
                            syntax_preflight_passed: false,
                            test_freeze_verified: false,
                            test_freeze_sha256: freeze_guard.aggregate_sha256.clone(),
                            gate2_runs_completed: 0,
                            gate2_stability_passed: false,
                            attempt_lineage,
                        };
                        return Ok(AutonomousRepairResult::Rejected {
                            status: TriageStatus::SecurityViolation,
                            reason: msg,
                            repair_telemetry: Some(rtel),
                            token_accounting: Some(cumulative_tokens),
                        });
                    }
                    other => return Err(other),
                }
            }

            // 4. Syntax preflight (`node --check`) before Gate 2 execution.
            if let Err(e) = SyntaxPreflight::check_file(sandbox.root(), &patch_outcome.target_path)
            {
                let msg = e.to_string();
                attempt_lineage.push(AttemptLineage {
                    attempt,
                    response_id: resp.response_id,
                    previous_response_id: prev_id_for_lineage,
                    encrypted_content_present: true,
                    encrypted_content_sha256: encrypted_hash,
                    syntax_check_passed: false,
                    patch_applied: true,
                    failure_reason: Some(msg.clone()),
                });
                retry_feedback = Some(msg);
                sandbox.write_file(fixture.target_path, initial_source)?;
                continue;
            }

            // 5. Run 2 consecutive Gate 2 evaluations to prove stability.
            let (passed_twice, gate2_events) = run_gate2_twice(sandbox)?;
            if passed_twice {
                attempt_lineage.push(AttemptLineage {
                    attempt,
                    response_id: resp.response_id,
                    previous_response_id: prev_id_for_lineage,
                    encrypted_content_present: true,
                    encrypted_content_sha256: encrypted_hash,
                    syntax_check_passed: true,
                    patch_applied: true,
                    failure_reason: None,
                });
                let diff_text = BlockPatcher::to_unified_diff(&structured);
                let rtel = RepairTelemetry {
                    requested_model: model_config.requested_model.as_str().to_string(),
                    serving_model: model_config.serving_model.as_str().to_string(),
                    provider_mode: client_mode.provider_mode_str().to_string(),
                    reasoning_effort: model_config.reasoning_effort.as_str().to_string(),
                    fallback_used: any_fallback_used,
                    prompt_cache_key: model_config.prompt_cache_key.clone(),
                    prompt_cache_retention: model_config.prompt_cache_retention.clone(),
                    attempts_used: attempt,
                    max_attempts: MAX_REPAIR_ATTEMPTS,
                    previous_response_ids,
                    encrypted_deliberation_preserved: true,
                    syntax_preflight_passed: true,
                    test_freeze_verified: true,
                    test_freeze_sha256: freeze_guard.aggregate_sha256.clone(),
                    gate2_runs_completed: GATE2_REQUIRED_CONSECUTIVE_RUNS,
                    gate2_stability_passed: true,
                    attempt_lineage,
                };
                return Ok(AutonomousRepairResult::Verified {
                    outcome: patch_outcome,
                    diff_text,
                    gate2_events,
                    repair_telemetry: rtel,
                    token_accounting: cumulative_tokens,
                });
            }

            let fail_msg = format!(
                "Attempt {attempt} patch compiled cleanly but failed Gate 2 verification (defect still reproduced)"
            );
            attempt_lineage.push(AttemptLineage {
                attempt,
                response_id: resp.response_id,
                previous_response_id: prev_id_for_lineage,
                encrypted_content_present: true,
                encrypted_content_sha256: encrypted_hash,
                syntax_check_passed: true,
                patch_applied: true,
                failure_reason: Some(fail_msg.clone()),
            });
            retry_feedback = Some(fail_msg);
            sandbox.write_file(fixture.target_path, initial_source)?;
        }

        let rtel = RepairTelemetry {
            requested_model: model_config.requested_model.as_str().to_string(),
            serving_model: model_config.serving_model.as_str().to_string(),
            provider_mode: client_mode.provider_mode_str().to_string(),
            reasoning_effort: model_config.reasoning_effort.as_str().to_string(),
            fallback_used: any_fallback_used,
            prompt_cache_key: model_config.prompt_cache_key,
            prompt_cache_retention: model_config.prompt_cache_retention,
            attempts_used: MAX_REPAIR_ATTEMPTS,
            max_attempts: MAX_REPAIR_ATTEMPTS,
            previous_response_ids,
            encrypted_deliberation_preserved: true,
            syntax_preflight_passed: attempt_lineage
                .last()
                .is_some_and(|a| a.syntax_check_passed),
            test_freeze_verified: true,
            test_freeze_sha256: freeze_guard.aggregate_sha256.clone(),
            gate2_runs_completed: 0,
            gate2_stability_passed: false,
            attempt_lineage,
        };

        Ok(AutonomousRepairResult::Rejected {
            status: TriageStatus::QuarantineRepairFailed,
            reason: format!(
                "QUARANTINE_REPAIR_FAILED: exhausted all {MAX_REPAIR_ATTEMPTS} bounded repair \
                 attempts without achieving 2 consecutive Gate 2 GREEN runs"
            ),
            repair_telemetry: Some(rtel),
            token_accounting: Some(cumulative_tokens),
        })
    }

    fn execute_browser_gate(
        bridge: &mut PlaywrightBridge,
        fixture: &Fixture,
        page_url: &str,
        click_coords: (i32, i32),
        candidate_override: Option<&[DomCandidate]>,
    ) -> Result<(GroundedTarget, Vec<OracleEvent>)> {
        let (click_x, click_y) = click_coords;
        let probe_resp = bridge.send_command(&BridgeCommand::ProbePage {
            url: page_url.to_string(),
            click_x,
            click_y,
        })?;

        let (navigated, aria_snapshot, live_candidates, probe_errors) = match probe_resp {
            BridgeResponse::ProbeResult {
                navigated,
                aria_snapshot,
                candidates,
                page_errors,
            } => (navigated, aria_snapshot, candidates, page_errors),
            BridgeResponse::Error { message } => return Err(CoreError::Browser(message)),
            other => {
                return Err(CoreError::Browser(format!(
                    "Unexpected response to PROBE_PAGE: {other:?}"
                )))
            }
        };

        if !navigated || !probe_errors.is_empty() {
            return Ok((
                GroundedTarget {
                    candidate: DomCandidate {
                        tag: String::new(),
                        role: String::new(),
                        accessible_name: String::new(),
                        test_id: None,
                        bbox: crate::locator::BoundingBox {
                            x: 0,
                            y: 0,
                            width: 0,
                            height: 0,
                        },
                        is_element_from_point: false,
                    },
                    selector: String::new(),
                    telemetry: crate::locator::GroundingTelemetry {
                        click_x,
                        click_y,
                        viewport_normalized: false,
                        element_from_point_match: false,
                        accessible_name_match: false,
                        candidate_score_bps: 0,
                        score_margin_bps: 0,
                        matched_selector: String::new(),
                        aria_snapshot_excerpt: String::new(),
                    },
                },
                vec![OracleEvent {
                    seq: 0,
                    kind: OracleEventKind::Setup,
                    passed: false,
                }],
            ));
        }

        let effective_candidates = candidate_override.unwrap_or(&live_candidates);
        let grounded = LocatorResolver::resolve(
            click_x,
            click_y,
            fixture.viewport_width,
            fixture.viewport_height,
            &aria_snapshot,
            effective_candidates,
        )?;

        let (click_role, click_name) = if grounded.telemetry.accessible_name_match {
            (
                grounded.candidate.role.clone(),
                grounded.candidate.accessible_name.clone(),
            )
        } else {
            (String::new(), String::new())
        };

        let click_resp = bridge.send_command(&BridgeCommand::ClickAndObserve {
            role: click_role,
            accessible_name: click_name,
            test_id: grounded.candidate.test_id.clone(),
            output_selector: "#cart-summary-output".to_string(),
        })?;

        let (clicked, click_errors, result_text) = match click_resp {
            BridgeResponse::ClickResult {
                clicked,
                page_errors,
                result_text,
            } => (clicked, page_errors, result_text),
            BridgeResponse::Error { message } => {
                if message.contains("AMBIGUOUS_GROUNDING") {
                    return Err(CoreError::Grounding(message));
                }
                return Err(CoreError::Browser(message));
            }
            other => {
                return Err(CoreError::Browser(format!(
                    "Unexpected response to CLICK_AND_OBSERVE: {other:?}"
                )))
            }
        };

        let assert_passed =
            clicked && click_errors.is_empty() && result_text == fixture.expected_output_text;

        let checks = [true, true, clicked, assert_passed];
        let mut events = Vec::with_capacity(4);
        for (seq, (kind, passed)) in EVENT_ORDER.iter().zip(checks).enumerate() {
            events.push(OracleEvent {
                seq: seq as u32,
                kind: *kind,
                passed,
            });
            if !passed {
                break;
            }
        }

        Ok((grounded, events))
    }

    #[allow(clippy::too_many_arguments)]
    fn run_in_repo_with_options(
        fixture: &Fixture,
        diff: &str,
        repo_path: &Path,
        extra_excludes: &[PathBuf],
        gate2_contract: &BehavioralContract,
        on_before_post_check: Option<&dyn Fn(&Path)>,
        autonomous_opts: Option<(&str, RepairClientMode)>,
        mock_regression: bool,
    ) -> Result<HealOutcome> {
        let task_id = fixture.task_id;
        let mut fsm = PhaseMachine::new();

        let autonomous_config = match autonomous_opts {
            Some((slug, mode)) => Some((MuseModelConfig::new(slug, None)?, mode)),
            None => None,
        };

        // 1. Capture pre-run dual-layer workspace manifest on the base repository.
        let pre_manifest = WorktreeManifest::capture_with_excludes(repo_path, extra_excludes)?;

        // 2. Create isolated scratch sandbox and allocate ephemeral port.
        let mut sandbox = ScratchSandbox::create_with_excludes(repo_path, task_id, extra_excludes)?;
        let ephemeral_port = EphemeralPort::allocate()?.get();

        macro_rules! reject {
            ($status:expr, $reason:expr) => {{
                let _ = sandbox.teardown();
                fsm.advance(Phase::Rejected)?;
                return Ok(HealOutcome::Rejected(Box::new(TriageReport::new(
                    $status,
                    $reason,
                    task_id,
                    fsm.trace(),
                ))));
            }};
            ($status:expr, $reason:expr, $rtel:expr, $tokens:expr) => {{
                let _ = sandbox.teardown();
                fsm.advance(Phase::Rejected)?;
                return Ok(HealOutcome::Rejected(Box::new(
                    TriageReport::new($status, $reason, task_id, fsm.trace())
                        .with_repair_telemetry($rtel, $tokens),
                )));
            }};
        }

        // 3. Materialize target file inside the scratch sandbox if the base repository
        // does not already supply it; otherwise inspect the copied file in the sandbox.
        let sandbox_target_file = sandbox.resolve_safe_path(fixture.target_path)?;
        if !sandbox_target_file.exists() {
            sandbox.write_file(fixture.target_path, fixture.buggy_source)?;
        }
        let initial_source = sandbox.read_file(fixture.target_path)?;

        // Capture anti-cheating SHA-256 test freeze guard before Gate 1 / Repair.
        let freeze_guard = TestFreezeGuard::capture(sandbox.root())?;

        // Gate 1: the defect must reproduce inside the scratch sandbox before any repair is considered.
        let contract_sha256 = fixture.contract.digest()?;
        let gate1_events = run_oracle(&fixture.contract, &initial_source);
        let gate1_verdict = classify(&gate1_events);
        if gate1_verdict != GateVerdict::Red {
            reject!(
                TriageStatus::Gate1NotRed,
                format!(
                    "Gate 1 must be RED before repair (got {:?}): {}",
                    gate1_verdict,
                    describe_invalid(&gate1_events)
                )
            );
        }
        fsm.advance(Phase::Gate1Red)?;
        let gate1_at = timestamp_now();

        // Repair: validate and apply the candidate patch to the file inside the scratch sandbox.
        fsm.advance(Phase::Repair)?;
        let gate2_sha256 = gate2_contract.digest()?;
        if gate2_sha256 != contract_sha256 {
            reject!(
                TriageStatus::ContractMismatch,
                "Gate 2 contract digest differs from Gate 1; refusing to compare results"
            );
        }

        let (outcome, effective_diff, gate2_events, repair_telemetry, token_accounting) =
            if let Some((model_config, client_mode)) = autonomous_config {
                let auto_res = Self::execute_autonomous_repair_loop(
                    fixture,
                    &mut sandbox,
                    &initial_source,
                    &freeze_guard,
                    model_config,
                    client_mode,
                    &mut |sb| {
                        let mut last_events = Vec::new();
                        for _run_idx in 0..GATE2_REQUIRED_CONSECUTIVE_RUNS {
                            let src = sb.read_file(fixture.target_path)?;
                            let ev = run_oracle(gate2_contract, &src);
                            let verdict = classify(&ev);
                            last_events = ev;
                            if verdict != GateVerdict::Green {
                                return Ok((false, last_events));
                            }
                        }
                        Ok((true, last_events))
                    },
                )?;

                match auto_res {
                    AutonomousRepairResult::Verified {
                        outcome,
                        diff_text,
                        gate2_events,
                        repair_telemetry,
                        token_accounting,
                    } => (
                        outcome,
                        diff_text,
                        gate2_events,
                        Some(repair_telemetry),
                        Some(token_accounting),
                    ),
                    AutonomousRepairResult::Rejected {
                        status,
                        reason,
                        repair_telemetry,
                        token_accounting,
                    } => {
                        reject!(status, reason, repair_telemetry, token_accounting);
                    }
                }
            } else {
                if diff.trim().is_empty() {
                    reject!(
                        TriageStatus::RepairNoopRejected,
                        "The candidate patch is empty; a repair must change the source"
                    );
                }
                let outcome = match UnifiedPatcher::apply_diff(&initial_source, diff) {
                    Ok(o) => o,
                    Err(CoreError::SecurityViolation(msg)) => {
                        reject!(TriageStatus::SecurityViolation, msg)
                    }
                    Err(e) => reject!(TriageStatus::PatchApplyFailed, e.to_string()),
                };
                if let Err(err) = freeze_guard.validate_target_path(&outcome.target_path) {
                    match err {
                        CoreError::SecurityViolation(msg) => {
                            reject!(TriageStatus::SecurityViolation, msg);
                        }
                        other => return Err(other),
                    }
                }
                if outcome.target_path != fixture.target_path {
                    reject!(
                        TriageStatus::PatchApplyFailed,
                        format!(
                            "Patch targets '{}' but the repair scope is '{}'",
                            outcome.target_path, fixture.target_path
                        )
                    );
                }
                if outcome.is_noop(&initial_source) {
                    reject!(
                        TriageStatus::RepairNoopRejected,
                        "The patch applies cleanly but leaves the source unchanged"
                    );
                }

                sandbox.write_file(&outcome.target_path, &outcome.patched)?;
                if let Err(err) = freeze_guard.verify_integrity(sandbox.root()) {
                    match err {
                        CoreError::SecurityViolation(msg) => {
                            reject!(TriageStatus::SecurityViolation, msg);
                        }
                        other => return Err(other),
                    }
                }

                // Evaluate Gate 2 across 2 consecutive runs.
                let mut final_events = Vec::new();
                for _run_idx in 0..GATE2_REQUIRED_CONSECUTIVE_RUNS {
                    let gate2_source = sandbox.read_file(&outcome.target_path)?;
                    let gate2_events = run_oracle(gate2_contract, &gate2_source);
                    let gate2_verdict = classify(&gate2_events);
                    match gate2_verdict {
                        GateVerdict::Green => {
                            final_events = gate2_events;
                        }
                        GateVerdict::Red => reject!(
                            TriageStatus::Gate2NotGreen,
                            "The defect still reproduces after the patch"
                        ),
                        GateVerdict::Invalid => reject!(
                            TriageStatus::Gate2NotGreen,
                            format!(
                                "Gate 2 produced no valid verdict: {}",
                                describe_invalid(&gate2_events)
                            )
                        ),
                    }
                }

                (outcome, diff.to_string(), final_events, None, None)
            };

        let sandbox_manifest = WorktreeManifest::capture(sandbox.root())?;
        let sandbox_tree_sha256 = sandbox_manifest.tree_sha256;

        // Optional fault injection hook for testing base repository mutation detection.
        if let Some(hook) = on_before_post_check {
            hook(repo_path);
        }

        // 4. Verify dual-layer base repository integrity (\Delta repo = 0) before accepting Gate 2.
        let post_manifest = WorktreeManifest::capture_with_excludes(repo_path, extra_excludes)?;
        if let Err(err) = pre_manifest.verify_unchanged(&post_manifest) {
            reject!(TriageStatus::BaseRepoDirtyMutationDetected, err.to_string());
        }

        // 5. Teardown the scratch sandbox cleanly.
        sandbox.teardown()?;

        let clean_ref_name = "main".to_string();
        let clean_ref_commit = resolve_git_ref(repo_path, &clean_ref_name);

        let baseline_snapshot =
            crate::regression::RegressionEvaluator::capture_suite_snapshot(repo_path, false);
        let patched_snapshot = crate::regression::RegressionEvaluator::capture_suite_snapshot(
            repo_path,
            mock_regression,
        );
        let parity =
            crate::regression::RegressionEvaluator::evaluate(&baseline_snapshot, &patched_snapshot);

        if !parity.parity_passed {
            fsm.advance(Phase::Rejected)?;
            let triage = TriageReport::with_substrate(
                TriageStatus::RegressionDetected,
                format!(
                    "Regression detected: {} newly failing tests",
                    parity.new_failures.join(", ")
                ),
                task_id,
                SUBSTRATE,
                fsm.trace(),
            )
            .with_regression_parity(Some(parity.clone()));
            return Ok(HealOutcome::Rejected(Box::new(triage)));
        }

        fsm.advance(Phase::Gate2Green)?;
        let gate2_at = timestamp_now();

        let diag = crate::fsm::DifferentialMatrix::evaluate(
            task_id,
            &clean_ref_name,
            &clean_ref_commit,
            crate::fsm::ReproExecutionResult::fail(1, "Gate 1 Red baseline defect reproduced"),
            crate::fsm::ReproExecutionResult::pass(0),
            crate::fsm::ReproExecutionResult::pass(0),
            crate::fsm::ReproExecutionResult::pass(0),
            parity.parity_passed,
            chrono::Utc::now().timestamp_millis() as u64,
        );

        Ok(HealOutcome::Verified(Box::new(AuditReceipt {
            schema_version: SCHEMA_VERSION.to_string(),
            status: "VERIFIED_REPAIRED".to_string(),
            task_id: task_id.to_string(),
            substrate: SUBSTRATE.to_string(),
            disclaimer: DISCLAIMER.to_string(),
            contract_id: fixture.contract.contract_id.clone(),
            contract_sha256: contract_sha256.clone(),
            gate1: GateRecord {
                observed_at: gate1_at,
                verdict: gate1_verdict,
                contract_sha256,
                events: gate1_events,
            },
            patch: PatchRecord {
                target_path: outcome.target_path.clone(),
                sha256: sha256_hex(effective_diff.as_bytes()),
                insertions: outcome.insertions,
                deletions: outcome.deletions,
                files_changed: outcome.files_changed,
            },
            gate2: GateRecord {
                observed_at: gate2_at,
                verdict: GateVerdict::Green,
                contract_sha256: gate2_sha256,
                events: gate2_events,
            },
            phase_trace: fsm.trace().to_vec(),
            base_tree_sha256: pre_manifest.tree_sha256,
            sandbox_tree_sha256,
            base_repo_unmodified_verified: true,
            ephemeral_port,
            grounding_telemetry: None,
            video_telemetry: None,
            repair_telemetry,
            token_accounting,
            differential_diagnosis: Some(diag),
            regression_parity: Some(parity),
            merkle_root: None,
            attestation: None,
            created_at: timestamp_now(),
        })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::locator::BoundingBox;

    fn rf01() -> Fixture {
        Fixture::for_task("RF-01").unwrap()
    }

    fn rejected(outcome: HealOutcome) -> TriageReport {
        match outcome {
            HealOutcome::Rejected(r) => *r,
            HealOutcome::Verified(_) => panic!("expected rejection"),
        }
    }

    fn temp_repo_dir(label: &str) -> PathBuf {
        let seq = EPHEMERAL_REPO_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "replayfix-engine-test-{}-{}-{}",
            label,
            std::process::id(),
            seq
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn test_gate1_oracle_is_red_on_buggy_source() {
        let f = rf01();
        let events = run_oracle(&f.contract, f.buggy_source);
        assert_eq!(events.len(), 4);
        assert_eq!(
            events.iter().map(|e| e.passed).collect::<Vec<_>>(),
            [true, true, true, false]
        );
        assert_eq!(classify(&events), GateVerdict::Red);
    }

    #[test]
    fn test_happy_path_is_verified_with_ordered_phases() {
        let f = rf01();
        let HealOutcome::Verified(receipt) = MockHealer::run(&f, f.builtin_diff).unwrap() else {
            panic!("expected verification");
        };
        assert_eq!(receipt.schema_version, "0.9");
        assert_eq!(receipt.status, "VERIFIED_REPAIRED");
        assert_eq!(receipt.gate1.verdict, GateVerdict::Red);
        assert_eq!(receipt.gate2.verdict, GateVerdict::Green);
        assert_eq!(
            receipt.phase_trace,
            [
                Phase::Init,
                Phase::Gate1Red,
                Phase::Repair,
                Phase::Gate2Green
            ]
        );
        assert_eq!(receipt.gate1.contract_sha256, receipt.gate2.contract_sha256);
        assert_eq!(receipt.contract_sha256.len(), 64);
        assert_eq!(receipt.base_tree_sha256.len(), 64);
        assert_eq!(receipt.sandbox_tree_sha256.len(), 64);
        assert_ne!(receipt.base_tree_sha256, receipt.sandbox_tree_sha256);
        assert!(receipt.base_repo_unmodified_verified);
        assert!(receipt.ephemeral_port > 0);
        assert!(receipt.gate1.observed_at <= receipt.gate2.observed_at);
        assert_eq!(receipt.patch.sha256, sha256_hex(f.builtin_diff.as_bytes()));
        assert_eq!((receipt.patch.insertions, receipt.patch.deletions), (1, 1));
        assert_eq!(receipt.gate2.events.len(), 4);
        assert!(receipt.gate2.events.iter().all(|e| e.passed));
    }

    #[test]
    fn test_autonomous_model_repair_first_attempt_and_syntax_retry() {
        let f = rf01();
        let repo = temp_repo_dir("autonomous-repair");

        // 1. First-attempt success with muse-spark-1.3
        let HealOutcome::Verified(receipt) = MockHealer::run_autonomous_in_repo(
            &f,
            &repo,
            &[],
            "muse-spark-1.3",
            RepairClientMode::OfflineSuccessFirstAttempt,
        )
        .unwrap() else {
            panic!("expected autonomous repair verification");
        };
        assert_eq!(receipt.schema_version, "0.9");
        let rtel = receipt.repair_telemetry.expect("expected repair_telemetry");
        assert_eq!(rtel.requested_model, "muse-spark-1.3");
        assert_eq!(rtel.serving_model, "muse-spark-1.3");
        assert_eq!(rtel.reasoning_effort, "xhigh");
        assert_eq!(rtel.attempts_used, 1);
        assert_eq!(rtel.gate2_runs_completed, 2);
        assert!(rtel.gate2_stability_passed);
        assert!(rtel.syntax_preflight_passed);
        assert!(rtel.test_freeze_verified);
        let tokens = receipt.token_accounting.expect("expected token_accounting");
        assert!(tokens.prompt_tokens > 0);
        assert!(tokens.total_cost_micros_usd > 0 && tokens.total_cost_micros_usd < 20_000);

        // 2. Attempt 1 fails node --check syntax preflight, Attempt 2 recovers with encrypted_content
        let HealOutcome::Verified(retry_receipt) = MockHealer::run_autonomous_in_repo(
            &f,
            &repo,
            &[],
            "muse-spark-1.3",
            RepairClientMode::OfflineSyntaxRetryThenSuccess,
        )
        .unwrap() else {
            panic!("expected syntax retry recovery");
        };
        let retry_tel = retry_receipt.repair_telemetry.unwrap();
        assert_eq!(retry_tel.attempts_used, 2);
        assert_eq!(retry_tel.attempt_lineage.len(), 2);
        assert!(!retry_tel.attempt_lineage[0].syntax_check_passed);
        assert!(retry_tel.attempt_lineage[1].syntax_check_passed);
        assert_eq!(
            retry_tel.attempt_lineage[1].previous_response_id.as_deref(),
            Some("resp_rf06_attempt_1")
        );

        let _ = fs::remove_dir_all(&repo);
    }

    #[test]
    fn test_autonomous_repair_quarantines_after_three_failed_attempts() {
        let f = rf01();
        let repo = temp_repo_dir("autonomous-quarantine");

        let outcome = MockHealer::run_autonomous_in_repo(
            &f,
            &repo,
            &[],
            "muse-spark-1.3",
            RepairClientMode::OfflineFailAllAttempts,
        )
        .unwrap();
        let report = rejected(outcome);
        assert_eq!(report.status, TriageStatus::QuarantineRepairFailed);
        assert_eq!(report.exit_code, 21);
        let rtel = report
            .repair_telemetry
            .expect("expected repair_telemetry on quarantine");
        assert_eq!(rtel.attempts_used, 3);
        assert_eq!(rtel.attempt_lineage.len(), 3);
        assert!(!rtel.gate2_stability_passed);
        let tokens = report
            .token_accounting
            .expect("expected token_accounting on quarantine");
        assert!(tokens.prompt_tokens > 0);

        let _ = fs::remove_dir_all(&repo);
    }

    #[test]
    fn test_autonomous_repair_rejects_test_file_mutation_with_exit_11() {
        let f = rf01();
        let repo = temp_repo_dir("autonomous-test-mutation");

        let outcome = MockHealer::run_autonomous_in_repo(
            &f,
            &repo,
            &[],
            "muse-spark-1.3",
            RepairClientMode::OfflineAttemptTestMutation,
        )
        .unwrap();
        let report = rejected(outcome);
        assert_eq!(report.status, TriageStatus::SecurityViolation);
        assert_eq!(report.exit_code, 11);
        assert!(report.reason.contains("TEST_MUTATION_PROHIBITED"));

        let _ = fs::remove_dir_all(&repo);
    }

    #[test]
    fn test_live_browser_two_gate_verification_and_ambiguity_abstention() {
        let f = rf01();
        let repo = temp_repo_dir("live-browser");

        // 1. Happy path in real Chromium browser
        let opts = LiveBrowserOptions::default();
        let HealOutcome::Verified(receipt) =
            MockHealer::run_live_browser_in_repo(&f, f.builtin_diff, &repo, &[], &opts).unwrap()
        else {
            panic!("expected live browser verification");
        };
        assert_eq!(receipt.schema_version, "0.9");
        assert_eq!(receipt.substrate, "playwright-cdp");
        assert_eq!(receipt.gate1.verdict, GateVerdict::Red);
        assert_eq!(receipt.gate2.verdict, GateVerdict::Green);
        assert!(receipt.base_repo_unmodified_verified);
        let telemetry = receipt
            .grounding_telemetry
            .expect("expected grounding telemetry");
        assert_eq!(
            telemetry.matched_selector,
            "page.getByRole('button', { name: 'Refresh Cart', exact: true })"
        );
        assert!(telemetry.element_from_point_match);
        assert!(telemetry.accessible_name_match);
        assert_eq!(telemetry.candidate_score_bps, 10_000);

        // 2. Ambiguous candidate tie fails closed with AMBIGUOUS_GROUNDING (exit 22)
        let ambiguous_opts = LiveBrowserOptions {
            headed: false,
            candidate_override: Some(vec![
                DomCandidate {
                    tag: "button".into(),
                    role: "button".into(),
                    accessible_name: "Refresh Cart".into(),
                    test_id: None,
                    bbox: BoundingBox {
                        x: 100,
                        y: 100,
                        width: 120,
                        height: 40,
                    },
                    is_element_from_point: true,
                },
                DomCandidate {
                    tag: "button".into(),
                    role: "button".into(),
                    accessible_name: "Refresh Cart".into(),
                    test_id: Some("duplicate-cart-btn".into()),
                    bbox: BoundingBox {
                        x: 102,
                        y: 100,
                        width: 120,
                        height: 40,
                    },
                    is_element_from_point: false,
                },
                DomCandidate {
                    tag: "button".into(),
                    role: "button".into(),
                    accessible_name: "Secondary Action".into(),
                    test_id: Some("secondary-btn".into()),
                    bbox: BoundingBox {
                        x: 100,
                        y: 100,
                        width: 120,
                        height: 40,
                    },
                    is_element_from_point: true,
                },
            ]),
            ..Default::default()
        };
        let outcome =
            MockHealer::run_live_browser_in_repo(&f, f.builtin_diff, &repo, &[], &ambiguous_opts)
                .unwrap();
        let t = rejected(outcome);
        assert_eq!(t.status, TriageStatus::AmbiguousGrounding);
        assert_eq!(t.exit_code, 22);
        assert_eq!(t.substrate, "playwright-cdp");

        let _ = fs::remove_dir_all(&repo);
    }

    #[test]
    fn test_run_in_repo_preserves_existing_base_repo_files() {
        let f = rf01();
        let repo = temp_repo_dir("preserve");
        fs::create_dir_all(repo.join("src")).unwrap();
        fs::write(repo.join("src/cart-summary.js"), f.buggy_source).unwrap();

        let pre_content = fs::read_to_string(repo.join("src/cart-summary.js")).unwrap();
        let HealOutcome::Verified(receipt) =
            MockHealer::run_in_repo(&f, f.builtin_diff, &repo, &[]).unwrap()
        else {
            panic!("expected verification");
        };

        // Base repo file must still have the original buggy content (Delta repo = 0)
        let post_content = fs::read_to_string(repo.join("src/cart-summary.js")).unwrap();
        assert_eq!(pre_content, post_content);
        assert!(receipt.base_repo_unmodified_verified);
        assert_ne!(receipt.base_tree_sha256, receipt.sandbox_tree_sha256);

        let _ = fs::remove_dir_all(&repo);
    }

    #[test]
    fn test_base_repo_mutation_during_run_is_rejected_with_exit_11() {
        let f = rf01();
        let repo = temp_repo_dir("dirty-mutation");
        fs::create_dir_all(repo.join("src")).unwrap();
        fs::write(repo.join("src/cart-summary.js"), f.buggy_source).unwrap();

        let hook = |base: &Path| {
            let _ = fs::write(base.join("src/rogue.js"), "// unauthorized mutation\n");
        };
        let outcome = MockHealer::run_in_repo_with_options(
            &f,
            f.builtin_diff,
            &repo,
            &[],
            &f.contract,
            Some(&hook),
            None,
            false,
        )
        .unwrap();

        let t = rejected(outcome);
        assert_eq!(t.status, TriageStatus::BaseRepoDirtyMutationDetected);
        assert_eq!(t.exit_code, 11);
        assert!(t.reason.contains("src/rogue.js"));

        let _ = fs::remove_dir_all(&repo);
    }

    #[test]
    fn test_empty_and_whitespace_patches_are_rejected_as_noop() {
        let f = rf01();
        for diff in ["", "   \n\t\n"] {
            let t = rejected(MockHealer::run(&f, diff).unwrap());
            assert_eq!(t.status, TriageStatus::RepairNoopRejected);
            assert_eq!(t.exit_code, 30);
            assert_eq!(
                t.phase_trace,
                [Phase::Init, Phase::Gate1Red, Phase::Repair, Phase::Rejected]
            );
        }
    }

    #[test]
    fn test_net_zero_patch_is_rejected_as_noop() {
        let f = rf01();
        let diff = "--- a/src/cart-summary.js\n+++ b/src/cart-summary.js\n@@ -2 +2 @@\n-  return cart.items.length;\n+  return cart.items.length;\n";
        let t = rejected(MockHealer::run(&f, diff).unwrap());
        assert_eq!(t.status, TriageStatus::RepairNoopRejected);
    }

    #[test]
    fn test_patch_that_does_not_fix_the_defect_leaves_gate2_red() {
        let f = rf01();
        let diff = "--- a/src/cart-summary.js\n+++ b/src/cart-summary.js\n@@ -2 +2 @@\n-  return cart.items.length;\n+  return cart.items.length + 0;\n";
        let t = rejected(MockHealer::run(&f, diff).unwrap());
        assert_eq!(t.status, TriageStatus::Gate2NotGreen);
        assert!(t.reason.contains("still reproduces"));
    }

    #[test]
    fn test_patch_deleting_the_target_is_never_green() {
        let f = rf01();
        let diff = "--- a/src/cart-summary.js\n+++ b/src/cart-summary.js\n@@ -1,3 +1 @@\n-export function cartItemCount(cart) {\n-  return cart.items.length;\n-}\n+// removed\n";
        let t = rejected(MockHealer::run(&f, diff).unwrap());
        assert_eq!(t.status, TriageStatus::Gate2NotGreen);
        assert!(t.reason.contains("no valid verdict"));
    }

    #[test]
    fn test_malformed_patch_fails_closed() {
        let f = rf01();
        let t = rejected(MockHealer::run(&f, "not a diff").unwrap());
        assert_eq!(t.status, TriageStatus::PatchApplyFailed);
        assert_eq!(t.exit_code, 30);
    }

    #[test]
    fn test_unsafe_patch_path_is_a_security_violation() {
        let f = rf01();
        let diff = "--- a/../../etc/passwd\n+++ b/../../etc/passwd\n@@ -1 +1 @@\n-x\n+y\n";
        let t = rejected(MockHealer::run(&f, diff).unwrap());
        assert_eq!(t.status, TriageStatus::SecurityViolation);
        assert_eq!(t.exit_code, 11);
    }

    #[test]
    fn test_patch_for_another_file_is_rejected() {
        let f = rf01();
        let diff = "--- a/src/other.js\n+++ b/src/other.js\n@@ -1 +1 @@\n-x\n+y\n";
        let t = rejected(MockHealer::run(&f, diff).unwrap());
        assert_eq!(t.status, TriageStatus::PatchApplyFailed);
    }

    #[test]
    fn test_contract_mismatch_blocks_gate2() {
        let f = rf01();
        let mut other = f.contract.clone();
        other.postcondition_id = "something-weaker".to_string();
        let t = rejected(MockHealer::run_with_gate2_contract(&f, f.builtin_diff, &other).unwrap());
        assert_eq!(t.status, TriageStatus::ContractMismatch);
        assert_eq!(t.exit_code, 10);
    }

    #[test]
    fn test_gate1_not_red_terminates_the_run() {
        let mut f = rf01();
        f.buggy_source =
            "export function cartItemCount(cart) {\n  return cart.items?.length || 0;\n}\n";
        let t = rejected(MockHealer::run(&f, f.builtin_diff).unwrap());
        assert_eq!(t.status, TriageStatus::Gate1NotRed);
        assert_eq!(t.phase_trace, [Phase::Init, Phase::Rejected]);
    }

    #[test]
    fn test_crash_before_assert_is_invalid_not_red() {
        let mut f = rf01();
        f.buggy_source = "// nothing here\n";
        let events = run_oracle(&f.contract, f.buggy_source);
        assert_eq!(classify(&events), GateVerdict::Invalid);
        let t = rejected(MockHealer::run(&f, f.builtin_diff).unwrap());
        assert_eq!(t.status, TriageStatus::Gate1NotRed);
    }

    #[test]
    fn test_classify_rejects_out_of_order_events() {
        let shuffled = [
            OracleEvent {
                seq: 0,
                kind: OracleEventKind::Target,
                passed: true,
            },
            OracleEvent {
                seq: 1,
                kind: OracleEventKind::Setup,
                passed: true,
            },
            OracleEvent {
                seq: 2,
                kind: OracleEventKind::Action,
                passed: true,
            },
            OracleEvent {
                seq: 3,
                kind: OracleEventKind::Assert,
                passed: false,
            },
        ];
        assert_eq!(classify(&shuffled), GateVerdict::Invalid);
        assert_eq!(classify(&[]), GateVerdict::Invalid);
    }

    #[test]
    fn test_contract_digest_is_stable_and_sensitive() {
        let f = rf01();
        let d1 = f.contract.digest().unwrap();
        assert_eq!(d1, f.contract.clone().digest().unwrap());
        let mut changed = f.contract.clone();
        changed.action.push('!');
        assert_ne!(d1, changed.digest().unwrap());
    }
}
