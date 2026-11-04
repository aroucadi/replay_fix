//! CLI driver for `replayfix swarm` (Release 1.8.0).
//!
//! Multi-worker distributed work-stealing defect reproduction and repair cluster.

use clap::Args;
use replayfix_core::error::Result;
use replayfix_core::manifest::TaskManifest;
use replayfix_core::swarm::{SwarmConfig, SwarmCoordinator, SwarmTask};
use std::fs;
use std::path::PathBuf;

/// CLI arguments for `replayfix swarm`.
#[derive(Args, Debug, Clone)]
pub struct SwarmArgs {
    /// Number of concurrent worker threads (default: 4)
    #[arg(long, default_value_t = 4)]
    pub workers: usize,

    /// Comma-separated list of bug IDs to evaluate (default: RF-01,RF-02,RF-03,RF-04,RF-05)
    #[arg(long)]
    pub bugs: Option<String>,

    /// Explicit task manifest JSON paths or bug IDs to dispatch to the work-stealing queue
    #[arg(long, num_args = 1..)]
    pub tasks: Option<Vec<String>>,

    /// Target repository path (default: .)
    #[arg(long, default_value = ".")]
    pub repo: String,

    /// Output artifacts directory (default: .replayfix)
    #[arg(long, default_value = ".replayfix")]
    pub output: PathBuf,

    /// Optional RFC 6962 transparency log path to automatically notarize verified receipts
    #[arg(long)]
    pub notary_log: Option<PathBuf>,

    /// Deterministic offline mock mode
    #[arg(long)]
    pub mock_heal: bool,

    /// SLA timeout in seconds per swarm run (default: 120)
    #[arg(long, default_value_t = 120)]
    pub timeout: u64,

    /// Emit machine-readable JSON output
    #[arg(long)]
    pub json: bool,
}

pub fn run_swarm_cmd(args: SwarmArgs) -> Result<i32> {
    let tasks: Vec<SwarmTask> = if let Some(ref raw_tasks) = args.tasks {
        let mut parsed_tasks = Vec::with_capacity(raw_tasks.len());
        for item in raw_tasks {
            let p = PathBuf::from(item);
            if p.exists() {
                if let Ok(raw) = fs::read_to_string(&p) {
                    if let Ok(manifest) = TaskManifest::parse_str(&raw) {
                        parsed_tasks.push(SwarmTask {
                            task_id: manifest.task_id.clone(),
                            bug: Some(manifest.task_id),
                            title: Some(manifest.title),
                            manifest_path: Some(p),
                        });
                        continue;
                    }
                }
                let stem = p
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("task")
                    .to_string();
                parsed_tasks.push(SwarmTask {
                    task_id: stem,
                    bug: None,
                    title: None,
                    manifest_path: Some(p),
                });
            } else {
                for token in item.split(',').map(str::trim).filter(|s| !s.is_empty()) {
                    parsed_tasks.push(SwarmTask {
                        task_id: token.to_string(),
                        bug: Some(token.to_string()),
                        title: Some(format!("Swarm defect triage for {token}")),
                        manifest_path: None,
                    });
                }
            }
        }
        parsed_tasks
    } else {
        let bug_list: Vec<String> = if let Some(ref raw) = args.bugs {
            raw.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        } else {
            vec![
                "RF-01".to_string(),
                "RF-02".to_string(),
                "RF-03".to_string(),
                "RF-04".to_string(),
                "RF-05".to_string(),
            ]
        };

        bug_list
            .into_iter()
            .map(|bug_id| SwarmTask {
                task_id: bug_id.clone(),
                bug: Some(bug_id.clone()),
                title: Some(format!("Swarm defect triage for {bug_id}")),
                manifest_path: None,
            })
            .collect()
    };

    let config = SwarmConfig {
        num_workers: args.workers,
        repo_path: PathBuf::from(&args.repo),
        output_dir: args.output.clone(),
        mock_heal: args.mock_heal,
        mock_patch: "builtin".to_string(),
        timeout_secs: args.timeout,
    };

    let mut coordinator = SwarmCoordinator::new(config);
    if let Some(log_path) = args.notary_log {
        coordinator = coordinator.with_notary_log(log_path);
    }
    let summary = coordinator.run_swarm(tasks)?;

    if args.json {
        println!("{}", serde_json::to_string_pretty(&summary)?);
    } else {
        println!("==> ReplayFix Distributed Swarm Execution Complete");
        println!("    Workers:     {}", summary.workers);
        println!("    Total Tasks: {}", summary.total_tasks);
        println!("    Completed:   {}", summary.completed_tasks);
        println!("    Failed:      {}", summary.failed_tasks);
        println!("    Duration:    {} ms", summary.duration_ms);
        println!(
            "    Summary:     {}/swarm_summary.json",
            args.output.display()
        );

        for t in &summary.tasks {
            let status_mark = if t.status == "VERIFIED_HEALED" {
                "✓"
            } else {
                "✗"
            };
            println!(
                "    [{status_mark}] Task {:<8} Status: {:<16} Port: {:<5} Duration: {}ms",
                t.task_id, t.status, t.port, t.duration_ms
            );
        }
    }

    if summary.failed_tasks > 0 {
        Ok(41) // SWEEP_PARTIAL_FAILURE
    } else {
        Ok(0)
    }
}
