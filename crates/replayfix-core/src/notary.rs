//! RFC 6962 Merkle Receipt Transparency Log & Key Notary (`crates/replayfix-core/src/notary.rs`).
//!
//! Cryptographically chains historical triage receipts into an append-only verifiable ledger
//! (`transparency_log.json`):
//! - RFC 6962 domain-separated leaf hashing (`0x00`) and node hashing (`0x01`).
//! - RFC 6962 consistency proof generation (`SUBPROOF(m, D[0:n], b)`) and verification.
//! - RFC 8032 Ed25519 aggregate attestations.

use crate::crypto::{verify_attestation, AttestationRecord, KeypairManager, TrustPolicy};
use crate::digest::{canonical_json, sha256_hex};
use crate::error::{CoreError, Result};
use crate::merkle::{hash_internal_node, largest_power_of_2_less_than, RFC6962_LEAF_PREFIX};
use crate::receipt::{timestamp_now, AuditReceipt};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs;
use std::path::Path;

pub const TRANSPARENCY_LOG_FILE: &str = "transparency_log.json";
pub const TRANSPARENCY_SCHEMA_VERSION: &str = "1.8.0";

/// Single entry recorded in the transparency log.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TransparencyLogEntry {
    pub leaf_index: usize,
    pub receipt_sha256: String,
    pub task_id: String,
    pub timestamp: String,
    pub leaf_hash: String,
}

impl TransparencyLogEntry {
    /// Calculate RFC 6962 leaf hash: SHA-256(0x00 || canonical_json(entry_without_leaf_hash))
    pub fn compute_leaf_hash(
        leaf_index: usize,
        receipt_sha256: &str,
        task_id: &str,
        timestamp: &str,
    ) -> Result<[u8; 32]> {
        let val = serde_json::json!({
            "leaf_index": leaf_index,
            "receipt_sha256": receipt_sha256,
            "task_id": task_id,
            "timestamp": timestamp,
        });
        let canon = canonical_json(&val)?;

        let mut hasher = Sha256::new();
        hasher.update([RFC6962_LEAF_PREFIX]);
        hasher.update(canon.as_bytes());
        Ok(hasher.finalize().into())
    }
}

/// Append-only cryptographic transparency ledger (`transparency_log.json`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TransparencyLog {
    pub schema_version: String,
    pub tree_size: usize,
    pub merkle_root: String,
    pub entries: Vec<TransparencyLogEntry>,
    pub attestation: AttestationRecord,
}

impl TransparencyLog {
    /// Load an existing log or initialize a new empty ledger.
    pub fn load_or_new(log_path: &Path, keypair: &KeypairManager) -> Result<Self> {
        if log_path.exists() {
            let content = fs::read_to_string(log_path)?;
            let log: Self = serde_json::from_str(&content)
                .map_err(|e| CoreError::Notary(format!("Failed to parse transparency log: {e}")))?;
            Ok(log)
        } else {
            let empty_root = sha256_hex(b"");
            let now = timestamp_now();
            let attestation = keypair.sign(TRANSPARENCY_SCHEMA_VERSION, &empty_root, 0, &now)?;

            Ok(Self {
                schema_version: TRANSPARENCY_SCHEMA_VERSION.to_string(),
                tree_size: 0,
                merkle_root: empty_root,
                entries: Vec::new(),
                attestation,
            })
        }
    }

    /// Compute Merkle Tree Hash (MTH) for subslice of leaves according to RFC 6962.
    pub fn compute_sub_tree_hash(hashes: &[[u8; 32]]) -> [u8; 32] {
        let n = hashes.len();
        if n == 0 {
            return Sha256::digest([]).into();
        }
        if n == 1 {
            return hashes[0];
        }

        let k = largest_power_of_2_less_than(n);
        let left = Self::compute_sub_tree_hash(&hashes[..k]);
        let right = Self::compute_sub_tree_hash(&hashes[k..]);
        hash_internal_node(&left, &right)
    }

    /// Compute current Merkle root from all log entries.
    pub fn compute_root(&self) -> Result<[u8; 32]> {
        self.root_at_size(self.entries.len())
    }

