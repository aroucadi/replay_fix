//! CLI driver for `replayfix dashboard` (Release 1.8.0).
//!
//! Live streaming async HTTP and WebSocket observability portal for CDP screencasts and Muse Spark deliberation.

use clap::Args;
use replayfix_core::dashboard::DashboardServerHandle;
use replayfix_core::error::Result;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// CLI arguments for `replayfix dashboard`.
#[derive(Args, Debug, Clone)]
pub struct DashboardArgs {
    /// TCP port to bind HTTP and WebSocket dashboard listener (default: 3000, 0 for ephemeral)
    #[arg(long, default_value_t = 3000)]
    pub port: u16,

    /// Artifacts directory containing audit receipts (default: .replayfix)
    #[arg(long, default_value = ".replayfix")]
    pub artifacts: PathBuf,

    /// Maximum duration in seconds to run before automatic termination (default: unlimited)
    #[arg(long)]
    pub duration: Option<u64>,

    /// Automatically open the live dashboard URL in the default browser
    #[arg(long)]
    pub open: bool,

    /// Emit machine-readable JSON output
    #[arg(long)]
    pub json: bool,
}

pub fn run_dashboard_cmd(args: DashboardArgs) -> Result<i32> {
    let mut handle = DashboardServerHandle::start(args.port, args.artifacts)?;
    let base_url = handle.base_url();

    if args.open && !replayfix_core::trace::is_ci_environment() {
        #[cfg(target_os = "windows")]
        {
            let _ = std::process::Command::new("cmd")
                .args(["/C", "start", "", &base_url])
                .spawn();
        }
        #[cfg(target_os = "macos")]
        {
            let _ = std::process::Command::new("open").arg(&base_url).spawn();
        }
        #[cfg(target_os = "linux")]
        {
            let _ = std::process::Command::new("xdg-open")
                .arg(&base_url)
                .spawn();
        }
    }

    if args.json {
        let json_out = serde_json::json!({
            "status": "ONLINE",
            "port": handle.port(),
            "url": base_url,
            "open_requested": args.open,
            "websocket_endpoint": format!("ws://127.0.0.1:{}/ws/screencast/live", handle.port()),
            "sse_endpoint": format!("http://127.0.0.1:{}/events/reasoning/live", handle.port()),
        });
        println!("{}", serde_json::to_string_pretty(&json_out)?);
    } else {
        println!("==> ReplayFix Live Streaming Dashboard Server Online");
        println!("    Web Portal:         {base_url}");
        println!(
            "    WebSocket Stream:   ws://127.0.0.1:{}/ws/screencast/live",
            handle.port()
        );
        println!(
            "    SSE Reasoning Feed: http://127.0.0.1:{}/events/reasoning/live",
            handle.port()
        );
        println!("    Press Ctrl+C to terminate server.");
    }

    if let Some(dur) = args.duration {
        thread::sleep(Duration::from_secs(dur));
    } else {
        // Run indefinitely until interrupted
        let running = Arc::new(AtomicBool::new(true));
        let r = Arc::clone(&running);

        let _ = ctrlc_handler(move || {
            r.store(false, Ordering::SeqCst);
        });

        while running.load(Ordering::SeqCst) {
            thread::sleep(Duration::from_millis(100));
        }
    }

    handle.shutdown();
    Ok(0)
}

fn ctrlc_handler<F>(f: F) -> Result<()>
where
    F: FnOnce() + Send + 'static,
{
    // Minimal cross-platform signal handling or placeholder
    let _ = f;
    Ok(())
}
