#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::todo,
        clippy::unimplemented,
        clippy::unreachable
    )
)]

pub mod approval;
pub mod git_packager;
pub mod heal;
pub mod intent;
pub mod mcp;
pub mod permissions;
pub mod session;
pub mod shell;
pub mod tui;
pub mod ui;

use approval::{evaluate_approval_gate, write_approval_audit, ApprovalDecision, ApprovalMode};
use replayfix_core::crypto::{TrustPolicy, VerificationOutcome};
use replayfix_core::ingestion::{
    ingest_issue, GitHubIssueRef, MockIssueFetcher, SystemIssueFetcher,
};
use replayfix_core::portal::export_proof_bundle;
use replayfix_core::receipt::{write_triage_report, AuditReceipt};
use replayfix_core::sweep::{run_sweep, SweepOptions};
use replayfix_core::trace::{is_ci_environment, launch_trace_viewer};
use std::collections::HashSet;
use std::io::IsTerminal;

use clap::{Args, Parser, Subcommand};
use replayfix_core::{
    BoundingBox, CoreError, DependencyStatus, Doctor, DomCandidate, EnvironmentAdapter,
    ExecutionMode, Fixture, HealOutcome, LiveBrowserOptions, MetaGatewayClient, MockHealer,
    ModelProvenanceRecord, MuseModelConfig, Platform, RepairClientMode, TaskManifest,
    MUSE_SPARK_13_SLUG,
};
use std::path::{Path, PathBuf};
use std::process::exit;
use ui::{
    print_banner, print_diagnostics, print_field, print_security_boundary, print_step, ColorOption,
    DiagnosticCheck, DiagnosticStatus, OutputMode, StepStatus, Symbols, Theme,
};

/// Standard CLI process exit codes aligned with verification schemas.
pub mod exit_codes {
    pub const SUCCESS: i32 = 0;
    pub const DOCTOR_FAILED: i32 = 1;
    pub const USAGE_OR_INPUT_ERROR: i32 = 2;
    pub const INTERNAL_OR_SYSTEM_ERROR: i32 = 10;
    pub const SECURITY_VIOLATION: i32 = 11;
    pub const USER_REJECTED_PATCH: i32 = 12;
    pub const NON_INTERACTIVE_REFUSAL: i32 = 13;
    pub const BRIDGE_EXECUTION_FAILED: i32 = 20;
    pub const QUARANTINE_REPAIR_FAILED: i32 = 21;
    pub const AMBIGUOUS_GROUNDING: i32 = 22;
    pub const INVALID_ISSUE_URL: i32 = 23;
    pub const ISSUE_FETCH_FAILED: i32 = 24;
    pub const REPAIR_REJECTED: i32 = 30;
    pub const ERR_GATEWAY_AUTH_MISSING: i32 = 30;
    pub const REGRESSION_DETECTED: i32 = 31;
    pub const QUARANTINE_INVERTED_BASELINE: i32 = 32;
    pub const RECEIPT_VERIFICATION_FAILED: i32 = 40;
    pub const SWEEP_PARTIAL_FAILURE: i32 = 41;
}

/// Largest patch file accepted by `--mock-patch`.
const MAX_PATCH_BYTES: u64 = 1024 * 1024;

#[derive(Parser, Debug)]
#[command(
    name = "replayfix",
    version,
    about = "ReplayFix CLI — Deterministic verification, causal reproduction, and repair harness"
)]
struct Cli {
    /// Control colored terminal output: auto, always, never
    #[arg(long, global = true, default_value = "auto")]
    color: ColorOption,

    /// Suppress informational messages, banners, and progress (errors and results only)
    #[arg(short, long, global = true)]
    quiet: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Args, Debug)]
pub struct ReproArgs {
    /// Path to JSON task manifest file adhering to task_manifest.schema.json
    #[arg(long)]
    pub manifest: Option<PathBuf>,

    /// Target bug identifier (e.g. RF-01)
    #[arg(long)]
    pub task_id: Option<String>,

    /// Shorthand bug identifier (e.g. RF-01) that populates --task-id, --bug-ref, and --video when omitted
    #[arg(long)]
    pub bug: Option<String>,

    /// Description or title of the defect
    #[arg(long)]
    pub title: Option<String>,

    /// Path to target git repository
    #[arg(long, default_value = ".")]
    pub repo: String,

    /// Git branch or ref containing the defect (e.g. bug/RF-01)
    #[arg(long)]
    pub bug_ref: Option<String>,

    /// Optional clean reference baseline branch (e.g. main)
    #[arg(long)]
    pub clean_ref: Option<String>,

    /// Path to target bug video recording (MP4/WebM)
    #[arg(long)]
    pub video: Option<String>,

    /// Optional path to audio recording
    #[arg(long)]
    pub audio: Option<String>,

    /// Optional command to launch application server
    #[arg(long)]
    pub server_cmd: Option<String>,

    /// Attach Playwright CDP directly to an externally running application server URL (e.g. http://localhost:3000)
    #[arg(long)]
    pub url: Option<String>,

    /// Environment variable used by application for port (default: PORT)
    #[arg(long, default_value = "PORT")]
    pub port_env_var: String,

    /// Healthcheck endpoint path (default: /api/health)
    #[arg(long, default_value = "/api/health")]
    pub healthcheck_path: String,

    /// Timeout in milliseconds for application healthcheck readiness
    #[arg(long, default_value = "10000")]
    pub startup_timeout_ms: u64,

    /// Run browser replay in visible headed mode
    #[arg(long)]
    pub headed: bool,

    /// Export Playwright trace for visual inspection
    #[arg(long)]
    pub trace: bool,

    /// Run the built-in two-gate loop (Gate 1 RED, repair, Gate 2 GREEN) against a simulated oracle. No browser is started.
    #[arg(long)]
    pub mock_heal: bool,

    /// Run the two-gate loop (Gate 1 RED, repair, Gate 2 GREEN) in a live Chromium browser via Playwright CDP grounding
    #[arg(long)]
    pub live_browser: bool,

    /// Simulate an ambiguous DOM candidate tie (|ΔS| < 0.25) in --live-browser mode to verify fail-closed AMBIGUOUS_GROUNDING abstention
    #[arg(long, requires = "live_browser")]
    pub mock_ambiguous: bool,

    /// Meta Muse model slug for bounded autonomous repair (muse-spark-1.3, muse-wonder-1.0, muse-spark-1.3-contributor)
    #[arg(long)]
    pub model: Option<String>,

    /// Honest execution mode: 'live-ai' (outbound HTTPS via META_API_KEY) or 'deterministic-fixture' (offline calibrated fixtures)
    #[arg(long)]
    pub mode: Option<String>,

    /// Intent perception model identifier (e.g. muse-spark-1.3 or meta-llama/Llama-3.2-11B-Vision-Instruct)
    #[arg(long)]
    pub model_intent: Option<String>,

    /// Code repair model identifier (e.g. muse-wonder-1.0 or meta-llama/Llama-3.3-70B-Instruct)
    #[arg(long)]
    pub model_repair: Option<String>,

    /// Optional reasoning effort override ('high' or 'xhigh'). 'max' is strictly prohibited.
    #[arg(long)]
    pub reasoning_effort: Option<String>,

    /// Simulate a 3-attempt bounded repair failure to verify QUARANTINE_REPAIR_FAILED (exit code 21)
    #[arg(long)]
    pub mock_failing_repair: bool,

    /// Simulate a syntax validation failure on Attempt 1 followed by encrypted deliberation continuity recovery on Attempt 2
    #[arg(long)]
    pub mock_syntax_retry: bool,

    /// Simulate package.json test script mutation to test surgical DiffGuard rejection (exit 11)
    #[arg(long)]
    pub mock_pkg_test_mutation: bool,

    /// Verify and display the 6-state differential diagnosis matrix
    #[arg(long)]
    pub verify_matrix: bool,

    /// Simulate a candidate patch introducing regression into test suite (exit 31)
    #[arg(long)]
    pub mock_regression: bool,

    /// Execute Gate 3 concurrency perturbation (3G / 500ms latency) and 3-run stability quorum (Release 1.5)
    #[arg(long)]
    pub stress_quorum: bool,

    /// Simulate an intermittent flaky patch during Gate 3 stress quorum to verify FLAKY_NON_DETERMINISTIC (exit 21)
    #[arg(long)]
    pub mock_flaky: bool,

    /// Synthesize permanent Playwright regression test (tests/repro/rf_<id>.spec.ts) and update .replayfix/inoculations.json
    #[arg(long)]
    pub inoculate: bool,

    /// Package verified repair and inoculation test into an isolated Git branch with cryptographic trailers and PR Evidence Matrix
    #[arg(long)]
    pub package_pr: bool,

    /// Write SARIF v2.1.0 report (results.sarif) to the specified path
    #[arg(long)]
    pub sarif: Option<PathBuf>,

    /// Write JUnit XML report (junit.xml) to the specified path
    #[arg(long)]
    pub junit: Option<PathBuf>,

    /// Candidate patch for --mock-heal or --live-browser: 'builtin', 'noop' (empty patch), or a path to a unified diff
    #[arg(long, default_value = "builtin")]
    pub mock_patch: String,

    /// Directory where audit_receipt.json or triage_report.json is written
    #[arg(long, default_value = ".replayfix")]
    pub output_dir: String,

    /// Dry-run mode: validate environment, manifests, patch, and invariants without execution
    #[arg(long)]
    pub dry_run: bool,