    /// Compute the RFC 6962 Merkle Tree Hash for the prefix of size `size` ($0 \le \text{size} \le \text{entries.len()}$).
    pub fn root_at_size(&self, size: usize) -> Result<[u8; 32]> {
        if size > self.entries.len() {
            return Err(CoreError::Notary(format!(
                "Requested tree size {size} exceeds ledger length {}",
                self.entries.len()
            )));
        }
        if size == 0 {
            return Ok(Sha256::digest([]).into());
        }

        let mut hashes = Vec::with_capacity(size);
        for entry in &self.entries[..size] {
            let hash_bytes = hex_to_32bytes(&entry.leaf_hash)?;
            hashes.push(hash_bytes);
        }

        Ok(Self::compute_sub_tree_hash(&hashes))
    }

    /// Compute the hex-encoded RFC 6962 Merkle root for the prefix of size `size`.
    pub fn root_hex_at_size(&self, size: usize) -> Result<String> {
        let root = self.root_at_size(size)?;
        Ok(bytes32_to_hex(&root))
    }

    /// Append an `AuditReceipt` to the ledger, recomputing the Merkle root and Ed25519 signature.
    pub fn append_receipt(
        &mut self,
        receipt: &AuditReceipt,
        keypair: &KeypairManager,
    ) -> Result<TransparencyLogEntry> {
        let val = serde_json::to_value(receipt)?;
        let canon = canonical_json(&val)?;
        let receipt_sha256 = sha256_hex(canon.as_bytes());

        let leaf_index = self.entries.len();
        let now = timestamp_now();
        let leaf_hash_bytes = TransparencyLogEntry::compute_leaf_hash(
            leaf_index,
            &receipt_sha256,
            &receipt.task_id,
            &now,
        )?;
        let leaf_hash = bytes32_to_hex(&leaf_hash_bytes);

        let entry = TransparencyLogEntry {
            leaf_index,
            receipt_sha256,
            task_id: receipt.task_id.clone(),
            timestamp: now.clone(),
            leaf_hash,
        };

        self.entries.push(entry.clone());
        self.tree_size = self.entries.len();

        let new_root = self.compute_root()?;
        self.merkle_root = bytes32_to_hex(&new_root);

        self.attestation = keypair.sign(
            TRANSPARENCY_SCHEMA_VERSION,
            &self.merkle_root,
            self.tree_size,
            &now,
        )?;

        Ok(entry)
    }

