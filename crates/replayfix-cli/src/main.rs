use clap::{Parser, Subcommand};
use replayfix_core::{CoreError, DependencyStatus, Doctor, EnvironmentAdapter, Platform, TaskManifest};
use std::path::PathBuf;
use std::process::exit;

#[derive(Parser, Debug)]
#[command(
    name = "replayfix",
    version = "0.1.1",
    about = "Deterministic Rust verification harness for visual bug reproduction and repair"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
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
    Repro {
        /// Path to JSON task manifest file adhering to task_manifest.schema.json
        #[arg(long)]
        manifest: Option<PathBuf>,

        /// Target bug identifier (e.g. RF-01)
        #[arg(long)]
        task_id: Option<String>,

        /// Description or title of the defect
        #[arg(long)]
        title: Option<String>,

        /// Path to target git repository
        #[arg(long, default_value = ".")]
        repo: String,

        /// Git branch or ref containing the defect (e.g. bug/RF-01)
        #[arg(long)]
        bug_ref: Option<String>,

        /// Optional clean reference baseline branch (e.g. main)
        #[arg(long)]
        clean_ref: Option<String>,

        /// Path to target bug video recording (MP4/WebM)
        #[arg(long)]
        video: Option<String>,

        /// Optional path to audio recording
        #[arg(long)]
        audio: Option<String>,

        /// Optional command to launch application server
        #[arg(long)]
        server_cmd: Option<String>,

        /// Environment variable used by application for port (default: PORT)
        #[arg(long, default_value = "PORT")]
        port_env_var: String,

        /// Healthcheck endpoint path (default: /api/health)
        #[arg(long, default_value = "/api/health")]
        healthcheck_path: String,

        /// Timeout in milliseconds for application healthcheck readiness
        #[arg(long, default_value = "10000")]
        startup_timeout_ms: u64,

        /// Run browser replay in visible headed mode
        #[arg(long)]
        headed: bool,

        /// Export Playwright trace for visual inspection
        #[arg(long)]
        trace: bool,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Doctor { json }) => {
            let report = Doctor::check_environment();

            if json {
                let json_str = serde_json::to_string_pretty(&report)?;
                println!("{}", json_str);
            } else {
                println!("=========================================================================================");
                println!("                              ReplayFix Doctor Diagnostics                               ");
                println!("=========================================================================================");
                println!("Host OS    : {}", report.host_os);
                println!("Checked At : {}", report.timestamp);
                println!("-----------------------------------------------------------------------------------------");
                println!(
                    "{:<12} {:<12} {:<16} {:<16} {:<10}",
                    "Dependency", "Executable", "Found Version", "Required", "Status"
                );
                println!("-----------------------------------------------------------------------------------------");

                for check in &report.checks {
                    let (status_str, status_color) = match &check.status {
                        DependencyStatus::Compatible => ("[OK]", ""),
                        DependencyStatus::IncompatibleVersion { .. } => ("[OUTDATED]", ""),
                        DependencyStatus::NotFound => ("[MISSING]", ""),
                        DependencyStatus::PermissionDenied => ("[DENIED]", ""),
                    };

                    let found = check.found_version.as_deref().unwrap_or("none");
                    println!(
                        "{:<12} {:<12} {:<16} {:<16} {}{}",
                        check.name, check.executable, found, check.required_version, status_str, status_color
                    );

                    if let Some(remedy) = &check.remediation {
                        if check.status != DependencyStatus::Compatible {
                            println!("   >> Remedy: {}", remedy);
                        }
                    }
                }

                println!("-----------------------------------------------------------------------------------------");
                if report.overall_status {
                    println!("Result: ALL SYSTEM DEPENDENCIES SATISFIED (System Ready for Reproduction)");
                } else {
                    eprintln!("Result: DEPENDENCY DEFICIT DETECTED (Please remediate missing tools above)");
                    exit(1);
                }
            }
        }
        Some(Commands::Repro {
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
        }) => {
            let task_manifest = if let Some(manifest_path) = manifest {
                match TaskManifest::from_file(&manifest_path) {
                    Ok(mut m) => {
                        // CLI flags override manifest if explicitly provided
                        if headed {
                            m.headed = true;
                        }
                        m
                    }
                    Err(CoreError::SecurityViolation(msg)) => {
                        eprintln!("SECURITY_VIOLATION: {}", msg);
                        exit(11);
                    }
                    Err(e) => {
                        eprintln!("Error loading manifest file {}: {}", manifest_path.display(), e);
                        exit(2);
                    }
                }
            } else {
                let id = task_id.unwrap_or_else(|| {
                    eprintln!("Error: Missing required argument '--task-id' (or specify '--manifest')");
                    exit(2);
                });
                let bug_branch = bug_ref.unwrap_or_else(|| {
                    eprintln!("Error: Missing required argument '--bug-ref'");
                    exit(2);
                });
                let video_file = video.unwrap_or_else(|| {
                    eprintln!("Error: Missing required argument '--video'");
                    exit(2);
                });
                let task_title = title.unwrap_or_else(|| format!("Reproduction Task {}", id));

                let adapter = if server_cmd.is_some() {
                    Some(EnvironmentAdapter {
                        server_cmd,
                        port_env_var,
                        healthcheck_path,
                        startup_timeout_ms,
                    })
                } else {
                    None
                };

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
                    match e {
                        CoreError::SecurityViolation(msg) => {
                            eprintln!("SECURITY_VIOLATION: {}", msg);
                            exit(11);
                        }
                        _ => {
                            eprintln!("Manifest Validation Error: {}", e);
                            exit(2);
                        }
                    }
                }

                m
            };

            println!("=========================================================================================");
            println!("                           ReplayFix Task Ingestion Verified                             ");
            println!("=========================================================================================");
            println!("Task ID       : {}", task_manifest.task_id);
            println!("Title         : {}", task_manifest.title);
            println!("Target Repo   : {}", task_manifest.repo_path);
            println!("Bug Ref       : {}", task_manifest.bug_ref);
            if let Some(clean) = &task_manifest.clean_ref {
                println!("Clean Ref     : {}", clean);
            }
            println!("Video Source  : {}", task_manifest.video_path);
            println!("Headed Mode   : {}", task_manifest.headed);
            println!("Trace Export  : {}", trace);
            println!("Manifest State: VALIDATED (Schema compliant with task_manifest.schema.json)");
            println!("-----------------------------------------------------------------------------------------");
            println!("Status: Ready for Gate 1 Baseline Execution (Slice 010 Walking Skeleton)");
        }
        None => {
            println!("ReplayFix CLI v0.1.1 — Deterministic Visual Bug Reproduction Harness");
            println!("Host OS: {:?}", Platform::os_family());
            println!("Run 'replayfix --help' for available commands.");
        }
    }

    Ok(())
}