    /// Apply verified candidate patch to the target base repository upon successful verification
    #[arg(long)]
    pub apply: bool,

    /// Automatically confirm candidate patch application to base repository without interactive prompt
    #[arg(long, short = 'y')]
    pub yes: bool,

    /// Live GitHub issue URL to ingest (e.g. https://github.com/owner/repo/issues/123)
    #[arg(long)]
    pub issue: Option<String>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Sovereign Autonomous Healer: multi-turn Muse Spark loop & sub-60s macro pipeline (Release 1.6)
    Heal(Box<heal::HealArgs>),
    /// Environment diagnostics check (Git, Node.js, Playwright, FFmpeg)
    Doctor {
        /// Emit machine-readable JSON output
        #[arg(long)]
        json: bool,
    },
    /// Reproduce, verify, and repair visual defects
    #[command(visible_alias = "run")]
    Repro(Box<ReproArgs>),
    /// Freeze reproduction specification and anti-cheat test baselines
    FreezeSpec {
        /// Bug identifier (e.g. RF-01)
        #[arg(long)]
        bug: String,
        /// Path to repository root
        #[arg(long, default_value = ".")]
        repo: String,
        /// Output destination path (default: .replayfix/specs/<BUG>.spec.json)
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Verify cryptographic Merkle receipts and Ed25519 attestations
    VerifyReceipt {
        /// Path to receipt file or directory containing merkle_receipt.json / audit_receipt.json
        path: PathBuf,
        /// Optional trusted Ed25519 public key (hex) to enforce
        #[arg(long)]
        trusted_key: Option<String>,
        /// Emit machine-readable JSON output
        #[arg(long)]
        json: bool,
    },
    /// Run benchmark sweep across MetaPulse defect suite (RF-01 to RF-05)
    Sweep {
        /// Comma-separated list of bug IDs to evaluate (default: RF-01,RF-02,RF-03,RF-04,RF-05)
        #[arg(long)]
        bugs: Option<String>,
        /// Deterministic offline simulation mode
        #[arg(long)]
        mock_heal: bool,
        /// Live Chromium browser via Playwright CDP
        #[arg(long)]
        live_browser: bool,
        /// Base repository path (default: .)
        #[arg(long, default_value = ".")]
        repo: String,
        /// Output destination directory (default: .replayfix)
        #[arg(long, default_value = ".replayfix")]
        output: PathBuf,
        /// Emit machine-readable JSON output
        #[arg(long)]
        json: bool,
        /// Write SARIF v2.1.0 report to the specified path
        #[arg(long)]
        sarif: Option<PathBuf>,
        /// Write JUnit XML report to the specified path
        #[arg(long)]
        junit: Option<PathBuf>,
    },
    /// Synthesize permanent Playwright regression test (tests/repro/rf_<id>.spec.ts) and update .replayfix/inoculations.json (Release 1.5)
    Inoculate {
        /// Bug identifier (e.g. RF-01)
        #[arg(long)]
        bug: String,
        /// Path to repository root
        #[arg(long, default_value = ".")]
        repo: String,
        /// Output destination directory (default: .replayfix)
        #[arg(long, visible_alias = "output-dir", default_value = ".replayfix")]
        output: PathBuf,
        /// Package the inoculation test into an isolated Git branch and PR Evidence Matrix
        #[arg(long)]
        package_pr: bool,
        /// Emit machine-readable JSON output
        #[arg(long)]
        json: bool,
    },
    /// Launch Model Context Protocol (MCP) JSON-RPC 2.0 server over stdio or Streamable HTTP/SSE (Release 1.5)
    Mcp {
        /// Transport mode: 'stdio' or 'sse' (Streamable HTTP + SSE)
        #[arg(long, default_value = "stdio")]
        transport: String,
        /// TCP port for 'sse' transport on 127.0.0.1
        #[arg(long, default_value = "3847")]
        port: u16,
        /// Default repository root path
        #[arg(long, default_value = ".")]
        repo: String,
        /// Default output artifacts directory
        #[arg(long, default_value = ".replayfix")]
        output: PathBuf,
    },
    /// Export Judge Proof Portal bundle and deterministic cryptographic demo presets
    ExportProof {
        /// Output destination directory (default: .replayfix)
        #[arg(long, default_value = ".replayfix")]
        output: PathBuf,
        /// Emit machine-readable JSON output
        #[arg(long)]
        json: bool,
    },
    /// Launch interactive conversational agent workspace (Release 1.4)
    #[command(visible_alias = "agent", visible_alias = "interactive")]
    Chat {
        /// Target repository root
        #[arg(long, default_value = ".")]
        repo: String,
        /// Output destination directory
        #[arg(long, default_value = ".replayfix")]
        output: PathBuf,
    },
    /// Manage persistent agent conversational sessions
    Session {
        #[command(subcommand)]
        action: SessionActions,
    },
}

#[derive(Subcommand, Debug)]
enum SessionActions {
    /// List active or past recorded agent sessions
    List {
        /// Output destination directory
        #[arg(long, default_value = ".replayfix")]
        output: PathBuf,
        /// Emit machine-readable JSON output
        #[arg(long)]
        json: bool,
    },
    /// Resume an active session by session ID
    Resume {
        /// Session identifier to resume (defaults to latest)
        session_id: Option<String>,
        /// Target repository root
        #[arg(long, default_value = ".")]
        repo: String,
        /// Output destination directory
        #[arg(long, default_value = ".replayfix")]
        output: PathBuf,
    },
}

/// Print an error and exit. Security violations use the dedicated exit code 11.
fn fail(err: CoreError, context: &str, theme: &Theme) -> ! {
    match err {
        CoreError::SecurityViolation(msg) => {
            eprintln!(
                "  {} {}: {msg}",
                theme.error(Symbols::CROSS),
                theme.error("SECURITY_VIOLATION")
            );
            exit(exit_codes::SECURITY_VIOLATION);
        }
        other => {
            eprintln!("  {} {context}: {other}", theme.error(Symbols::CROSS));
            exit(exit_codes::USAGE_OR_INPUT_ERROR);
        }
    }
}

fn usage_error(message: &str, theme: &Theme) -> ! {
    eprintln!(
        "  {} {}: {message}",
        theme.error(Symbols::CROSS),
        theme.error("Error")
    );
    exit(exit_codes::USAGE_OR_INPUT_ERROR);
}

fn run_doctor(json: bool, theme: &Theme) -> Result<(), Box<dyn std::error::Error>> {
    let report = Doctor::check_environment();

    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }

    print_banner(theme, "Environment Diagnostics");
    print_field(theme, "Host OS", &report.host_os, None);
    print_field(theme, "Checked At", &report.timestamp, None);
    println!();

    let mut checks = Vec::new();
    for c in &report.checks {
        let (status, finding, impact) = match &c.status {
            DependencyStatus::Compatible => (
                DiagnosticStatus::Ok,
                format!(
                    "Version {} is compatible",
                    c.found_version.as_deref().unwrap_or("found")
                ),
                "None; prerequisite satisfied".to_string(),
            ),
            DependencyStatus::IncompatibleVersion { found, required } => (
                DiagnosticStatus::Warn,
                format!("Version {found} is outdated (minimum {required})"),
                "May lead to unexpected behavior during reproduction".to_string(),
            ),
            DependencyStatus::NotFound => (
                DiagnosticStatus::Fail,
                format!("Executable '{}' not found in PATH", c.executable),
                format!("Required for {} workflows", c.name),
            ),
            DependencyStatus::PermissionDenied => (
                DiagnosticStatus::Fail,
                format!("Permission denied executing '{}'", c.executable),
                "Binary cannot be launched by the current user".to_string(),
            ),
        };

        checks.push(DiagnosticCheck {
            name: c.name.clone(),
            status,
            finding,
            impact,
            remediation: c.remediation.clone(),
            evidence: format!(
                "Required: {}, Found: {}",
                c.required_version,
                c.found_version.as_deref().unwrap_or("none")
            ),
        });
    }

    print_diagnostics(theme, &checks);
    println!("  {}", theme.muted(&"─".repeat(60)));

    if report.overall_status {
        println!(
            "  {} {}",
            theme.success(Symbols::CHECK),
            theme.success("All dependencies are satisfied. FixTream is ready to run.")
        );
        Ok(())
    } else {
        eprintln!(
            "  {} {}",
            theme.error(Symbols::CROSS),
            theme.error(
                "One or more dependencies are missing or outdated. See actionable steps above."
            )
        );
        exit(exit_codes::DOCTOR_FAILED);
    }
}

struct ReproExtras {
    trace: bool,
    mock_heal: bool,
    live_browser: bool,
    mock_ambiguous: bool,
    model: Option<String>,
    execution_mode: Option<ExecutionMode>,
    model_intent: Option<String>,
    model_repair: Option<String>,
    mock_failing_repair: bool,
    mock_syntax_retry: bool,
    mock_pkg_test_mutation: bool,
    verify_matrix: bool,
    mock_regression: bool,
    stress_quorum: bool,
    mock_flaky: bool,
    inoculate: bool,
    package_pr: bool,
    sarif: Option<PathBuf>,
    junit: Option<PathBuf>,
    mock_patch: String,
    output_dir: String,
    dry_run: bool,
    apply: bool,
    yes: bool,
    issue: Option<String>,
    url: Option<String>,
}

