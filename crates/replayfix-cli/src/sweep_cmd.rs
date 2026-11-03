//! CLI Subcommand: `replayfix sweep` for v1.9.0 High-Throughput Parallel Swarm Benchmark Engine.

use clap::Args;
use replayfix_core::sweep::{SweepConfig, SweepCoordinator};
use replayfix_core::sweep_ui::SweepProgressRenderer;
use replayfix_core::Result;
use std::path::PathBuf;

#[derive(Args, Debug, Clone)]
pub struct SweepCommandArgs {
    /// Path to benchmark dataset directory or JSON manifest file (e.g. benchmarks/metapulse/MANIFEST.json)
    #[arg(
        long,
        visible_alias = "manifest",
        default_value = "benchmarks/metapulse/MANIFEST.json"
    )]
    pub dataset: PathBuf,

    /// Number of concurrent swarm triage workers (default: 4)
    #[arg(long, short = 'p', default_value = "4")]
    pub parallel: usize,

    /// Execution mode: 'deterministic-fixture' (calibrated offline fixtures) or 'live-ai' (authenticated Meta API)
    #[arg(long, default_value = "deterministic-fixture")]
    pub mode: String,

    /// Base target repository path (default: .)
    #[arg(long, default_value = ".")]
    pub repo: PathBuf,

    /// Output destination directory for benchmark artifacts
    #[arg(long, default_value = ".replayfix/sweep")]
    pub output: PathBuf,

    /// Emit streaming non-interactive NDJSON progress events for headless CI/CD
    #[arg(long)]
    pub json: bool,
}

pub fn run_sweep_command(args: SweepCommandArgs) -> Result<()> {
    let renderer = SweepProgressRenderer::new(args.json);

    let config = SweepConfig {
        dataset_or_manifest: args.dataset,
        parallel_workers: args.parallel,
        mode: args.mode,
        base_repo: args.repo,
        output_dir: args.output,
        json_stream: args.json,
    };

    let coordinator = SweepCoordinator::new(config);
    let (summary, _telemetry) = coordinator.run_sweep()?;

    renderer.finish(summary.success_count, summary.total_bugs);

    if args.json {
        let summary_json = serde_json::to_string(&summary).unwrap_or_default();
        println!("{summary_json}");
    } else {
        println!("============================================================");
        println!("  REPLAYFIX v1.9.0 BENCHMARK SWEEP SUMMARY");
        println!("============================================================");
        println!("  Suite Name:           {}", summary.suite_name);
        println!("  Total Bugs Swept:     {}", summary.total_bugs);
        println!("  Success Count:        {}", summary.success_count);
        println!("  Failure Count:        {}", summary.failure_count);
        println!(
            "  Gate 1 Red Rate:      {:.1}%",
            summary.gate1_red_rate * 100.0
        );
        println!(
            "  Gate 2 Green Rate:    {:.1}%",
            summary.gate2_green_rate * 100.0
        );
        println!("  Mean Latency / Bug:   {:.2} ms", summary.mean_latency_ms);
        println!("  P95 Latency / Bug:    {:.2} ms", summary.p95_latency_ms);
        println!(
            "  Wall-Clock Duration:  {:.2} s",
            summary.wall_clock_duration_secs
        );
        println!("  Batch Merkle Root:    {}", summary.batch_merkle_root);
        println!(
            "  Ed25519 Signature:    {}...",
            &summary.ed25519_signature[..32.min(summary.ed25519_signature.len())]
        );
        println!("============================================================");
    }

    Ok(())
}
