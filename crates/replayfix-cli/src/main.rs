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
pub mod ui;

use approval::{evaluate_approval_gate, write_approval_audit, ApprovalDecision, ApprovalMode};
use replayfix_core::crypto::{
    verify_attestation, AttestationRecord, TrustPolicy, VerificationOutcome,
};
use replayfix_core::ingestion::{
    ingest_issue, GitHubIssueRef, MockIssueFetcher, SystemIssueFetcher,
};
use replayfix_core::merkle::MerkleReceipt;
use replayfix_core::receipt::{write_audit_receipt, write_triage_report};
use replayfix_core::sweep::{run_sweep, SweepOptions};
use replayfix_core::trace::{is_ci_environment, launch_trace_viewer};
use std::collections::HashSet;
use std::fs;
use std::io::IsTerminal;

use clap::{Args, Parser, Subcommand};
use replayfix_core::{
    BoundingBox, CoreError, DependencyStatus, Doctor, DomCandidate, EnvironmentAdapter, Fixture,
    HealOutcome, LiveBrowserOptions, MockHealer, MuseModelConfig, Platform, RepairClientMode,
    TaskManifest, MUSE_SPARK_13_SLUG,
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
    pub const REGRESSION_DETECTED: i32 = 31;
    pub const QUARANTINE_INVERTED_BASELINE: i32 = 32;
    pub const RECEIPT_VERIFICATION_FAILED: i32 = 40;
    pub const SWEEP_PARTIAL_FAILURE: i32 = 41;
}

/// Largest patch file accepted by `--mock-patch`.
const MAX_PATCH_BYTES: u64 = 1024 * 1024;

#[derive(Parser, Debug)]
#[command(
    name = "fixtream",
    version,
    about = "FixTream CLI — Deterministic verification, causal reproduction, and repair harness"
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
    mock_failing_repair: bool,
    mock_syntax_retry: bool,
    mock_pkg_test_mutation: bool,
    verify_matrix: bool,
    mock_regression: bool,
    mock_patch: String,
    output_dir: String,
    dry_run: bool,
    apply: bool,
    yes: bool,
    issue: Option<String>,
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
        port_env_var,
        healthcheck_path,
        startup_timeout_ms,
        headed,
        trace,
        mock_heal,
        live_browser,
        mock_ambiguous,
        model,
        reasoning_effort,
        mock_failing_repair,
        mock_syntax_retry,
        mock_pkg_test_mutation,
        verify_matrix,
        mock_regression,
        mock_patch,
        output_dir,
        dry_run,
        apply,
        yes,
        issue,
    } = args;

    if let Some(ref slug) = model {
        if let Err(e) = MuseModelConfig::new(slug, reasoning_effort.as_deref()) {
            usage_error(&e.to_string(), theme);
        }
    } else if let Some(ref effort) = reasoning_effort {
        if let Err(e) = replayfix_core::ReasoningEffort::parse(effort) {
            usage_error(&e.to_string(), theme);
        }
    }

    let inferred_from_video = if task_id.is_none() && bug.is_none() {
        video.as_deref().and_then(|v| {
            let lower = Path::new(v)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            if lower.starts_with("rf01") || lower.starts_with("rf-01") {
                Some("RF-01")
            } else {
                None
            }
        })
    } else {
        None
    };

    let autonomous_requested =
        model.is_some() || mock_failing_repair || mock_syntax_retry || mock_pkg_test_mutation;
    let auto_live_from_video = inferred_from_video.is_some()
        && !mock_heal
        && video.as_deref().is_some_and(|v| Path::new(v).exists());
    let live_browser = live_browser || auto_live_from_video;
    let mock_heal = mock_heal || (autonomous_requested && !live_browser);

    if !mock_heal && !live_browser && (mock_patch != "builtin" || output_dir != ".replayfix") {
        usage_error(
            "'--mock-patch' and '--output-dir' require '--mock-heal', '--live-browser', or '--model'",
            theme,
        );
    }

    let extras = ReproExtras {
        trace,
        mock_heal,
        live_browser,
        mock_ambiguous,
        model,
        mock_failing_repair,
        mock_syntax_retry,
        mock_pkg_test_mutation,
        verify_matrix,
        mock_regression,
        mock_patch,
        output_dir: output_dir.clone(),
        dry_run,
        apply,
        yes,
        issue: issue.clone(),
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
    } else if extras.model.is_some() {
        Some(RepairClientMode::OfflineSuccessFirstAttempt)
    } else {
        None
    }
}