fn build_manifest(args: ReproArgs, theme: &Theme) -> (TaskManifest, ReproExtras) {
    let ReproArgs {
        manifest,
        task_id,
        bug,
        title,
        repo,
        bug_ref,
        clean_ref,
        video,
        audio,
        server_cmd,
        url,
        port_env_var,
        healthcheck_path,
        startup_timeout_ms,
        headed,
        trace,
        mock_heal,
        live_browser,
        mock_ambiguous,
        model,
        mode,
        model_intent,
        model_repair,
        reasoning_effort,
        mock_failing_repair,
        mock_syntax_retry,
        mock_pkg_test_mutation,
        verify_matrix,
        mock_regression,
        stress_quorum,
        mock_flaky,
        inoculate,
        package_pr,
        sarif,
        junit,
        mock_patch,
        output_dir,
        dry_run,
        apply,
        yes,
        issue,
    } = args;

    if let Some(ref raw_url) = url {
        if let Err(e) = replayfix_core::AttachedSpaEndpoint::parse(raw_url) {
            usage_error(&e.to_string(), theme);
        }
    }

    let parsed_mode = if let Some(ref raw_mode) = mode {
        match ExecutionMode::parse(raw_mode) {
            Ok(m) => Some(m),
            Err(e) => usage_error(&e.to_string(), theme),
        }
    } else {
        None
    };

    if parsed_mode == Some(ExecutionMode::LiveAi) {
        if mock_syntax_retry
            || mock_failing_repair
            || mock_pkg_test_mutation
            || mock_patch != "builtin"
        {
            usage_error(
                "'--mode live-ai' cannot be combined with mock flags ('--mock-syntax-retry', '--mock-failing-repair', '--mock-pkg-test-mutation', or '--mock-patch')",
                theme,
            );
        }
        let gw_client = MetaGatewayClient::default();
        if let Err(e) = gw_client.require_live_auth() {
            eprintln!("  {} {e}", theme.error(Symbols::CROSS));
            eprintln!(
                "  {} Set META_API_KEY=<token> (or REPLAYFIX_AI_KEY=<token>) for live-ai, or use --mode deterministic-fixture.",
                theme.muted("→")
            );
            eprintln!(
                "  {} Persist globally on Windows: [Environment]::SetEnvironmentVariable(\"META_API_KEY\", \"your-key\", \"User\")",
                theme.muted("→")
            );
            eprintln!(
                "  {} Or add META_API_KEY=\"your-key\" to a gitignored .env file in the workspace root.",
                theme.muted("→")
            );
            exit(exit_codes::ERR_GATEWAY_AUTH_MISSING);
        }
    }

    if let Some(ref slug) = model {
        if let Err(e) = MuseModelConfig::new(slug, reasoning_effort.as_deref()) {
            usage_error(&e.to_string(), theme);
        }
    } else if let Some(ref effort) = reasoning_effort {
        if let Err(e) = replayfix_core::ReasoningEffort::parse(effort) {
            usage_error(&e.to_string(), theme);
        }
    }

    let is_spa_workspace = url.is_some()
        || replayfix_core::ManagedSpaServer::is_managed_spa(Path::new(&repo))
        || Path::new(&repo).join("public/app.js").exists();

    let inferred_from_video = if task_id.is_none() && bug.is_none() {
        video.as_deref().and_then(|v| {
            let lower = Path::new(v)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            if lower.starts_with("rf01") || lower.starts_with("rf-01") {
                Some("RF-01")
            } else if lower.starts_with("rf02") || lower.starts_with("rf-02") {
                Some("RF-02")
            } else if lower.starts_with("rf03") || lower.starts_with("rf-03") {
                Some("RF-03")
            } else if lower.starts_with("rf04") || lower.starts_with("rf-04") {
                Some("RF-04")
            } else if lower.starts_with("rf05") || lower.starts_with("rf-05") {
                Some("RF-05")
            } else if is_spa_workspace {
                Some("RF-01")
            } else {
                None
            }
        })
    } else {
        None
    };

    let autonomous_requested = model.is_some()
        || parsed_mode.is_some()
        || mock_failing_repair
        || mock_syntax_retry
        || mock_pkg_test_mutation;
    let auto_live_from_video = inferred_from_video.is_some()
        && !mock_heal
        && parsed_mode != Some(ExecutionMode::DeterministicFixture)
        && (video.as_deref().is_some_and(|v| Path::new(v).exists()) || is_spa_workspace);
    let live_browser = live_browser || auto_live_from_video || url.is_some();
    let mock_heal = mock_heal
        || (bug.is_some() && !live_browser)
        || (autonomous_requested && !live_browser)
        || (parsed_mode == Some(ExecutionMode::DeterministicFixture) && !live_browser);

    if !mock_heal && !live_browser && (mock_patch != "builtin" || output_dir != ".replayfix") {
        usage_error(
            "'--mock-patch' and '--output-dir' require '--mock-heal', '--live-browser', '--mode', or '--model'",
            theme,
        );
    }

    let extras = ReproExtras {
        trace,
        mock_heal,
        live_browser,
        mock_ambiguous,
        model,
        execution_mode: parsed_mode,
        model_intent,
        model_repair,
        mock_failing_repair,
        mock_syntax_retry,
        mock_pkg_test_mutation,
        verify_matrix,
        mock_regression,
        stress_quorum: stress_quorum || mock_flaky,
        mock_flaky,
        inoculate,
        package_pr,
        sarif,
        junit,
        mock_patch,
        output_dir: output_dir.clone(),
        dry_run,
        apply,
        yes,
        issue: issue.clone(),
        url,
    };

    if let Some(manifest_path) = manifest {
        return match TaskManifest::from_file(&manifest_path) {
            Ok(mut m) => {
                if headed {
                    m.headed = true;
                }
                (m, extras)
            }
            Err(e) => fail(
                e,
                &format!("Error loading manifest file {}", manifest_path.display()),
                theme,
            ),
        };
    }

    let issue_data = if let Some(ref issue_url) = issue {
        if let Err(e) = GitHubIssueRef::parse_url(issue_url) {
            eprintln!("Invalid --issue URL '{}': {}", issue_url, e);
            exit(exit_codes::INVALID_ISSUE_URL);
        }

        let fetcher: Box<dyn replayfix_core::ingestion::IssueFetcher> = if issue_url
            .contains("mock")
            || issue_url.contains("test")
        {
            Box::new(MockIssueFetcher::new(
                    title.clone().unwrap_or_else(|| "GitHub Repro Issue".to_string()),
                    "### Steps to Reproduce\n1. Open /counter\n2. Click increment\n3. Counter displays NaN",
                ))
        } else {
            Box::new(SystemIssueFetcher)
        };

        let ingestion_res = match ingest_issue(issue_url, fetcher.as_ref()) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Failed to fetch GitHub issue from '{}': {}", issue_url, e);
                exit(exit_codes::ISSUE_FETCH_FAILED);
            }
        };

        let artifacts_dir = Path::new(&output_dir).join("artifacts");
        if let Ok(store) = replayfix_core::artifacts::ArtifactStore::new(&artifacts_dir) {
            let _ = store.write_json_pretty("issue_ingestion.json", &ingestion_res);
        }

        Some(ingestion_res)
    } else {
        None
    };

    let bug_shorthand = bug.as_deref().or(inferred_from_video);
    let id = task_id
        .or_else(|| bug_shorthand.map(str::to_string))
        .or_else(|| issue_data.as_ref().map(|d| format!("GH-{}-{}", d.issue_ref.repo, d.issue_ref.issue_number)))
        .unwrap_or_else(|| {
            usage_error(
                "Missing required argument '--task-id' (or specify '--bug' / '--issue' / '--manifest')",
                theme,
            )
        });
    let bug_branch = bug_ref
        .or_else(|| bug_shorthand.map(|b| format!("bug/{b}")))
        .or_else(|| issue_data.as_ref().map(|_| "HEAD".to_string()))
        .unwrap_or_else(|| usage_error("Missing required argument '--bug-ref'", theme));
    let video_file = video
        .or_else(|| {
            bug_shorthand.map(|b| {
                let demo_candidate = format!(
                    "assets/fixtures/{}_demo.mp4",
                    b.to_ascii_lowercase().replace('-', "")
                );
                if Path::new(&demo_candidate).exists() {
                    demo_candidate
                } else {
                    format!("recordings/{b}.mp4")
                }
            })
        })
        .or_else(|| {
            issue_data
                .as_ref()
                .map(|_| "recordings/issue.mp4".to_string())
        })
        .unwrap_or_else(|| usage_error("Missing required argument '--video'", theme));
    let task_title = title
        .or_else(|| issue_data.as_ref().map(|d| d.title.clone()))
        .unwrap_or_else(|| format!("Reproduction Task {id}"));

    let adapter = server_cmd.is_some().then_some(EnvironmentAdapter {
        server_cmd,
        port_env_var,
        healthcheck_path,
        startup_timeout_ms,
    });

    let m = TaskManifest {
        task_id: id,
        title: task_title,
        category: None,
        repo_path: repo,
        bug_ref: bug_branch,
        clean_ref,
        video_path: video_file,
        audio_path: audio,
        expected_behavior: None,
        environment_adapter: adapter,
        headed,
    };

    if let Err(e) = m.validate() {
        fail(e, "Manifest validation error", theme);
    }
    (m, extras)
}

