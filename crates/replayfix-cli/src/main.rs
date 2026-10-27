pub mod ui;

use clap::{Args, Parser, Subcommand};
use replayfix_core::receipt::{write_audit_receipt, write_triage_report};
use replayfix_core::{
    CoreError, DependencyStatus, Doctor, EnvironmentAdapter, Fixture, HealOutcome, MockHealer,
    Platform, TaskManifest,
};
use ui::{print_banner, print_field, print_step, Symbols, Theme};

/// Standard CLI process exit codes aligned with verification schemas.
pub mod exit_codes {
    pub const SUCCESS: i32 = 0;
    pub const DOCTOR_FAILED: i32 = 1;
    pub const USAGE_OR_INPUT_ERROR: i32 = 2;
    pub const INTERNAL_OR_SYSTEM_ERROR: i32 = 10;
    pub const SECURITY_VIOLATION: i32 = 11;
    pub const REPAIR_REJECTED: i32 = 30;
}
use std::path::{Path, PathBuf};
use std::process::exit;

/// Largest patch file accepted by `--mock-patch`.
const MAX_PATCH_BYTES: u64 = 1024 * 1024;

#[derive(Parser, Debug)]
#[command(
    name = "fixtream",
    version,
    about = "FixTream CLI — Deterministic verification and repair harness for modern web applications"
)]
struct Cli {
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

    /// Candidate patch for --mock-heal: 'builtin', 'noop' (empty patch), or a path to a unified diff
    #[arg(long, default_value = "builtin", requires = "mock_heal")]
    pub mock_patch: String,

    /// Directory where --mock-heal writes audit_receipt.json or triage_report.json
    #[arg(long, default_value = ".replayfix", requires = "mock_heal")]
    pub output_dir: String,
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
    Repro(Box<ReproArgs>),
}

/// Print an error and exit. Security violations use the dedicated exit code 11.
fn fail(err: CoreError, context: &str) -> ! {
    let theme = Theme::for_stderr();
    match err {
        CoreError::SecurityViolation(msg) => {
            eprintln!(
                "{} {}: {msg}",
                theme.error(Symbols::CROSS),
                theme.error("SECURITY_VIOLATION")
            );
            exit(exit_codes::SECURITY_VIOLATION);
        }
        other => {
            eprintln!("{} {context}: {other}", theme.error(Symbols::CROSS));
            exit(exit_codes::USAGE_OR_INPUT_ERROR);
        }
    }
}

fn usage_error(message: &str) -> ! {
    let theme = Theme::for_stderr();
    eprintln!(
        "{} {}: {message}",
        theme.error(Symbols::CROSS),
        theme.error("Error")
    );
    exit(exit_codes::USAGE_OR_INPUT_ERROR);
}

fn run_doctor(json: bool) -> Result<(), Box<dyn std::error::Error>> {
    let report = Doctor::check_environment();

    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }

    let theme = Theme::for_stdout();
    print_banner(&theme, "Environment Diagnostics");
    print_field(&theme, "Host OS", &report.host_os, None);
    print_field(&theme, "Checked At", &report.timestamp, None);
    println!();

    let dep_hdr = format!("{}", theme.muted("Dependency"));
    let exe_hdr = format!("{}", theme.muted("Executable"));
    let found_hdr = format!("{}", theme.muted("Found Version"));
    let req_hdr = format!("{}", theme.muted("Required"));
    let status_hdr = format!("{}", theme.muted("Status"));

    if theme.color_enabled {
        println!("  {dep_hdr:<21} {exe_hdr:<21} {found_hdr:<25} {req_hdr:<25} {status_hdr}");
    } else {
        println!(
            "  {:<12} {:<12} {:<16} {:<16} Status",
            "Dependency", "Executable", "Found Version", "Required"
        );
    }
    println!("  {}", theme.muted(&"─".repeat(68)));

    for check in &report.checks {
        let (status_text, is_ok) = match &check.status {
            DependencyStatus::Compatible => (format!("{} [OK]", Symbols::CHECK), true),
            DependencyStatus::IncompatibleVersion { .. } => {
                (format!("{} [OUTDATED]", Symbols::WARN), false)
            }
            DependencyStatus::NotFound => (format!("{} [MISSING]", Symbols::CROSS), false),
            DependencyStatus::PermissionDenied => (format!("{} [DENIED]", Symbols::CROSS), false),
        };
        let styled_status = if is_ok {
            format!("{}", theme.success(&status_text))
        } else {
            format!("{}", theme.error(&status_text))
        };
        let found = check.found_version.as_deref().unwrap_or("none");
        if theme.color_enabled {
            let styled_name = format!("{}", theme.bold(&check.name));
            println!(
                "  {:<21} {:<12} {:<16} {:<16} {}",
                styled_name, check.executable, found, check.required_version, styled_status
            );
        } else {
            println!(
                "  {:<12} {:<12} {:<16} {:<16} {}",
                check.name, check.executable, found, check.required_version, styled_status
            );
        }
        if let Some(remedy) = &check.remediation {
            if check.status != DependencyStatus::Compatible {
                println!(
                    "     {} Remediation: {remedy}",
                    theme.warning(Symbols::ARROW_RIGHT)
                );
            }
        }
    }

    println!("  {}", theme.muted(&"─".repeat(68)));
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
            theme
                .error("One or more dependencies are missing or outdated. See the remedies above.")
        );
        exit(exit_codes::DOCTOR_FAILED);
    }
}

