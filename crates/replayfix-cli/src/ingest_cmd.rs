//! CLI driver for `replayfix ingest` (Release 1.8.0).
//!
//! Ingests production APM telemetry:
//! - `--rrweb <file>`: Ingest RRWeb session recording JSON.
//! - `--sentry <url-or-file>`: Ingest Sentry / APM crash dump.
//! - Synthesizes a validated reproduction manifest adhering to `TaskManifest`.

use clap::Args;
use replayfix_core::error::{CoreError, Result};
use replayfix_core::rrweb::{
    synthesize_manifest_from_telemetry, RrwebParser, RrwebSession, SentryCrashDump, SentryIngestion,
};
use std::fs;
use std::path::{Path, PathBuf};

/// CLI arguments for `replayfix ingest`.
#[derive(Args, Debug, Clone)]
pub struct IngestArgs {
    /// Path to RRWeb JSON session recording file
    #[arg(long)]
    pub rrweb: Option<PathBuf>,

    /// File path or HTTPS URL to Sentry crash event JSON
    #[arg(long)]
    pub sentry: Option<String>,

    /// Target repository path (default: .)
    #[arg(long, default_value = ".")]
    pub repo: String,

    /// Output destination path for synthesized manifest (default: .replayfix/manifests/apm_<id>.json)
    #[arg(long)]
    pub output: Option<PathBuf>,

    /// Emit machine-readable JSON output
    #[arg(long)]
    pub json: bool,
}

pub fn run_ingest(args: IngestArgs) -> Result<i32> {
    if args.rrweb.is_none() && args.sentry.is_none() {
        return Err(CoreError::Manifest(
            "At least one of --rrweb <file> or --sentry <url-or-file> must be provided".to_string(),
        ));
    }

    let session = if let Some(ref rrweb_path) = args.rrweb {
        let content = fs::read_to_string(rrweb_path).map_err(|e| {
            CoreError::SessionReplay(format!(
                "Failed to read RRWeb file {}: {e}",
                rrweb_path.display()
            ))
        })?;
        let session_id = rrweb_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("rrweb_session");
        RrwebParser::parse_json(&content, session_id)?
    } else {
        // Construct fallback single-interaction session
        RrwebSession {
            session_id: "synthetic_apm".to_string(),
            viewport_width: 1280,
            viewport_height: 720,
            base_url: Some("http://localhost:3000".to_string()),
            interactions: Vec::new(),
            mutations: Vec::new(),
            start_time: 0,
            end_time: 0,
        }
    };

    let crash: Option<SentryCrashDump> = if let Some(ref sentry_src) = args.sentry {
        let raw = if sentry_src.starts_with("http://") || sentry_src.starts_with("https://") {
            SentryIngestion::fetch_remote_crash(sentry_src)?
        } else {
            fs::read_to_string(sentry_src).map_err(|e| {
                CoreError::SessionReplay(format!(
                    "Failed to read Sentry crash file {sentry_src}: {e}"
                ))
            })?
        };
        Some(SentryIngestion::parse_json(&raw)?)
    } else {
        None
    };

    let repo_path = Path::new(&args.repo);
    let manifest = synthesize_manifest_from_telemetry(&session, crash.as_ref(), repo_path)?;

    let out_path = args.output.unwrap_or_else(|| {
        PathBuf::from(".replayfix")
            .join("manifests")
            .join(format!("{}.manifest.json", manifest.task_id))
    });

    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let serialized = serde_json::to_string_pretty(&manifest)?;
    fs::write(&out_path, serialized)?;

    if args.json {
        let json_out = serde_json::json!({
            "status": "INGESTED",
            "task_id": manifest.task_id,
            "title": manifest.title,
            "manifest_path": out_path.to_string_lossy(),
            "interactions_count": session.interactions.len(),
            "has_crash_dump": crash.is_some(),
        });
        println!("{}", serde_json::to_string_pretty(&json_out)?);
    } else {
        println!("==> ReplayFix Telemetry Ingestion Complete");
        println!("    Task ID:       {}", manifest.task_id);
        println!("    Title:         {}", manifest.title);
        println!("    Manifest:      {}", out_path.display());
        println!("    Interactions:  {}", session.interactions.len());
        println!(
            "    Sentry Crash:  {}",
            if crash.is_some() { "Linked" } else { "None" }
        );
    }

    Ok(0)
}