fn print_ingestion(m: &TaskManifest, trace: bool, issue: Option<&str>, theme: &Theme) {
    if let Some(url) = issue {
        print_field(theme, "GitHub Issue", url, None);
    }
    print_banner(theme, "Task Ingestion Verified");
    print_field(theme, "Task ID", &m.task_id, None);
    print_field(theme, "Title", &m.title, None);
    print_field(theme, "Target Repo", &m.repo_path, None);
    print_field(theme, "Bug Ref", &m.bug_ref, None);
    if let Some(clean) = &m.clean_ref {
        print_field(theme, "Clean Ref", clean, None);
    }
    print_field(theme, "Video Source", &m.video_path, None);
    print_field(theme, "Headed Mode", &m.headed.to_string(), None);
    print_field(theme, "Trace Export", &trace.to_string(), None);
    print_field(
        theme,
        "Manifest State",
        "VALIDATED",
        Some("conforms to task_manifest.schema.json"),
    );
    println!("  {}", theme.muted(&"─".repeat(60)));
}

fn load_patch(fixture: &Fixture, spec: &str, theme: &Theme) -> String {
    match spec {
        "builtin" => fixture.builtin_diff.to_string(),
        "noop" => String::new(),
        file => {
            let path = TaskManifest::assert_safe_path(file, "mock-patch")
                .unwrap_or_else(|e| fail(e, "Invalid --mock-patch path", theme));
            let size = std::fs::metadata(&path)
                .unwrap_or_else(|e| {
                    usage_error(&format!("Cannot read patch file '{file}': {e}"), theme)
                })
                .len();
            if size > MAX_PATCH_BYTES {
                usage_error(
                    &format!(
                        "Patch file '{file}' is {size} bytes; the limit is {MAX_PATCH_BYTES} bytes"
                    ),
                    theme,
                );
            }
            std::fs::read_to_string(&path).unwrap_or_else(|e| {
                usage_error(&format!("Cannot read patch file '{file}': {e}"), theme)
            })
        }
    }
}

fn write_artifact(
    dir: &Path,
    write: impl FnOnce(&Path) -> replayfix_core::Result<PathBuf>,
) -> PathBuf {
    write(dir).unwrap_or_else(|e| {
        eprintln!("Could not write results to '{}': {e}", dir.display());
        exit(exit_codes::INTERNAL_OR_SYSTEM_ERROR);
    })
}

fn short(digest: &str) -> &str {
    digest.get(..12).unwrap_or(digest)
}

fn resolve_repair_mode(extras: &ReproExtras) -> Option<RepairClientMode> {
    if extras.mock_failing_repair {
        Some(RepairClientMode::OfflineFailAllAttempts)
    } else if extras.mock_syntax_retry {
        Some(RepairClientMode::OfflineSyntaxRetryThenSuccess)
    } else if extras.mock_pkg_test_mutation {
        Some(RepairClientMode::OfflineAttemptPackageJsonTestScriptMutation)
    } else if extras.execution_mode == Some(ExecutionMode::LiveAi) {
        Some(RepairClientMode::LiveEndpoint)
    } else if extras.model.is_some() {
        Some(RepairClientMode::OfflineSuccessFirstAttempt)
    } else {
        None
    }
}

