pub mod ui;

use clap::{Args, Parser, Subcommand};
use replayfix_core::receipt::{write_audit_receipt, write_triage_report};
use replayfix_core::{
    BoundingBox, CoreError, DependencyStatus, Doctor, DomCandidate, EnvironmentAdapter, Fixture,
    HealOutcome, LiveBrowserOptions, MockHealer, Platform, TaskManifest,
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
    pub const BRIDGE_EXECUTION_FAILED: i32 = 20;
    pub const AMBIGUOUS_GROUNDING: i32 = 22;
    pub const REPAIR_REJECTED: i32 = 30;
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

    /// Candidate patch for --mock-heal or --live-browser: 'builtin', 'noop' (empty patch), or a path to a unified diff
    #[arg(long, default_value = "builtin")]
    pub mock_patch: String,

    /// Directory where audit_receipt.json or triage_report.json is written
    #[arg(long, default_value = ".replayfix")]
    pub output_dir: String,

    /// Dry-run mode: validate environment, manifests, patch, and invariants without execution
    #[arg(long)]
    pub dry_run: bool,
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
    mock_patch: String,
    output_dir: String,
    dry_run: bool,
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
        mock_patch,
        output_dir,
        dry_run,
    } = args;

    if !mock_heal && !live_browser && (mock_patch != "builtin" || output_dir != ".replayfix") {
        usage_error(
            "'--mock-patch' and '--output-dir' require either '--mock-heal' or '--live-browser'",
            theme,
        );
    }

    let extras = ReproExtras {
        trace,
        mock_heal,
        live_browser,
        mock_ambiguous,
        mock_patch,
        output_dir,
        dry_run,
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

    let bug_shorthand = bug.as_deref();
    let id = task_id
        .or_else(|| bug_shorthand.map(str::to_string))
        .unwrap_or_else(|| {
            usage_error(
                "Missing required argument '--task-id' (or specify '--bug' / '--manifest')",
                theme,
            )
        });
    let bug_branch = bug_ref
        .or_else(|| bug_shorthand.map(|b| format!("bug/{b}")))
        .unwrap_or_else(|| usage_error("Missing required argument '--bug-ref'", theme));
    let video_file = video
        .or_else(|| bug_shorthand.map(|b| format!("recordings/{b}.mp4")))
        .unwrap_or_else(|| usage_error("Missing required argument '--video'", theme));
    let task_title = title.unwrap_or_else(|| format!("Reproduction Task {id}"));

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

fn print_ingestion(m: &TaskManifest, trace: bool, theme: &Theme) {
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
        };
        MockHealer::run_live_browser_in_repo(
            &fixture,
            &diff,
            &repo_dir,
            std::slice::from_ref(&out_dir),
            &opts,
        )
    } else {
        MockHealer::run_in_repo(&fixture, &diff, &repo_dir, std::slice::from_ref(&out_dir))
    }
    .unwrap_or_else(|e| {
        eprintln!("Internal error while running the two-gate loop: {e}");
        exit(exit_codes::INTERNAL_OR_SYSTEM_ERROR);
    });

    match outcome {
        HealOutcome::Verified(receipt) => {
            let path = write_artifact(&out_dir, |dir| write_audit_receipt(dir, &receipt));
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
                "Behavior verified on patched source",
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
            println!(
                "    {} Receipt   : {}",
                theme.muted(Symbols::STEP),
                theme.info(&path.display().to_string())
            );
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
            print_ingestion(&manifest, extras.trace, &stdout_theme);
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