struct ReproExtras {
    trace: bool,
    mock_heal: bool,
    mock_patch: String,
    output_dir: String,
}

fn build_manifest(args: ReproArgs) -> (TaskManifest, ReproExtras) {
    let ReproArgs {
        manifest,
        task_id,
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
        mock_patch,
        output_dir,
    } = args;

    let extras = ReproExtras {
        trace,
        mock_heal,
        mock_patch,
        output_dir,
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
            ),
        };
    }

    let id = task_id.unwrap_or_else(|| {
        usage_error("Missing required argument '--task-id' (or specify '--manifest')")
    });
    let bug_branch =
        bug_ref.unwrap_or_else(|| usage_error("Missing required argument '--bug-ref'"));
    let video_file = video.unwrap_or_else(|| usage_error("Missing required argument '--video'"));
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
        fail(e, "Manifest validation error");
    }
    (m, extras)
}

fn print_ingestion(m: &TaskManifest, trace: bool) {
    let theme = Theme::for_stdout();
    print_banner(&theme, "Task Ingestion Verified");
    print_field(&theme, "Task ID", &m.task_id, None);
    print_field(&theme, "Title", &m.title, None);
    print_field(&theme, "Target Repo", &m.repo_path, None);
    print_field(&theme, "Bug Ref", &m.bug_ref, None);
    if let Some(clean) = &m.clean_ref {
        print_field(&theme, "Clean Ref", clean, None);
    }
    print_field(&theme, "Video Source", &m.video_path, None);
    print_field(&theme, "Headed Mode", &m.headed.to_string(), None);
    print_field(&theme, "Trace Export", &trace.to_string(), None);
    print_field(
        &theme,
        "Manifest State",
        "VALIDATED",
        Some("conforms to task_manifest.schema.json"),
    );
    println!("  {}", theme.muted(&"─".repeat(60)));
}