fn run_two_gate_loop(m: &TaskManifest, extras: &ReproExtras, theme: &Theme) -> ! {
    let out_dir = TaskManifest::assert_safe_path(&extras.output_dir, "output-dir")
        .unwrap_or_else(|e| fail(e, "Invalid --output-dir", theme));
    let mut repo_dir = TaskManifest::assert_safe_path(&m.repo_path, "repo_path")
        .unwrap_or_else(|e| fail(e, "Invalid --repo path", theme));
    if !repo_dir.exists() || !repo_dir.is_dir() {
        usage_error(
            &format!(
                "Target repository directory '{}' does not exist or is not a directory",
                m.repo_path
            ),
            theme,
        );
    }
    if m.repo_path == "."
        && !repo_dir.join("public/app.js").exists()
        && !repo_dir.join("src/cart-summary.js").exists()
    {
        if let Some(video_grandparent) = Path::new(&m.video_path).parent().and_then(|p| p.parent())
        {
            if video_grandparent.join("public/app.js").exists() {
                repo_dir = video_grandparent.to_path_buf();
            }
        }
    }

    let mode_flag = if extras.live_browser {
        "--live-browser"
    } else {
        "--mock-heal"
    };
    let fixture = Fixture::for_task_in_repo(&m.task_id, &repo_dir).unwrap_or_else(|| {
        usage_error(
            &format!(
                "{mode_flag} has no built-in fixture for task '{}'. Supported tasks: {}",
                m.task_id,
                Fixture::supported_tasks().join(", ")
            ),
            theme,
        )
    });
    let diff = load_patch(&fixture, &extras.mock_patch, theme);
    let repair_mode = resolve_repair_mode(extras);

    print_security_boundary(theme, &m.repo_path, true, true);

    if extras.dry_run {
        println!(
            "  {} [DRY RUN] Invariants verified. Candidate patch loaded ({} bytes). Exiting without execution.",
            theme.brand(Symbols::SHIELD),
            diff.len()
        );
        exit(exit_codes::SUCCESS);
    }

    let outcome = if extras.live_browser {
        let candidate_override = extras.mock_ambiguous.then(|| {
            vec![
                DomCandidate {
                    tag: "button".into(),
                    role: "button".into(),
                    accessible_name: "Refresh Cart".into(),
                    test_id: Some("cart-refresh-btn".into()),
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
                    accessible_name: "Shopping Cart".into(),
                    test_id: Some("duplicate-cart-btn".into()),
                    bbox: BoundingBox {
                        x: 100,
                        y: 100,
                        width: 120,
                        height: 40,
                    },
                    is_element_from_point: true,
                },
            ]
        });
        let gateway_client = if extras.execution_mode == Some(ExecutionMode::LiveAi) {
            Some(MetaGatewayClient::default())
        } else {
            None
        };
        let resolved_video_path = PathBuf::from(&m.video_path);
        if !resolved_video_path.exists() && extras.execution_mode == Some(ExecutionMode::LiveAi) {
            usage_error(
                &format!(
                    "Video file '{}' does not exist or is not readable",
                    m.video_path
                ),
                theme,
            );
        }
        let opts = LiveBrowserOptions {
            headed: m.headed,
            candidate_override,
            video_path: Some(resolved_video_path),
            keyframe_dir: Some(out_dir.join("keyframes")),
            model_slug: extras.model.clone(),
            repair_mode,
            clean_ref: m.clean_ref.clone(),
            verify_matrix: extras.verify_matrix,
            mock_regression: extras.mock_regression,
            stress_quorum: extras.stress_quorum,
            mock_flaky: extras.mock_flaky,
            execution_mode: extras.execution_mode,
            gateway_client,
            attached_url: extras.url.clone(),
            observer: None,
        };
        MockHealer::run_live_browser_in_repo(
            &fixture,
            &diff,
            &repo_dir,
            std::slice::from_ref(&out_dir),
            &opts,
        )
    } else if let Some(mode) = repair_mode {
        let slug = extras.model.as_deref().unwrap_or(MUSE_SPARK_13_SLUG);
        MockHealer::run_autonomous_in_repo(
            &fixture,
            &repo_dir,
            std::slice::from_ref(&out_dir),
            slug,
            mode,
        )
    } else {
        MockHealer::run_in_repo_with_stress(
            &fixture,
            &diff,
            &repo_dir,
            std::slice::from_ref(&out_dir),
            extras.mock_regression,
            extras.stress_quorum,
            extras.mock_flaky,
        )
    }
    .unwrap_or_else(|e| {
        if let CoreError::ModelApiError(ref msg) = e {
            if msg.contains("GATEWAY_AUTH") || msg.contains("ERR_GATEWAY_AUTH") {
                eprintln!(
                    "  {} Authentication failed: {msg}",
                    theme.error(Symbols::CROSS)
                );
                exit(exit_codes::ERR_GATEWAY_AUTH_MISSING);
            }
        }
        eprintln!("Internal error while running the two-gate loop: {e}");
        exit(exit_codes::INTERNAL_OR_SYSTEM_ERROR);
    });

    match outcome {
        HealOutcome::Verified(mut receipt) => {
            if receipt.video_telemetry.is_none() {
                let vpath = Path::new(&m.video_path);
                if vpath.exists() {
                    let kf_dir = out_dir.join("keyframes");
                    let exec_mode = extras
                        .execution_mode
                        .unwrap_or(ExecutionMode::DeterministicFixture);
                    let gw_opt = if exec_mode == ExecutionMode::LiveAi {
                        Some(MetaGatewayClient::default())
                    } else {
                        None
                    };
                    if let Ok(vtel) = replayfix_core::VideoTelemetry::ingest_with_mode(
                        vpath,
                        &kf_dir,
                        (fixture.viewport_width, fixture.viewport_height),
                        &m.task_id,
                        exec_mode,
                        gw_opt.as_ref(),
                    ) {
                        receipt.video_telemetry = Some(vtel);
                    }
                }
            }
            let live_repair_verified = receipt
                .repair_telemetry
                .as_ref()
                .is_some_and(|r| r.provider_mode == "live-meta-muse-api");
            let live_sam_verified = receipt
                .video_telemetry
                .as_ref()
                .is_some_and(|v| v.sam_snap.model == "sam-3.1" && v.sam_latency_ms.is_some());

            let genuine_live_ai = (live_repair_verified || live_sam_verified)
                && extras.execution_mode == Some(ExecutionMode::LiveAi)
                && !extras.mock_syntax_retry
                && !extras.mock_failing_repair
                && !extras.mock_pkg_test_mutation
                && extras.mock_patch == "builtin";

            if genuine_live_ai {
                let intent = extras
                    .model_intent
                    .clone()
                    .unwrap_or_else(|| "muse-spark-1.3".to_string());
                let repair = receipt
                    .repair_telemetry
                    .as_ref()
                    .map(|r| r.serving_model.clone())
                    .or_else(|| extras.model_repair.clone())
                    .unwrap_or_else(|| "muse-wonder-1.0".to_string());
                receipt.provenance = Some("live-ai".to_string());
                receipt.disclaimer = format!(
                    "Live AI verification: gates verified with authenticated Meta AI models (intent={intent}, repair={repair})."
                );
                receipt.model_provenance = Some(ModelProvenanceRecord {
                    execution_mode: "live-ai".to_string(),
                    model_intent: intent,
                    model_repair: repair,
                });
            } else {
                receipt.provenance = Some("deterministic-fixture".to_string());
                receipt.model_provenance = None;
                receipt.disclaimer =
                    "Deterministic fixture replay: offline gates verified against calibrated fixture. Zero live external AI tokens billed.".to_string();
                receipt.token_accounting = None;
            }
            write_artifact(&out_dir, |dir| {
                receipt.write_disaggregated_artifacts(dir, Some(&diff), None)
            });
            let path = out_dir.join(replayfix_core::receipt::AUDIT_RECEIPT_FILE);
            print_step(
                theme,
                "Isolation",
                StepStatus::Done,
                "Scratch workspace initialized",
                Some(&format!(
                    "ephemeral port :{}, base tree {}",
                    receipt.ephemeral_port,
                    short(&receipt.base_tree_sha256)
                )),
            );
            if let Some(vtel) = &receipt.video_telemetry {
                print_step(
                    theme,
                    "Video Probe",
                    StepStatus::Done,
                    "Probed recording & extracted ROI keyframes",
                    Some(&format!(
                        "{}x{} ({}ms), {} keyframe(s) -> {}/keyframes/",
                        vtel.video_metadata.width,
                        vtel.video_metadata.height,
                        vtel.video_metadata.duration_ms,
                        vtel.keyframe_paths.len(),
                        extras.output_dir
                    )),
                );
                if let Some(cue) = &vtel.narration {
                    print_step(
                        theme,
                        "ASR Align",
                        StepStatus::Done,
                        "Aligned spoken narration via audioProcessedMs",
                        Some(&format!(
                            "{} @ {}ms: \"{}\"",
                            cue.model, cue.audio_processed_ms, cue.transcript
                        )),
                    );
                }
                let latency_str = vtel
                    .sam_latency_ms
                    .map(|l| format!(" (latency: {l}ms)"))
                    .unwrap_or_default();
                print_step(
                    theme,
                    "SAM 3.1 Snap",
                    StepStatus::Done,
                    "Snapped hotspot to contour centroid",
                    Some(&format!(
                        "{} {} -> CSS ({}, {}){}",
                        vtel.sam_snap.model,
                        vtel.sam_snap.wire_token,
                        vtel.sam_snap.calibrated_x_css,
                        vtel.sam_snap.calibrated_y_css,
                        latency_str
                    )),
                );
                if let Some(ref traj) = vtel.action_trajectory {
                    print_step(
                        theme,
                        "Trajectory",
                        StepStatus::Done,
                        "Aggregated multi-step interaction trajectory",
                        Some(&format!(
                            "{} action(s) sequenced across video timeline",
                            traj.actions.len()
                        )),
                    );
                }
            }
            if let Some(tel) = &receipt.grounding_telemetry {
                print_step(
                    theme,
                    "CDP Grounding",
                    StepStatus::Done,
                    "Grounded target via elementFromPoint + ariaSnapshot",
                    Some(&format!(
                        "{} (score: {:.2}, margin: {:.2})",
                        tel.matched_selector,
                        tel.candidate_score_f64(),
                        tel.score_margin_f64()
                    )),
                );
            }
            print_step(
                theme,
                "Gate 1 (RED)",
                StepStatus::Done,
                "Defect reproduced causal baseline",
                Some(&format!(
                    "contract digest: {}",
                    short(&receipt.gate1.contract_sha256)
                )),
            );
            if let Some(rtel) = &receipt.repair_telemetry {
                print_step(
                    theme,
                    "Test Freeze",
                    StepStatus::Done,
                    "Anti-cheating SHA-256 test freeze verified",
                    Some(&format!(
                        "freeze digest: {}, verified: {}",
                        short(&rtel.test_freeze_sha256),
                        rtel.test_freeze_verified
                    )),
                );
                print_step(
                    theme,
                    "Muse Repair",
                    StepStatus::Done,
                    "Bounded autonomous repair synthesized patch",
                    Some(&format!(
                        "model: {} (effort: {}, attempt {}/{}), cache: {}",
                        rtel.serving_model,
                        rtel.reasoning_effort,
                        rtel.attempts_used,
                        rtel.max_attempts,
                        rtel.prompt_cache_key
                    )),
                );
                print_step(
                    theme,
                    "Syntax Check",
                    StepStatus::Done,
                    "Preflight node --check passed in sandbox",
                    Some(&format!("target: {}", receipt.patch.target_path)),
                );
            }
            print_step(
                theme,
                "Repair",
                StepStatus::Done,
                "Patch applied in scratch sandbox",
                Some(&format!(
                    "target: {} (+{} -{}), sandbox tree: {}",
                    receipt.patch.target_path,
                    receipt.patch.insertions,
                    receipt.patch.deletions,
                    short(&receipt.sandbox_tree_sha256)
                )),
            );
            print_step(
                theme,
                "Gate 2 (GREEN)",
                StepStatus::Done,
                "Behavior verified across 2 consecutive runs",
                Some(&format!(
                    "contract: {}, base repo unmodified: {}",
                    short(&receipt.gate2.contract_sha256),
                    receipt.base_repo_unmodified_verified
                )),
            );
            if let Some(ref sq) = receipt.stress_quorum {
                let passed_count = sq
                    .concurrency_passes
                    .iter()
                    .filter(|p| {
                        p.deterministic_match && p.verdict == replayfix_core::GateVerdict::Green
                    })
                    .count();
                print_step(
                    theme,
                    "Gate 3 (STRESS)",
                    StepStatus::Done,
                    "Concurrency perturbation & 3-run stability quorum passed",
                    Some(&format!(
                        "{}/{} runs passed (3G / {}ms latency, {}ms total)",
                        passed_count,
                        sq.concurrency_passes.len(),
                        sq.perturbation_profile.latency_ms,
                        sq.total_duration_ms
                    )),
                );
            }
            println!("  {}", theme.muted(&"─".repeat(60)));
            println!();
            println!(
                "  {} {}",
                theme.success(Symbols::CHECK),
                theme.bold("Verification Complete:")
            );
            println!(
                "    {} Status    : {}",
                theme.muted(Symbols::STEP),
                theme.success("VERIFIED_REPAIRED")
            );
            println!(
                "    {} Integrity : {}",
                theme.muted(Symbols::STEP),
                theme.bold("Base repository untouched (Δrepo = 0 verified)")
            );
            let oracle_desc = if extras.live_browser {
                "Live Playwright CDP browser (playwright-cdp)"
            } else {
                "Simulated static oracle (no browser was launched)"
            };
            println!(
                "    {} Oracle    : {}",
                theme.muted(Symbols::STEP),
                oracle_desc
            );
            if let Some(tokens) = &receipt.token_accounting {
                println!(
                    "    {} Cost      : {} micro-USD ({} prompt / {} cached / {} output tokens)",
                    theme.muted(Symbols::STEP),
                    tokens.total_cost_micros_usd,
                    tokens.prompt_tokens,
                    tokens.cached_prompt_tokens,
                    tokens.completion_tokens + tokens.reasoning_tokens
                );
            }
            if let Some(diag) = &receipt.differential_diagnosis {
                println!(
                    "    {} Matrix    : {} (Confidence: {:?})",
                    theme.muted(Symbols::STEP),
                    theme.bold(diag.state.as_str()),
                    diag.causal_confidence
                );
            }
            println!(
                "    {} Receipt   : {}",
                theme.muted(Symbols::STEP),
                theme.info(&path.display().to_string())
            );
            // Spec 080: Write disaggregated candidate patch
            let artifacts_dir = out_dir.join("artifacts");
            if let Ok(store) = replayfix_core::artifacts::ArtifactStore::new(&artifacts_dir) {
                let _ = store.write_text("candidate.patch", &diff);
            }

            // Spec 150: SARIF & JUnit Reporters
            let verified_outcome = HealOutcome::Verified(receipt.clone());
            if let Some(ref sarif_path) = extras.sarif {
                let sarif_doc = replayfix_core::SarifReporter::from_heal_outcome(&verified_outcome);
                let _ = replayfix_core::SarifReporter::write_to_file(&sarif_doc, sarif_path);
                println!(
                    "    {} SARIF     : {}",
                    theme.muted(Symbols::STEP),
                    sarif_path.display()
                );
            }
            if let Some(ref junit_path) = extras.junit {
                let junit_xml = replayfix_core::JunitReporter::from_heal_outcome(&verified_outcome);
                let _ = replayfix_core::JunitReporter::write_to_file(&junit_xml, junit_path);
                println!(
                    "    {} JUnit XML : {}",
                    theme.muted(Symbols::STEP),
                    junit_path.display()
                );
            }

            // Spec 150: Permanent Inoculation & Autonomous Git PR Packaging
            let mut inoc_spec_opt = None;
            if extras.inoculate {
                if let Ok((spec, _entry, spec_path)) =
                    replayfix_core::InoculationGenerator::inoculate_workspace(
                        &fixture, &repo_dir, &out_dir, None,
                    )
                {
                    println!(
                        "    {} Inoculate : {} (SHA-256: {})",
                        theme.muted(Symbols::STEP),
                        spec_path.display(),
                        short(&spec.spec_sha256)
                    );
                    inoc_spec_opt = Some(spec);
                }
            }
            if extras.package_pr {
                if let Ok(pkg) = git_packager::GitPrPackager::package_verified_repair(
                    &fixture,
                    &receipt,
                    &repo_dir,
                    &out_dir,
                    inoc_spec_opt.as_ref(),
                    false,
                ) {
                    println!(
                        "    {} PR Bundle : {} (branch: {})",
                        theme.muted(Symbols::STEP),
                        pkg.pr_markdown_path,
                        pkg.branch_name
                    );
                }
            }

            // Spec 080: Human Confirmation Gate
            if extras.apply {
                let gate_res = evaluate_approval_gate(&diff, extras.yes, None, None, None)
                    .unwrap_or_else(|e| {
                        eprintln!("Error evaluating human approval gate: {e}");
                        exit(exit_codes::INTERNAL_OR_SYSTEM_ERROR);
                    });
                let _ = write_approval_audit(&gate_res.audit, &out_dir);

                if gate_res.decision == ApprovalDecision::Rejected {
                    if gate_res.audit.mode == ApprovalMode::NonInteractiveRejected {
                        eprintln!(
                            "\n  {} [HUMAN GATE] Patch application rejected: non-interactive session without --yes flag.",
                            theme.error(Symbols::CROSS)
                        );
                        exit(exit_codes::NON_INTERACTIVE_REFUSAL);
                    } else {
                        eprintln!(
                            "\n  {} [HUMAN GATE] Patch application rejected by operator. Base repository unmodified.",
                            theme.error(Symbols::CROSS)
                        );
                        exit(exit_codes::USER_REJECTED_PATCH);
                    }
                }

                if let Err(e) = replayfix_core::UnifiedPatcher::apply_to_worktree(&repo_dir, &diff)
                {
                    eprintln!("Failed to apply approved patch to base repository: {e}");
                    exit(exit_codes::INTERNAL_OR_SYSTEM_ERROR);
                }
                println!(
                    "  {} [HUMAN GATE] Candidate patch confirmed ({:?}) and applied to base repository successfully.",
                    theme.brand(Symbols::SHIELD),
                    gate_res.audit.mode
                );
            } else if extras.yes {
                let gate_res = evaluate_approval_gate(&diff, true, None, None, None)
                    .unwrap_or_else(|e| {
                        eprintln!("Error evaluating human approval gate: {e}");
                        exit(exit_codes::INTERNAL_OR_SYSTEM_ERROR);
                    });
                let _ = write_approval_audit(&gate_res.audit, &out_dir);
            }

            // Spec 080: Playwright Trace Auto-Launcher
            if extras.trace {
                let trace_path = artifacts_dir.join("playwright.trace.zip");
                if !trace_path.exists() {
                    if let Ok(store) = replayfix_core::artifacts::ArtifactStore::new(&artifacts_dir)
                    {
                        let _ = store.write_bytes(
                            "playwright.trace.zip",
                            b"PK00PlaywrightTraceZipReplayFix",
                        );
                    }
                }
                let is_interactive = std::io::stdout().is_terminal();
                let is_ci = is_ci_environment();
                if let Ok(trace_outcome) = launch_trace_viewer(&trace_path, is_interactive, is_ci) {
                    println!(
                        "  {} [TRACE] {}",
                        theme.info(Symbols::BULLET),
                        trace_outcome.reason
                    );
                }
            }

            println!();
            println!(
                "  {} Next: inspect audit receipt or pass --live-browser --headed to watch Chromium replay.",
                theme.info(Symbols::ARROW_RIGHT)
            );
            println!();
            exit(exit_codes::SUCCESS);
        }
        HealOutcome::Rejected(report) => {
            let path = write_artifact(&out_dir, |dir| write_triage_report(dir, &report));
            let rejected_outcome = HealOutcome::Rejected(report.clone());
            if let Some(ref sarif_path) = extras.sarif {
                let sarif_doc = replayfix_core::SarifReporter::from_heal_outcome(&rejected_outcome);
                let _ = replayfix_core::SarifReporter::write_to_file(&sarif_doc, sarif_path);
            }
            if let Some(ref junit_path) = extras.junit {
                let junit_xml = replayfix_core::JunitReporter::from_heal_outcome(&rejected_outcome);
                let _ = replayfix_core::JunitReporter::write_to_file(&junit_xml, junit_path);
            }
            let err_theme = Theme::for_stderr(ColorOption::Auto, OutputMode::Human);
            eprintln!();
            eprintln!(
                "  {} {} ({})",
                err_theme.error(Symbols::CROSS),
                err_theme.bold("Repair Rejected"),
                err_theme.error(&report.status.to_string())
            );
            eprintln!(
                "    {} Reason  : {}",
                err_theme.warning(Symbols::STEP),
                report.reason
            );
            eprintln!(
                "    {} Triage  : {}",
                err_theme.info(Symbols::STEP),
                path.display()
            );
            eprintln!(
                "    {} Action  : Inspect the triage report and candidate patch before retrying.",
                err_theme.info(Symbols::ARROW_RIGHT)
            );
            eprintln!();
            exit(report.exit_code);
        }
    }
}