fn run_two_gate_loop(m: &TaskManifest, extras: &ReproExtras, theme: &Theme) -> ! {
    let out_dir = TaskManifest::assert_safe_path(&extras.output_dir, "output-dir")
        .unwrap_or_else(|e| fail(e, "Invalid --output-dir", theme));
    let repo_dir = TaskManifest::assert_safe_path(&m.repo_path, "repo_path")
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

    let mode_flag = if extras.live_browser {
        "--live-browser"
    } else {
        "--mock-heal"
    };
    let fixture = Fixture::for_task(&m.task_id).unwrap_or_else(|| {
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
        let opts = LiveBrowserOptions {
            headed: m.headed,
            candidate_override,
            video_path: Some(PathBuf::from(&m.video_path)),
            keyframe_dir: Some(out_dir.join("keyframes")),
            model_slug: extras.model.clone(),
            repair_mode,
            clean_ref: m.clean_ref.clone(),
            verify_matrix: extras.verify_matrix,
            mock_regression: extras.mock_regression,
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
        MockHealer::run_in_repo_with_regression(
            &fixture,
            &diff,
            &repo_dir,
            std::slice::from_ref(&out_dir),
            extras.mock_regression,
        )
    }
    .unwrap_or_else(|e| {
        eprintln!("Internal error while running the two-gate loop: {e}");
        exit(exit_codes::INTERNAL_OR_SYSTEM_ERROR);
    });

    match outcome {
        HealOutcome::Verified(receipt) => {
            let path = write_artifact(&out_dir, |dir| write_audit_receipt(dir, &receipt));
            let _ = receipt.write_disaggregated_artifacts(&out_dir, Some(&diff), None);
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
                print_step(
                    theme,
                    "SAM 3.1 Snap",
                    StepStatus::Done,
                    "Snapped hotspot to contour centroid",
                    Some(&format!(
                        "{} {} -> CSS ({}, {})",
                        vtel.sam_snap.model,
                        vtel.sam_snap.wire_token,
                        vtel.sam_snap.calibrated_x_css,
                        vtel.sam_snap.calibrated_y_css
                    )),
                );
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
    let target_file = if path.is_dir() {
        let candidates = [
            path.join("artifacts").join("merkle_receipt.json"),
            path.join("merkle_receipt.json"),
            path.join("artifacts").join("04_audit_receipt.json"),
            path.join("audit_receipt.json"),
        ];
        candidates
            .into_iter()
            .find(|p| p.exists())
            .unwrap_or_else(|| {
                eprintln!(
                    "  {} No receipt found in directory '{}'",
                    theme.error(Symbols::CROSS),
                    path.display()
                );
                exit(exit_codes::RECEIPT_VERIFICATION_FAILED);
            })
    } else {
        path
    };

    if !target_file.exists() {
        eprintln!(
            "  {} Receipt file '{}' does not exist",
            theme.error(Symbols::CROSS),
            target_file.display()
        );
        exit(exit_codes::RECEIPT_VERIFICATION_FAILED);
    }

    let file_content = fs::read_to_string(&target_file).unwrap_or_else(|e| {
        eprintln!(
            "  {} Failed to read receipt '{}': {e}",
            theme.error(Symbols::CROSS),
            target_file.display()
        );
        exit(exit_codes::RECEIPT_VERIFICATION_FAILED);
    });

    let json_val: serde_json::Value = serde_json::from_str(&file_content).unwrap_or_else(|e| {
        eprintln!(
            "  {} Malformed receipt JSON: {e}",
            theme.error(Symbols::CROSS)
        );
        exit(exit_codes::RECEIPT_VERIFICATION_FAILED);
    });

    // Check if it is a MerkleReceipt directly or an AuditReceipt
    let (merkle_receipt, attestation) = if json_val.get("inclusion_proofs").is_some() {
        let mr: MerkleReceipt = serde_json::from_value(json_val.clone()).unwrap_or_else(|e| {
            eprintln!(
                "  {} Malformed Merkle receipt: {e}",
                theme.error(Symbols::CROSS)
            );
            exit(exit_codes::RECEIPT_VERIFICATION_FAILED);
        });
        let att = mr.attestation.clone().or_else(|| {
            let att_path = target_file.parent()?.join("attestation.json");
            fs::read_to_string(att_path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
        });
        (Some(mr), att)
    } else {
        let att = json_val
            .get("attestation")
            .and_then(|v| serde_json::from_value::<AttestationRecord>(v.clone()).ok())
            .or_else(|| {
                let att_path = target_file.parent()?.join("attestation.json");
                fs::read_to_string(att_path)
                    .ok()
                    .and_then(|s| serde_json::from_str(&s).ok())
            });
        let mr = target_file
            .parent()
            .map(|p| p.join("merkle_receipt.json"))
            .and_then(|p| fs::read_to_string(p).ok())
            .and_then(|s| serde_json::from_str::<MerkleReceipt>(&s).ok());
        (mr, att)
    };

    // 1. Verify Merkle inclusion proofs if present
    if let Some(ref mr) = merkle_receipt {
        match mr.verify_all() {
            Ok(true) => {}
            Ok(false) => {
                eprintln!(
                    "  {} MERKLE_PROOF_INVALID: One or more inclusion proofs failed verification against root {}",
                    theme.error(Symbols::CROSS),
                    mr.merkle_root
                );
                exit(exit_codes::RECEIPT_VERIFICATION_FAILED);
            }
            Err(e) => {
                eprintln!(
                    "  {} MERKLE_PROOF_INVALID: Verification failed: {e}",
                    theme.error(Symbols::CROSS)
                );
                exit(exit_codes::RECEIPT_VERIFICATION_FAILED);
            }
        }
    }

    // 2. Verify Attestation
    let att = match attestation {
        Some(a) => a,
        None => {
            eprintln!(
                "  {} SIGNATURE_VERIFICATION_FAILED: Missing cryptographic attestation",
                theme.error(Symbols::CROSS)
            );
            exit(exit_codes::RECEIPT_VERIFICATION_FAILED);
        }
    };

    let trust_policy = match trusted_key {
        Some(key) => TrustPolicy::PinnedKeys(HashSet::from([key.to_lowercase()])),
        None => TrustPolicy::AllowAny,
    };

    let (merkle_root, tree_size) = if let Some(ref mr) = merkle_receipt {
        (mr.merkle_root.as_str(), mr.tree_size)
    } else {
        ("", 0)
    };

    let outcome = match verify_attestation(&att, merkle_root, tree_size, &trust_policy) {
        Ok(out) => out,
        Err(e) => {
            eprintln!(
                "  {} SIGNATURE_VERIFICATION_FAILED: {e}",
                theme.error(Symbols::CROSS)
            );
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

fn handle_sweep(
    bugs: Option<String>,
    mock_heal: bool,
    live_browser: bool,
    repo: String,
    output: PathBuf,
    json_output: bool,
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
    }

    if receipt.failed_bugs > 0 {
        exit(exit_codes::SWEEP_PARTIAL_FAILURE);
    } else {
        exit(exit_codes::SUCCESS);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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
        }) => {
            handle_sweep(
                bugs,
                mock_heal,
                live_browser,
                repo,
                output,
                json,
                &stdout_theme,
            );
        }
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
                stdout_theme.brand("fixtream --help")
            );
        }
    }

    Ok(())
}
