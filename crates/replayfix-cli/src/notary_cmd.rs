//! CLI driver for `replayfix notary` (Release 1.8.0).
//!
//! Cryptographic RFC 6962 transparency log ledger and consistency proof verifier.

use clap::{Args, Subcommand};
use replayfix_core::crypto::KeypairManager;
use replayfix_core::error::{CoreError, Result};
use replayfix_core::notary::{TransparencyLog, TRANSPARENCY_LOG_FILE};
use replayfix_core::receipt::AuditReceipt;
use std::fs;
use std::path::PathBuf;

/// CLI arguments for `replayfix notary`.
#[derive(Args, Debug, Clone)]
pub struct NotaryArgs {
    #[command(subcommand)]
    pub action: NotaryActions,
}

#[derive(Subcommand, Debug, Clone)]
pub enum NotaryActions {
    /// Append an audit receipt to the append-only transparency log
    Append {
        /// Path to audit_receipt.json to append
        #[arg(long)]
        receipt: PathBuf,

        /// Path to transparency_log.json (default: .replayfix/transparency_log.json)
        #[arg(long)]
        log: Option<PathBuf>,

        /// Emit machine-readable JSON output
        #[arg(long)]
        json: bool,
    },
    /// Verify the cryptographic integrity and Ed25519 attestation of the transparency log
    VerifyLog {
        /// Path to transparency_log.json (default: .replayfix/transparency_log.json)
        #[arg(long)]
        log: Option<PathBuf>,

        /// Optional trusted Ed25519 public key hex to enforce
        #[arg(long)]
        trusted_key: Option<String>,

        /// Emit machine-readable JSON output
        #[arg(long)]
        json: bool,
    },
    /// Compute and verify RFC 6962 consistency proof between two log snapshot sizes
    ConsistencyProof {
        /// First (earlier) snapshot tree size
        #[arg(long)]
        first: usize,

        /// Second (later) snapshot tree size
        #[arg(long)]
        second: usize,

        /// Path to transparency_log.json (default: .replayfix/transparency_log.json)
        #[arg(long)]
        log: Option<PathBuf>,

        /// Emit machine-readable JSON output
        #[arg(long)]
        json: bool,
    },
}

