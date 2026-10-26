use clap::{Parser, Subcommand};
use replayfix_core::Platform;

#[derive(Parser, Debug)]
#[command(
    name = "replayfix",
    version = "0.1.0",
    about = "Deterministic Rust verification harness for visual bug reproduction and repair"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Environment diagnostics check (Git, Node, Playwright, FFmpeg)
    Doctor,
    /// Reproduce, verify, and repair visual defects
    Repro {
        /// Path to target bug video recording (MP4/WebM)
        #[arg(long)]
        video: Option<String>,

        /// Path to target git repository
        #[arg(long, default_value = ".")]
        repo: String,

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
        Some(Commands::Doctor) => {
            println!("ReplayFix Doctor: checking environment dependencies...");
            println!("Host OS: {:?}", Platform::os_family());
            println!("Status: Initialized (Slice 000 Genesis)");
        }
        Some(Commands::Repro {
            video,
            repo,
            headed,
            trace,
        }) => {
            println!("ReplayFix Repro initialized.");
            println!("Target repo: {}", repo);
            if let Some(v) = video {
                println!("Target video: {}", v);
            }
            println!("Headed: {}, Trace: {}", headed, trace);
        }
        None => {
            println!("ReplayFix CLI v0.1.0 — run with --help for usage details.");
        }
    }

    Ok(())
}