    /// Persist the ledger to disk atomically.
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let pretty = serde_json::to_string_pretty(self)?;
        crate::artifacts::atomic_write_file(path, pretty.as_bytes())?;
        Ok(())
    }

    /// Generate RFC 6962 consistency proof between snapshot sizes $m \le n$.
    pub fn generate_consistency_proof(&self, m: usize, n: usize) -> Result<Vec<String>> {
        if m == 0 || m > n || n > self.entries.len() {
            return Err(CoreError::Notary(format!(
                "Invalid consistency proof boundaries: m={m}, n={n}, total={}",
                self.entries.len()
            )));
        }

        if m == n {
            return Ok(Vec::new());
        }

        let mut leaf_hashes = Vec::with_capacity(n);
        for entry in &self.entries[..n] {
            leaf_hashes.push(hex_to_32bytes(&entry.leaf_hash)?);
        }

        let mut proof_nodes = Vec::new();
        subproof(m, &leaf_hashes, true, &mut proof_nodes);

        Ok(proof_nodes
            .into_iter()
            .map(|b| bytes32_to_hex(&b))
            .collect())
    }

    /// Verify an RFC 6962 consistency proof between tree sizes $m \le n$ with roots $R_m$ and $R_n$.
    pub fn verify_consistency_proof(
        m: usize,
        n: usize,
        root_m: &[u8; 32],
        root_n: &[u8; 32],
        proof_hex: &[String],
    ) -> Result<bool> {
        if m == 0 || m > n {
            return Ok(false);
        }
        if m == n {
            return Ok(proof_hex.is_empty() && root_m == root_n);
        }

        let mut proof = Vec::with_capacity(proof_hex.len());
        for h in proof_hex {
            proof.push(hex_to_32bytes(h)?);
        }

        // RFC 6962 Section 2.1.2 verification algorithm
        let mut fn_idx = m - 1;
        let mut sn_idx = n - 1;

        while (fn_idx & 1) == 1 {
            fn_idx >>= 1;
            sn_idx >>= 1;
        }

        let mut proof_iter = proof.into_iter();
        let (mut fr, mut sr) = if fn_idx == 0 {
            (*root_m, *root_m)
        } else {
            let first = proof_iter.next().ok_or_else(|| {
                CoreError::Notary("Insufficient nodes in consistency proof".to_string())
            })?;
            (first, first)
        };

        while sn_idx > 0 {
            if (fn_idx & 1) == 1 {
                let p = proof_iter.next().ok_or_else(|| {
                    CoreError::Notary("Insufficient nodes in consistency proof".to_string())
                })?;
                fr = hash_internal_node(&p, &fr);
                sr = hash_internal_node(&p, &sr);
                while fn_idx > 0 && (fn_idx & 1) == 0 {
                    fn_idx >>= 1;
                    sn_idx >>= 1;
                }
            } else if fn_idx < sn_idx {
                let p = proof_iter.next().ok_or_else(|| {
                    CoreError::Notary("Insufficient nodes in consistency proof".to_string())
                })?;
                sr = hash_internal_node(&sr, &p);
            }
            fn_idx >>= 1;
            sn_idx >>= 1;
        }

        Ok(&fr == root_m && &sr == root_n)
    }

    /// Verify full ledger integrity and Ed25519 signature.
    pub fn verify_ledger_integrity(&self, trusted_key_hex: Option<&str>) -> Result<bool> {
        if self.tree_size != self.entries.len() {
            return Ok(false);
        }

        // 1. Verify computed Merkle root
        let computed = self.compute_root()?;
        let computed_hex = bytes32_to_hex(&computed);
        if self.merkle_root != computed_hex {
            return Ok(false);
        }

        // 2. Verify sequential leaf indices and leaf hashes
        for (expected_idx, entry) in self.entries.iter().enumerate() {
            if entry.leaf_index != expected_idx {
                return Ok(false);
            }
            let leaf_hash_bytes = TransparencyLogEntry::compute_leaf_hash(
                entry.leaf_index,
                &entry.receipt_sha256,
                &entry.task_id,
                &entry.timestamp,
            )?;
            let hex = bytes32_to_hex(&leaf_hash_bytes);
            if entry.leaf_hash != hex {
                return Ok(false);
            }
        }

        // 3. Verify RFC 6962 prefix consistency across historical tree sizes
        let n = self.entries.len();
        if n >= 2 {
            for m in 1..n {
                let root_m = self.root_at_size(m)?;
                let proof = self.generate_consistency_proof(m, n)?;
                if !Self::verify_consistency_proof(m, n, &root_m, &computed, &proof)? {
                    return Ok(false);
                }
            }
        }

        // 4. Verify Ed25519 attestation
        let policy = if let Some(key) = trusted_key_hex {
            let mut set = HashSet::new();
            set.insert(key.to_string());
            TrustPolicy::PinnedKeys(set)
        } else {
            TrustPolicy::AllowAny
        };

        match verify_attestation(
            &self.attestation,
            &self.merkle_root,
            self.tree_size,
            &policy,
        ) {
            Ok(crate::crypto::VerificationOutcome::VerifiedTrusted) => Ok(true),
            Ok(crate::crypto::VerificationOutcome::VerifiedUntrustedSigner) => {
                Ok(trusted_key_hex.is_none())
            }
            Err(_) => Ok(false),
        }
    }
}

/// Recursive RFC 6962 SUBPROOF algorithm.
fn subproof(m: usize, d: &[[u8; 32]], b: bool, proof: &mut Vec<[u8; 32]>) {
    let n = d.len();
    if m == n {
        if !b {
            proof.push(TransparencyLog::compute_sub_tree_hash(d));
        }
        return;
    }

    let k = largest_power_of_2_less_than(n);
    if m <= k {
        subproof(m, &d[..k], b, proof);
        proof.push(TransparencyLog::compute_sub_tree_hash(&d[k..]));
    } else {
        subproof(m - k, &d[k..], false, proof);
        proof.push(TransparencyLog::compute_sub_tree_hash(&d[..k]));
    }
}