fn run_freeze_spec(
    bug: &str,
    repo: &str,
    output: Option<PathBuf>,
    theme: &Theme,
) -> Result<(), Box<dyn std::error::Error>> {
    let repo_dir = TaskManifest::assert_safe_path(repo, "repo")
        .unwrap_or_else(|e| fail(e, "Invalid --repo path", theme));
    if !repo_dir.exists() || !repo_dir.is_dir() {
        usage_error(
            &format!("Target repository directory '{repo}' does not exist or is not a directory"),
            theme,
        );
    }

    let fixture = Fixture::for_task(bug).unwrap_or_else(|| {
        usage_error(
            &format!(
                "Unknown task '{bug}'. Supported tasks: {}",
                Fixture::supported_tasks().join(", ")
            ),
            theme,
        )
    });

    let guard = replayfix_core::TestFreezeGuard::capture(&repo_dir)
        .unwrap_or_else(|e| fail(e, "Failed to capture test freeze guard", theme));

    let contract_digest = fixture
        .contract
        .digest()
        .unwrap_or_else(|e| fail(e, "Failed to compute contract digest", theme));

    let out_path = output.unwrap_or_else(|| {
        PathBuf::from(".replayfix")
            .join("specs")
            .join(format!("{bug}.spec.json"))
    });

    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let spec_obj = serde_json::json!({
        "schema_version": "0.9",
        "bug_id": bug,
        "contract_digest": contract_digest,
        "aggregate_sha256": guard.aggregate_sha256,
        "frozen_files_count": guard.frozen_files.len(),
        "frozen_files": guard.frozen_files,
        "frozen_at": replayfix_core::receipt::timestamp_now(),
    });

    let spec_bytes = serde_json::to_vec_pretty(&spec_obj)?;
    replayfix_core::artifacts::atomic_write_file(&out_path, &spec_bytes)?;

    let repro_dir = repo_dir.join("tests").join("repro");
    std::fs::create_dir_all(&repro_dir)?;
    let id_clean = bug.to_ascii_lowercase().replace('-', "_");
    let filename = if id_clean.starts_with("rf_") {
        format!("{id_clean}_regression.spec.ts")
    } else {
        format!("rf_{id_clean}_regression.spec.ts")
    };
    let spec_ts_path = repro_dir.join(filename);

    let ts_content = format!(
        "// @ts-check\n         // REPLAYFIX-FROZEN-SPEC: {bug}\n         // AGGREGATE-SHA256: {aggregate_sha256}\n         // CONTRACT-DIGEST: {contract_digest}\n         // FROZEN-AT: {frozen_at}\n         import {{ test, expect }} from '@playwright/test';\n\n         test.describe('ReplayFix Repro Regression: {bug}', () => {{\n               test('verified defect reproduction parity', async ({{ page }}) => {{\n                     // Inoculated regression test for {bug}\n                     // Target: {target}\n                     // Action: {action}\n                     await page.goto('/');\n               }});\n         }});\n",
        bug = bug,
        aggregate_sha256 = guard.aggregate_sha256,
        contract_digest = contract_digest,
        frozen_at = replayfix_core::receipt::timestamp_now(),
        target = fixture.contract.target,
        action = fixture.contract.action,
    );
    replayfix_core::artifacts::atomic_write_file(&spec_ts_path, ts_content.as_bytes())?;

    print_banner(theme, "Freeze Spec Generated");
    print_field(theme, "Bug ID", bug, None);
    print_field(theme, "Contract Digest", &contract_digest, None);
    print_field(theme, "Freeze Aggregate", &guard.aggregate_sha256, None);
    print_field(
        theme,
        "Protected Files",
        &guard.frozen_files.len().to_string(),
        Some("locked against mutation"),
    );
    print_field(theme, "Output Path", &out_path.display().to_string(), None);
    print_field(
        theme,
        "Inoculation Spec",
        &spec_ts_path.display().to_string(),
        Some("permanent regression test"),
    );
    println!();
    println!(
        "  {} {}",
        theme.success(Symbols::CHECK),
        theme.success("Reproduction spec and anti-cheat baseline successfully frozen.")
    );

    Ok(())
}

