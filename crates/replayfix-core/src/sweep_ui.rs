//! Real-time terminal progress renderer & streaming NDJSON for high-throughput sweeps.

use crate::sweep::WorkerStatus;
use std::io::{self, Write};
use std::time::Instant;

/// Real-time terminal dashboard and NDJSON stream formatter.
#[derive(Debug)]
pub struct SweepProgressRenderer {
    json_mode: bool,
    start_time: Instant,
}

impl SweepProgressRenderer {
    pub fn new(json_mode: bool) -> Self {
        Self {
            json_mode,
            start_time: Instant::now(),
        }
    }

    /// Emit worker status update.
    pub fn update(&self, status: &WorkerStatus) {
        if self.json_mode {
            let json_line = serde_json::to_string(status).unwrap_or_default();
            println!("{json_line}");
            let _ = io::stdout().flush();
        } else {
            let active = status.active_bug_id.as_deref().unwrap_or("IDLE");
            let elapsed_sec = status.elapsed_ms as f64 / 1000.0;
            print!(
                "\r  [Worker {:02}] Bug: {:<10} Stage: {:<10} ({:.1}s) - Status: {:<12}",
                status.worker_id, active, status.stage, elapsed_sec, status.status
            );
            let _ = io::stdout().flush();
        }
    }

    /// Finish render session.
    pub fn finish(&self, total_completed: usize, total_tasks: usize) {
        if !self.json_mode {
            println!(
                "\n  ✓ Sweep completed: {}/{} defects triaged & verified in {:.2}s\n",
                total_completed,
                total_tasks,
                self.start_time.elapsed().as_secs_f64()
            );
        }
    }
}