fn hex_to_32bytes(hex: &str) -> Result<[u8; 32]> {
    let clean = hex.trim();
    if clean.len() != 64 {
        return Err(CoreError::Notary(format!(
            "Invalid hex length {}: expected 64 chars",
            clean.len()
        )));
    }
    let mut arr = [0u8; 32];
    for i in 0..32 {
        let byte_str = &clean[i * 2..i * 2 + 2];
        arr[i] = u8::from_str_radix(byte_str, 16).map_err(|e| {
            CoreError::Notary(format!("Failed to parse hex byte '{byte_str}': {e}"))
        })?;
    }
    Ok(arr)
}

fn bytes32_to_hex(bytes: &[u8; 32]) -> String {
    let mut s = String::with_capacity(64);
    for b in bytes {
        use std::fmt::Write;
        let _ = write!(s, "{b:02x}");
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_receipt(id: &str) -> AuditReceipt {
        let gate1 = crate::receipt::GateRecord {
            observed_at: "2026-11-02T00:00:00Z".to_string(),
            verdict: crate::engine::GateVerdict::Red,
            contract_sha256: "0".repeat(64),
            events: vec![],
        };
        let patch = crate::receipt::PatchRecord {
            target_path: "src/App.tsx".to_string(),
            sha256: "0".repeat(64),
            insertions: 5,
            deletions: 1,
            files_changed: 1,
        };
        let gate2 = crate::receipt::GateRecord {
            observed_at: "2026-11-02T00:00:01Z".to_string(),
            verdict: crate::engine::GateVerdict::Green,
            contract_sha256: "0".repeat(64),
            events: vec![],
        };

        AuditReceipt {
            schema_version: "0.9".to_string(),
            status: "VERIFIED_REPAIRED".to_string(),
            task_id: id.to_string(),
            substrate: "test".to_string(),
            disclaimer: "test disclaimer".to_string(),
            provenance: None,
            model_provenance: None,
            contract_id: "RF-01".to_string(),
            contract_sha256: "0".repeat(64),
            gate1,
            patch,
            gate2,
            phase_trace: vec![crate::fsm::Phase::Init],
            base_tree_sha256: "0".repeat(64),
            sandbox_tree_sha256: "0".repeat(64),
            base_repo_unmodified_verified: true,
            ephemeral_port: 3000,
            grounding_telemetry: None,
            video_telemetry: None,
            repair_telemetry: None,
            token_accounting: None,
            differential_diagnosis: None,
            regression_parity: None,
            stress_quorum: None,
            inoculation_entry: None,
            deliberation: None,
            merkle_root: None,
            attestation: None,
            created_at: "2026-11-02T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn test_transparency_log_append_and_consistency_proof() {
        let temp =
            std::env::temp_dir().join(format!("replayfix-notary-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(&temp).unwrap();
        let log_file = temp.join(TRANSPARENCY_LOG_FILE);
        let keypair = KeypairManager::generate();

        let mut log = TransparencyLog::load_or_new(&log_file, &keypair).unwrap();
        assert_eq!(log.tree_size, 0);

        // Append 4 receipts
        for i in 1..=4 {
            let receipt = dummy_receipt(&format!("RF-0{i}"));
            log.append_receipt(&receipt, &keypair).unwrap();
        }
        assert_eq!(log.tree_size, 4);
        let root_size_4 = hex_to_32bytes(&log.merkle_root).unwrap();

        // Verify ledger integrity
        assert!(log.verify_ledger_integrity(None).unwrap());

        // Append 3 more receipts (total 7)
        for i in 5..=7 {
            let receipt = dummy_receipt(&format!("RF-0{i}"));
            log.append_receipt(&receipt, &keypair).unwrap();
        }
        assert_eq!(log.tree_size, 7);
        let root_size_7 = hex_to_32bytes(&log.merkle_root).unwrap();

        assert!(log.verify_ledger_integrity(None).unwrap());

        // Generate consistency proof from m=4 to n=7
        let proof = log.generate_consistency_proof(4, 7).unwrap();
        assert!(!proof.is_empty());

        // Verify consistency proof
        let valid =
            TransparencyLog::verify_consistency_proof(4, 7, &root_size_4, &root_size_7, &proof)
                .unwrap();
        assert!(valid);

        let _ = fs::remove_dir_all(&temp);
    }
}