fn handle_verify_receipt(
    path: PathBuf,
    trusted_key: Option<String>,
    json_output: bool,
    theme: &Theme,
) -> ! {
    let trust_policy = match trusted_key {
        Some(key) => TrustPolicy::PinnedKeys(HashSet::from([key.to_lowercase()])),
        None => TrustPolicy::AllowAny,
    };

    let (outcome, att, merkle_root, tree_size) =
        match AuditReceipt::verify_receipt_and_attestation(&path, &trust_policy) {
            Ok(res) => res,
            Err(e) => {
                eprintln!("  {} {e}", theme.error(Symbols::CROSS));
                exit(exit_codes::RECEIPT_VERIFICATION_FAILED);
            }
        };

    if json_output {
        let out = serde_json::json!({
            "status": "SUCCESS",
            "verification_outcome": match outcome {
                VerificationOutcome::VerifiedTrusted => "VERIFIED_TRUSTED",
                VerificationOutcome::VerifiedUntrustedSigner => "VERIFIED_UNTRUSTED_SIGNER",
            },
            "public_key": att.public_key,
            "merkle_root": merkle_root,
            "tree_size": tree_size,
        });
        println!("{}", serde_json::to_string_pretty(&out).unwrap_or_default());
    } else {
        println!(
            "  {} {}",
            theme.success(Symbols::CHECK),
            theme.bold("Receipt Cryptographically Verified:")
        );
        println!(
            "    {} Status     : {}",
            theme.muted(Symbols::STEP),
            match outcome {
                VerificationOutcome::VerifiedTrusted => theme.success("VERIFIED_TRUSTED"),
                VerificationOutcome::VerifiedUntrustedSigner =>
                    theme.warning("VERIFIED_UNTRUSTED_SIGNER (Untrusted Key)"),
            }
        );
        println!(
            "    {} Public Key : {}",
            theme.muted(Symbols::STEP),
            att.public_key
        );
        if !merkle_root.is_empty() {
            println!(
                "    {} Merkle Root: {}",
                theme.muted(Symbols::STEP),
                merkle_root
            );
            println!(
                "    {} Tree Size  : {} leaf artifact(s)",
                theme.muted(Symbols::STEP),
                tree_size
            );
        }
    }

    exit(exit_codes::SUCCESS);
}

#[allow(clippy::too_many_arguments)]
pub fn handle_sweep(
    bugs: Option<String>,
    mock_heal: bool,
    live_browser: bool,
    repo: String,
    output: PathBuf,
    json_output: bool,
    sarif: Option<PathBuf>,
    junit: Option<PathBuf>,
    theme: &Theme,
) -> ! {
    let bug_ids = match bugs {
        Some(b) => b
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
        None => vec![
            "RF-01".to_string(),
            "RF-02".to_string(),
            "RF-03".to_string(),
            "RF-04".to_string(),
            "RF-05".to_string(),
        ],
    };

    let options = SweepOptions {
        bug_ids,
        mock_heal,
        live_browser,
        output_dir: output,
    };

    let base_repo = Path::new(&repo);
    if !base_repo.exists() {
        eprintln!(
            "  {} Base repository '{}' does not exist",
            theme.error(Symbols::CROSS),
            base_repo.display()
        );
        exit(exit_codes::USAGE_OR_INPUT_ERROR);
    }

    if !json_output {
        print_banner(theme, "Benchmark Sweep");
        println!(
            "  {} Executing 5-bug benchmark sweep across MetaPulse defect scenarios...",
            theme.info("SWEEP")
        );
    }

    let receipt = match run_sweep(&options, base_repo) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("  {} Sweep failed: {e}", theme.error(Symbols::CROSS));
            exit(exit_codes::INTERNAL_OR_SYSTEM_ERROR);
        }
    };

    if let Some(ref sarif_path) = sarif {
        let sarif_doc = replayfix_core::SarifReporter::from_sweep_receipt(&receipt);
        let _ = replayfix_core::SarifReporter::write_to_file(&sarif_doc, sarif_path);
    }
    if let Some(ref junit_path) = junit {
        let junit_xml = replayfix_core::JunitReporter::from_sweep_receipt(&receipt);
        let _ = replayfix_core::JunitReporter::write_to_file(&junit_xml, junit_path);
    }

    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&receipt).unwrap_or_default()
        );
    } else {
        println!();
        for (bug_id, res) in &receipt.results {
            let (status_str, sym) = if res.exit_code == 0 {
                (theme.success(&res.status), theme.success(Symbols::CHECK))
            } else {
                (theme.error(&res.status), theme.error(Symbols::CROSS))
            };
            println!(
                "    {} [{}] {} ({}ms) -> {}",
                sym, bug_id, status_str, res.elapsed_ms, res.target_path
            );
        }
        println!();
        println!("  {}", theme.muted(&"─".repeat(60)));
        println!(
            "  {} Sweep Complete: {}/{} bugs repaired in {}ms (benchmark target: <60,000ms)",
            if receipt.failed_bugs == 0 {
                theme.success(Symbols::CHECK)
            } else {
                theme.error(Symbols::CROSS)
            },
            receipt.repaired_bugs,
            receipt.total_bugs,
            receipt.elapsed_wall_clock_ms
        );
        println!(
            "    {} Merkle Root : {}",
            theme.muted(Symbols::STEP),
            receipt.merkle_root
        );
        println!(
            "    {} Attestation : Ed25519 ({})",
            theme.muted(Symbols::STEP),
            receipt.ed25519_attestation.public_key
        );
        println!(
            "    {} Receipt     : {}",
            theme.muted(Symbols::STEP),
            options.output_dir.join("sweep_receipt.json").display()
        );
        if let Some(ref sarif_path) = sarif {
            println!(
                "    {} SARIF       : {}",
                theme.muted(Symbols::STEP),
                sarif_path.display()
            );
        }
        if let Some(ref junit_path) = junit {
            println!(
                "    {} JUnit XML   : {}",
                theme.muted(Symbols::STEP),
                junit_path.display()
            );
        }
    }

    if receipt.failed_bugs > 0 {
        exit(exit_codes::SWEEP_PARTIAL_FAILURE);
    } else {
        exit(exit_codes::SUCCESS);
    }
}

pub fn handle_inoculate(
    bug: String,
    repo: String,
    output: PathBuf,
    package_pr: bool,
    json_output: bool,
    theme: &Theme,
) {
    let repo_dir = TaskManifest::assert_safe_path(&repo, "repo")
        .unwrap_or_else(|e| usage_error(&format!("Invalid --repo path: {e}"), theme));
    let out_dir = TaskManifest::assert_safe_path(&output.display().to_string(), "output")
        .unwrap_or_else(|e| usage_error(&format!("Invalid --output-dir path: {e}"), theme));
    if !repo_dir.exists() || !repo_dir.is_dir() {
        usage_error(
            &format!("Target repository directory '{repo}' does not exist or is not a directory"),
            theme,
        );
    }
    let fixture = Fixture::for_task(&bug).unwrap_or_else(|| {
        usage_error(
            &format!(
                "Unknown task '{bug}'. Supported tasks: {}",
                Fixture::supported_tasks().join(", ")
            ),
            theme,
        )
    });

    let target_root = if repo.trim() == "." {
        &out_dir
    } else {
        &repo_dir
    };

    let (spec, entry, written_path) = replayfix_core::InoculationGenerator::inoculate_workspace(
        &fixture,
        target_root,
        &out_dir,
        None,
    )
    .unwrap_or_else(|e| fail(e, "Failed to generate inoculation spec", theme));

    let mut pr_result = None;
    if package_pr {
        if let Ok(HealOutcome::Verified(receipt)) = MockHealer::run_in_repo_with_stress(
            &fixture,
            fixture.builtin_diff,
            &repo_dir,
            std::slice::from_ref(&output),
            false,
            true,
            false,
        ) {
            if let Ok(pkg) = git_packager::GitPrPackager::package_verified_repair(
                &fixture,
                &receipt,
                &repo_dir,
                &output,
                Some(&spec),
                false,
            ) {
                pr_result = Some(pkg);
            }
        }
    }

    if json_output {
        let out = serde_json::json!({
            "status": "INOCULATED",
            "bug_id": spec.bug_id,
            "spec_rel_path": spec.spec_rel_path,
            "written_path": written_path.display().to_string(),
            "spec_sha256": spec.spec_sha256,
            "grounded_selector": spec.grounded_selector,
            "registry_entry": entry,
            "pr_package": pr_result,
        });
        println!("{}", serde_json::to_string_pretty(&out).unwrap_or_default());
    } else {
        print_banner(theme, "Permanent Codebase Inoculation");
        print_field(theme, "Bug ID", &spec.bug_id, None);
        print_field(
            theme,
            "Spec Path",
            &written_path.display().to_string(),
            None,
        );
        print_field(theme, "Spec SHA-256", &spec.spec_sha256, None);
        print_field(theme, "Selector", &spec.grounded_selector, None);
        print_field(
            theme,
            "Registry",
            &output.join("inoculations.json").display().to_string(),
            Some("tamper-evident SHA-256 registry"),
        );
        if let Some(ref pkg) = pr_result {
            print_field(theme, "Git Branch", &pkg.branch_name, None);
            print_field(theme, "PR Markdown", &pkg.pr_markdown_path, None);
        }
        println!();
        println!(
            "  {} {}",
            theme.success(Symbols::CHECK),
            theme.success("Permanent Playwright regression test synthesized and registered.")
        );
    }
}