fn load_patch(fixture: &Fixture, spec: &str) -> String {
    match spec {
        "builtin" => fixture.builtin_diff.to_string(),
        "noop" => String::new(),
        file => {
            let path = TaskManifest::assert_safe_path(file, "mock-patch")
                .unwrap_or_else(|e| fail(e, "Invalid --mock-patch path"));
            let size = std::fs::metadata(&path)
                .unwrap_or_else(|e| usage_error(&format!("Cannot read patch file '{file}': {e}")))
                .len();
            if size > MAX_PATCH_BYTES {
                usage_error(&format!(
                    "Patch file '{file}' is {size} bytes; the limit is {MAX_PATCH_BYTES} bytes"
                ));
            }
            std::fs::read_to_string(&path)
                .unwrap_or_else(|e| usage_error(&format!("Cannot read patch file '{file}': {e}")))
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

fn run_mock_heal(m: &TaskManifest, mock_patch: &str, output_dir: &str) -> ! {
    let out_dir = TaskManifest::assert_safe_path(output_dir, "output-dir")
        .unwrap_or_else(|e| fail(e, "Invalid --output-dir"));
    let repo_dir = TaskManifest::assert_safe_path(&m.repo_path, "repo_path")
        .unwrap_or_else(|e| fail(e, "Invalid --repo path"));
    if !repo_dir.exists() || !repo_dir.is_dir() {
        usage_error(&format!(
            "Target repository directory '{}' does not exist or is not a directory",
            m.repo_path
        ));
    }

    let fixture = Fixture::for_task(&m.task_id).unwrap_or_else(|| {
        usage_error(&format!(
            "--mock-heal has no built-in fixture for task '{}'. Supported tasks: {}",
            m.task_id,
            Fixture::supported_tasks().join(", ")
        ))
    });
    let diff = load_patch(&fixture, mock_patch);

    let outcome =
        MockHealer::run_in_repo(&fixture, &diff, &repo_dir, std::slice::from_ref(&out_dir))
            .unwrap_or_else(|e| {
                eprintln!("Internal error while running the two-gate loop: {e}");
                exit(exit_codes::INTERNAL_OR_SYSTEM_ERROR);
            });

    let theme = Theme::for_stdout();
    match outcome {
        HealOutcome::Verified(receipt) => {
            let path = write_artifact(&out_dir, |dir| write_audit_receipt(dir, &receipt));
            print_step(
                &theme,
                "Sandbox",
                "INFO",
                Some(&format!(
                    "isolated scratch workspace (ephemeral port :{}, base tree {})",
                    receipt.ephemeral_port,
                    short(&receipt.base_tree_sha256)
                )),
            );
            print_step(
                &theme,
                "Gate 1",
                "RED",
                Some(&format!(
                    "RED   (defect reproduced; contract {})",
                    short(&receipt.gate1.contract_sha256)
                )),
            );
            print_step(
                &theme,
                "Repair",
                "DONE",
                Some(&format!(
                    "patch applied in scratch sandbox to {} (+{} -{}, sandbox tree {})",
                    receipt.patch.target_path,
                    receipt.patch.insertions,
                    receipt.patch.deletions,
                    short(&receipt.sandbox_tree_sha256)
                )),
            );
            print_step(
                &theme,
                "Gate 2",
                "GREEN",
                Some(&format!(
                    "GREEN (same contract {}, base repo unmodified: {})",
                    short(&receipt.gate2.contract_sha256),
                    receipt.base_repo_unmodified_verified
                )),
            );
            println!("  {}", theme.muted(&"─".repeat(60)));
            println!(
                "  {} Result   : {} (simulated oracle; no browser was run)",
                theme.success(Symbols::CHECK),
                theme.success("VERIFIED_REPAIRED")
            );
            println!(
                "  {} Receipt  : {}",
                theme.info(Symbols::ARROW_RIGHT),
                path.display()
            );
            exit(exit_codes::SUCCESS);
        }
        HealOutcome::Rejected(report) => {
            let path = write_artifact(&out_dir, |dir| write_triage_report(dir, &report));
            let err_theme = Theme::for_stderr();
            eprintln!(
                "  {} Result   : {} (repair rejected)",
                err_theme.error(Symbols::CROSS),
                err_theme.error(&report.status.to_string())
            );
            eprintln!(
                "  {} Reason   : {}",
                err_theme.warning(Symbols::ARROW_RIGHT),
                report.reason
            );
            eprintln!(
                "  {} Triage   : {}",
                err_theme.info(Symbols::ARROW_RIGHT),
                path.display()
            );
            exit(report.exit_code);
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    match Cli::parse().command {
        Some(Commands::Doctor { json }) => run_doctor(json)?,
        Some(Commands::Repro(args)) => {
            let (manifest, extras) = build_manifest(*args);
            let repo_dir = TaskManifest::assert_safe_path(&manifest.repo_path, "repo_path")
                .unwrap_or_else(|e| fail(e, "Invalid --repo path"));
            if !repo_dir.exists() || !repo_dir.is_dir() {
                usage_error(&format!(
                    "Target repository directory '{}' does not exist or is not a directory",
                    manifest.repo_path
                ));
            }
            print_ingestion(&manifest, extras.trace);
            if extras.mock_heal {
                run_mock_heal(&manifest, &extras.mock_patch, &extras.output_dir);
            }
            let theme = Theme::for_stdout();
            println!(
                "  {} Status: manifest is valid. Add --mock-heal to run the simulated two-gate loop (real browser gates arrive in v0.4).",
                theme.info(Symbols::BULLET)
            );
        }
        None => {
            let theme = Theme::for_stdout();
            print_banner(&theme, "Deterministic Verification Harness");
            print_field(&theme, "Version", env!("CARGO_PKG_VERSION"), None);
            print_field(
                &theme,
                "Host OS",
                &format!("{:?}", Platform::os_family()),
                None,
            );
            println!();
            println!(
                "  Run '{}' to explore available commands and options.",
                theme.brand("fixtream --help")
            );
        }
    }

    Ok(())
}