pub fn run_notary_cmd(args: NotaryArgs) -> Result<i32> {
    match args.action {
        NotaryActions::Append { receipt, log, json } => {
            let log_path =
                log.unwrap_or_else(|| PathBuf::from(".replayfix").join(TRANSPARENCY_LOG_FILE));
            let key_dir = log_path
                .parent()
                .unwrap_or_else(|| std::path::Path::new(".replayfix"));
            let keypair = KeypairManager::load_or_generate(Some(key_dir))
                .unwrap_or_else(|_| KeypairManager::generate());

            let mut trans_log = TransparencyLog::load_or_new(&log_path, &keypair)?;
            let receipt_raw = fs::read_to_string(&receipt).map_err(|e| {
                CoreError::Notary(format!(
                    "Failed to read receipt file {}: {e}",
                    receipt.display()
                ))
            })?;
            let audit_receipt: AuditReceipt = serde_json::from_str(&receipt_raw)
                .map_err(|e| CoreError::Notary(format!("Failed to parse audit receipt: {e}")))?;

            let entry = trans_log.append_receipt(&audit_receipt, &keypair)?;
            trans_log.save(&log_path)?;

            if json {
                let out = serde_json::json!({
                    "status": "APPENDED",
                    "leaf_index": entry.leaf_index,
                    "task_id": entry.task_id,
                    "leaf_hash": entry.leaf_hash,
                    "tree_size": trans_log.tree_size,
                    "merkle_root": trans_log.merkle_root,
                    "log_path": log_path.to_string_lossy(),
                });
                println!("{}", serde_json::to_string_pretty(&out)?);
            } else {
                println!("==> ReplayFix Notary: Receipt Appended");
                println!("    Leaf Index:     {}", entry.leaf_index);
                println!("    Task ID:        {}", entry.task_id);
                println!("    Leaf Hash:      {}", entry.leaf_hash);
                println!("    New Tree Size:  {}", trans_log.tree_size);
                println!("    Merkle Root:    {}", trans_log.merkle_root);
                println!("    Transparency:   {}", log_path.display());
            }
            Ok(0)
        }
        NotaryActions::VerifyLog {
            log,
            trusted_key,
            json,
        } => {
            let log_path =
                log.unwrap_or_else(|| PathBuf::from(".replayfix").join(TRANSPARENCY_LOG_FILE));
            if !log_path.exists() {
                return Err(CoreError::Notary(format!(
                    "Transparency log does not exist at {}",
                    log_path.display()
                )));
            }
            let keypair = KeypairManager::generate();
            let trans_log = TransparencyLog::load_or_new(&log_path, &keypair)?;

            let valid = trans_log.verify_ledger_integrity(trusted_key.as_deref())?;

            if json {
                let out = serde_json::json!({
                    "status": if valid { "VERIFIED_VALID" } else { "INTEGRITY_VIOLATION" },
                    "valid": valid,
                    "tree_size": trans_log.tree_size,
                    "merkle_root": trans_log.merkle_root,
                    "signer_public_key": trans_log.attestation.public_key,
                });
                println!("{}", serde_json::to_string_pretty(&out)?);
            } else if valid {
                println!(
                    "==> ReplayFix Notary: Transparency Log Cryptographically Verified [VALID]"
                );
                println!("    Tree Size:      {}", trans_log.tree_size);
                println!("    Merkle Root:    {}", trans_log.merkle_root);
                println!("    Signer Public:  {}", trans_log.attestation.public_key);
            } else {
                eprintln!("==> ReplayFix Notary: Transparency Log Verification FAILED [CORRUPTED]");
            }
            Ok(if valid { 0 } else { 40 })
        }
        NotaryActions::ConsistencyProof {
            first,
            second,
            log,
            json,
        } => {
            let log_path =
                log.unwrap_or_else(|| PathBuf::from(".replayfix").join(TRANSPARENCY_LOG_FILE));
            if !log_path.exists() {
                return Err(CoreError::Notary(format!(
                    "Transparency log does not exist at {}",
                    log_path.display()
                )));
            }
            let keypair = KeypairManager::generate();
            let trans_log = TransparencyLog::load_or_new(&log_path, &keypair)?;

            let proof = trans_log.generate_consistency_proof(first, second)?;
            let root_m = trans_log.root_at_size(first)?;
            let root_n = trans_log.root_at_size(second)?;
            let first_root_hex = trans_log.root_hex_at_size(first)?;
            let second_root_hex = trans_log.root_hex_at_size(second)?;
            let verified =
                TransparencyLog::verify_consistency_proof(first, second, &root_m, &root_n, &proof)?;

            if json {
                let out = serde_json::json!({
                    "status": if verified { "PROOF_VERIFIED" } else { "PROOF_INVALID" },
                    "verified": verified,
                    "first": first,
                    "second": second,
                    "first_root": first_root_hex,
                    "second_root": second_root_hex,
                    "proof_nodes_count": proof.len(),
                    "proof": proof,
                });
                println!("{}", serde_json::to_string_pretty(&out)?);
            } else {
                println!("==> ReplayFix Notary: RFC 6962 Consistency Proof Generated");
                println!("    Snapshot Range:  Size {first} ➔ Size {second}");
                println!("    First Root:      {first_root_hex}");
                println!("    Second Root:     {second_root_hex}");
                println!("    Verified:        {verified}");
                println!("    Proof Nodes:     {}", proof.len());
                for (idx, node) in proof.iter().enumerate() {
                    println!("    [{idx}] {node}");
                }
            }
            Ok(if verified { 0 } else { 40 })
        }
    }
}