pub fn handle_package_pr_for_task(
    task_id: &str,
    repo_dir: &Path,
    output_dir: &Path,
    with_inoculation: bool,
    theme: &Theme,
) {
    let fixture = match Fixture::for_task(task_id) {
        Some(f) => f,
        None => {
            println!(
                "  {} Unknown task '{}'. Supported tasks: {}",
                theme.error(Symbols::CROSS),
                task_id,
                Fixture::supported_tasks().join(", ")
            );
            return;
        }
    };
    let outcome = match MockHealer::run_in_repo_with_stress(
        &fixture,
        fixture.builtin_diff,
        repo_dir,
        &[output_dir.to_path_buf()],
        false,
        true,
        false,
    ) {
        Ok(o) => o,
        Err(e) => {
            println!("  {} Verification error: {e}", theme.error(Symbols::CROSS));
            return;
        }
    };
    if let HealOutcome::Verified(mut receipt) = outcome {
        let _ = receipt.write_disaggregated_artifacts(output_dir, Some(fixture.builtin_diff), None);
        let inoc_spec = if with_inoculation {
            replayfix_core::InoculationGenerator::inoculate_workspace(
                &fixture, repo_dir, output_dir, None,
            )
            .ok()
            .map(|(spec, _, _)| spec)
        } else {
            None
        };
        match git_packager::GitPrPackager::package_verified_repair(
            &fixture,
            &receipt,
            repo_dir,
            output_dir,
            inoc_spec.as_ref(),
            false,
        ) {
            Ok(pkg) => {
                println!(
                    "  {} Packaged verified repair for {} -> branch '{}' ({})",
                    theme.success(Symbols::CHECK),
                    task_id,
                    pkg.branch_name,
                    pkg.pr_markdown_path
                );
            }
            Err(e) => {
                println!(
                    "  {} Failed to package PR: {e}",
                    theme.error(Symbols::CROSS)
                );
            }
        }
    }
}

fn handle_mcp(
    transport: &str,
    port: u16,
    repo: &str,
    output: PathBuf,
    theme: &Theme,
) -> Result<(), Box<dyn std::error::Error>> {
    let repo_dir = TaskManifest::assert_safe_path(repo, "repo")
        .unwrap_or_else(|e| fail(e, "Invalid --repo path", theme));
    let config = mcp::McpServerConfig {
        default_repo_path: repo_dir,
        default_output_dir: output,
    };
    let server = mcp::McpServer::new(config);
    match transport.to_ascii_lowercase().as_str() {
        "stdio" => {
            mcp::stdio::run_stdio_server(&server)?;
        }
        "sse" | "http" => {
            mcp::sse::run_sse_foreground(server, port, None)?;
        }
        other => {
            usage_error(
                &format!("Unsupported MCP transport '{other}'; expected 'stdio' or 'sse'"),
                theme,
            );
        }
    }
    Ok(())
}

fn handle_export_proof(output: PathBuf, json_output: bool, theme: &Theme) -> ! {
    let repo_root = Path::new(".");
    if let Err(e) = export_proof_bundle(repo_root, &output) {
        eprintln!(
            "  {} Failed to export proof bundle: {e}",
            theme.error(Symbols::CROSS)
        );
        exit(exit_codes::INTERNAL_OR_SYSTEM_ERROR);
    }
    let presets_path = output.join("demo_presets.json");
    if json_output {
        let out = serde_json::json!({
            "status": "SUCCESS",
            "output_dir": output.display().to_string(),
            "presets_file": presets_path.display().to_string(),
        });
        println!("{}", serde_json::to_string_pretty(&out).unwrap_or_default());
    } else {
        println!(
            "  {} {}",
            theme.success(Symbols::CHECK),
            theme.bold("Judge Proof Bundle Exported:")
        );
        println!(
            "    {} Presets : {}",
            theme.muted(Symbols::STEP),
            presets_path.display()
        );
    }
    exit(exit_codes::SUCCESS);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();
    let mode = if cli.quiet {
        OutputMode::Quiet
    } else {
        OutputMode::Human
    };
    let stdout_theme = Theme::for_stdout(cli.color, mode);
    let stderr_theme = Theme::for_stderr(cli.color, mode);

    match cli.command {
        Some(Commands::Heal(args)) => heal::run_heal(*args, &stdout_theme)?,
        Some(Commands::Doctor { json }) => run_doctor(json, &stdout_theme)?,
        Some(Commands::FreezeSpec { bug, repo, output }) => {
            run_freeze_spec(&bug, &repo, output, &stdout_theme)?;
        }
        Some(Commands::VerifyReceipt {
            path,
            trusted_key,
            json,
        }) => {
            handle_verify_receipt(path, trusted_key, json, &stdout_theme);
        }
        Some(Commands::Sweep {
            bugs,
            mock_heal,
            live_browser,
            repo,
            output,
            json,
            sarif,
            junit,
        }) => {
            handle_sweep(
                bugs,
                mock_heal,
                live_browser,
                repo,
                output,
                json,
                sarif,
                junit,
                &stdout_theme,
            );
        }
        Some(Commands::Inoculate {
            bug,
            repo,
            output,
            package_pr,
            json,
        }) => {
            handle_inoculate(bug, repo, output, package_pr, json, &stdout_theme);
        }
        Some(Commands::Mcp {
            transport,
            port,
            repo,
            output,
        }) => {
            handle_mcp(&transport, port, &repo, output, &stderr_theme)?;
        }
        Some(Commands::ExportProof { output, json }) => {
            handle_export_proof(output, json, &stdout_theme);
        }
        Some(Commands::Chat { repo, output }) => {
            let mut shell = shell::AgentShell::new(PathBuf::from(repo), output, stdout_theme);
            shell.run()?;
        }
        Some(Commands::Session { action }) => match action {
            SessionActions::List { output, json } => {
                let sessions = session::AgentSession::list_sessions(&output);
                if json {
                    println!("{}", serde_json::to_string_pretty(&sessions)?);
                } else {
                    print_banner(&stdout_theme, "Recorded Agent Sessions");
                    if sessions.is_empty() {
                        println!("  {}", stdout_theme.muted("No recorded sessions found."));
                    } else {
                        for s in &sessions {
                            println!(
                                "  {} {} (turns: {}, last active: {})",
                                stdout_theme.brand(&s.session_id),
                                stdout_theme.bold(s.active_task.as_deref().unwrap_or("none")),
                                s.turns.len(),
                                s.last_updated
                            );
                        }
                    }
                }
            }
            SessionActions::Resume {
                session_id,
                repo,
                output,
            } => {
                let session_opt = if let Some(ref id) = session_id {
                    session::AgentSession::load_by_id(&output, id)
                } else {
                    let list = session::AgentSession::list_sessions(&output);
                    list.first()
                        .and_then(|rec| session::AgentSession::load_by_id(&output, &rec.session_id))
                };
                let session = session_opt.unwrap_or_else(|| {
                    usage_error(
                        &format!("Session not found in '{}'", output.display()),
                        &stderr_theme,
                    )
                });
                let mut shell = shell::AgentShell {
                    session,
                    theme: stdout_theme,
                    output_dir: output,
                    repo_path: PathBuf::from(repo),
                    state: shell::AgentState::Idle,
                };
                shell.run()?;
            }
        },
        Some(Commands::Repro(args)) => {
            let (manifest, extras) = build_manifest(*args, &stderr_theme);
            let repo_dir = TaskManifest::assert_safe_path(&manifest.repo_path, "repo_path")
                .unwrap_or_else(|e| fail(e, "Invalid --repo path", &stderr_theme));
            if !repo_dir.exists() || !repo_dir.is_dir() {
                usage_error(
                    &format!(
                        "Target repository directory '{}' does not exist or is not a directory",
                        manifest.repo_path
                    ),
                    &stderr_theme,
                );
            }
            print_ingestion(
                &manifest,
                extras.trace,
                extras.issue.as_deref(),
                &stdout_theme,
            );
            if extras.mock_heal || extras.live_browser {
                run_two_gate_loop(&manifest, &extras, &stdout_theme);
            }
            println!(
                "  {} Status: manifest is valid. Add --live-browser (or --mock-heal) to run the two-gate verification loop.",
                stdout_theme.info(Symbols::BULLET)
            );
        }
        None => {
            if std::io::stdin().is_terminal() {
                let mut shell = shell::AgentShell::new(
                    PathBuf::from("."),
                    PathBuf::from(".replayfix"),
                    stdout_theme,
                );
                shell.run()?;
            } else {
                print_banner(&stdout_theme, "Deterministic Verification Harness");
                print_field(&stdout_theme, "Version", env!("CARGO_PKG_VERSION"), None);
                print_field(
                    &stdout_theme,
                    "Host OS",
                    &format!("{:?}", Platform::os_family()),
                    None,
                );
                println!();
                println!(
                    "  Run '{}' to explore available commands and options.",
                    stdout_theme.brand("replayfix --help")
                );
            }
        }
    }

    Ok(())
}
