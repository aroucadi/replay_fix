//! RFC 6962-compliant Merkle tree, leaf/node domain separation, and inclusion proofs.

use crate::crypto::AttestationRecord;
use crate::digest::canonical_json;
use crate::error::{CoreError, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

/// RFC 6962 Domain Separation Prefixes
pub const RFC6962_LEAF_PREFIX: u8 = 0x00;
pub const RFC6962_NODE_PREFIX: u8 = 0x01;

/// Empty tree root hash: SHA-256 of empty byte string.
pub const EMPTY_TREE_ROOT: &str =
    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

/// Largest power of 2 strictly less than `n` ($k < n \le 2k$).
#[must_use]
pub fn largest_power_of_2_less_than(n: usize) -> usize {
    if n <= 1 {
        0
    } else {
        1 << (usize::BITS - 1 - (n - 1).leading_zeros())
    }
}

/// A structured evidence item committed to the Merkle tree.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvidenceLeaf {
    pub artifact_type: String,
    pub file_name: String,
    pub content_sha256: String,
    pub size_bytes: usize,
    pub timestamp: String,
}

impl EvidenceLeaf {
    /// Compute the canonical byte representation of this leaf.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>> {
        let json_val = serde_json::to_value(self)?;
        let json_str = canonical_json(&json_val)?;
        Ok(json_str.into_bytes())
    }

    /// Compute RFC 6962 leaf hash: SHA-256(0x00 || canonical_leaf_bytes).
    pub fn leaf_hash(&self) -> Result<[u8; 32]> {
        let bytes = self.canonical_bytes()?;
        let mut hasher = Sha256::new();
        hasher.update([RFC6962_LEAF_PREFIX]);
        hasher.update(&bytes);
        Ok(hasher.finalize().into())
    }

    /// Hex-encoded leaf hash.
    pub fn leaf_hash_hex(&self) -> Result<String> {
        let hash = self.leaf_hash()?;
        Ok(to_hex(&hash))
    }
}

/// Compute RFC 6962 internal node hash: SHA-256(0x01 || left || right).
#[must_use]
pub fn hash_internal_node(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update([RFC6962_NODE_PREFIX]);
    hasher.update(left);
    hasher.update(right);
    hasher.finalize().into()
}

/// RFC 6962 Merkle inclusion proof for a single leaf.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MerkleInclusionProof {
    pub leaf_index: usize,
    pub tree_size: usize,
    pub audit_path: Vec<String>, // Hexadecimal SHA-256 hashes
}

impl MerkleInclusionProof {
    /// Verify this inclusion proof against an expected leaf hash and Merkle root.
    pub fn verify(&self, leaf_hash: &[u8; 32], expected_root: &[u8; 32]) -> Result<bool> {
        if self.leaf_index >= self.tree_size {
            return Err(CoreError::SecurityViolation(format!(
                "Leaf index {} exceeds tree size {}",
                self.leaf_index, self.tree_size
            )));
        }

        // Special case: single-leaf tree has empty audit path
        if self.tree_size == 1 {
            if self.audit_path.is_empty() && leaf_hash == expected_root {
                return Ok(true);
            }
            return Ok(false);
        }

        let mut path_bytes = Vec::with_capacity(self.audit_path.len());
        for p in &self.audit_path {
            let b = from_hex(p).map_err(|e| {
                CoreError::SecurityViolation(format!("Invalid hex in audit path: {}", e))
            })?;
            if b.len() != 32 {
                return Err(CoreError::SecurityViolation(
                    "Audit path node must be exactly 32 bytes".to_string(),
                ));
            }
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&b);
            path_bytes.push(arr);
        }

        // Traverse using RFC 6962 audit path verification algorithm
        let mut directions = Vec::new(); // true = sibling is right child, false = sibling is left child
        let mut index = self.leaf_index;
        let mut size = self.tree_size;

        while size > 1 {
            let k = largest_power_of_2_less_than(size);
            if index < k {
                directions.push(true);
                size = k;
            } else {
                directions.push(false);
                index -= k;
                size -= k;
            }
        }

        if directions.len() != path_bytes.len() {
            return Ok(false); // Path length mismatch
        }

        // Directions were gathered top-down; reverse to evaluate bottom-up matching path_bytes
        directions.reverse();

        let mut current_hash = *leaf_hash;
        for (sibling, is_right_sibling) in path_bytes.into_iter().zip(directions) {
            if is_right_sibling {
                current_hash = hash_internal_node(&current_hash, &sibling);
            } else {
                current_hash = hash_internal_node(&sibling, &current_hash);
            }
        }

        Ok(&current_hash == expected_root)
    }
}

/// An RFC 6962 Merkle Tree over evidence leaves.
#[derive(Debug, Clone)]
pub struct MerkleTree {
    leaves: Vec<EvidenceLeaf>,
    leaf_hashes: Vec<[u8; 32]>,
}

impl MerkleTree {
    /// Create a new Merkle tree from a collection of evidence leaves.
    pub fn new(leaves: Vec<EvidenceLeaf>) -> Result<Self> {
        let mut leaf_hashes = Vec::with_capacity(leaves.len());
        for leaf in &leaves {
            leaf_hashes.push(leaf.leaf_hash()?);
        }
        Ok(Self {
            leaves,
            leaf_hashes,
        })
    }

    /// Total number of leaves in the tree.
    #[must_use]
    pub fn len(&self) -> usize {
        self.leaves.len()
    }

    /// Whether the tree has no leaves.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.leaves.is_empty()
    }

    /// All evidence leaves in the tree.
    #[must_use]
    pub fn leaves(&self) -> &[EvidenceLeaf] {
        &self.leaves
    }

    /// Compute the Merkle tree root hash using RFC 6962 binary split.
    #[must_use]
    pub fn root_hash(&self) -> [u8; 32] {
        if self.leaf_hashes.is_empty() {
            let mut arr = [0u8; 32];
            if let Ok(b) = from_hex(EMPTY_TREE_ROOT) {
                arr.copy_from_slice(&b);
            }
            return arr;
        }
        self.sub_tree_hash(0, self.leaf_hashes.len())
    }

    /// Hexadecimal Merkle tree root hash.
    #[must_use]
    pub fn root_hex(&self) -> String {
        to_hex(&self.root_hash())
    }

    fn sub_tree_hash(&self, start: usize, len: usize) -> [u8; 32] {
        if len == 0 {
            return [0u8; 32];
        }
        if len == 1 {
            return self.leaf_hashes[start];
        }

        let k = largest_power_of_2_less_than(len);
        let left = self.sub_tree_hash(start, k);
        let right = self.sub_tree_hash(start + k, len - k);
        hash_internal_node(&left, &right)
    }

    /// Generate an inclusion proof for the leaf at `leaf_index`.
    pub fn inclusion_proof(&self, leaf_index: usize) -> Result<MerkleInclusionProof> {
        if leaf_index >= self.leaves.len() {
            return Err(CoreError::SecurityViolation(format!(
                "Leaf index {} out of bounds for tree size {}",
                leaf_index,
                self.leaves.len()
            )));
        }

        let mut audit_path = Vec::new();
        self.build_audit_path(leaf_index, 0, self.leaves.len(), &mut audit_path);

        Ok(MerkleInclusionProof {
            leaf_index,
            tree_size: self.leaves.len(),
            audit_path: audit_path.into_iter().map(|h| to_hex(&h)).collect(),
        })
    }

    fn build_audit_path(&self, m: usize, start: usize, n: usize, path: &mut Vec<[u8; 32]>) {
        if n <= 1 {
            return;
        }

        let k = largest_power_of_2_less_than(n);
        if m < k {
            // Leaf is in the left subtree; recurse first so bottom siblings come first in path
            self.build_audit_path(m, start, k, path);
            let right_hash = self.sub_tree_hash(start + k, n - k);
            path.push(right_hash);
        } else {
            // Leaf is in the right subtree; recurse first so bottom siblings come first in path
            self.build_audit_path(m - k, start + k, n - k, path);
            let left_hash = self.sub_tree_hash(start, k);
            path.push(left_hash);
        }
    }

    /// Generate complete MerkleReceipt containing all leaves and their inclusion proofs.
    pub fn to_receipt(&self) -> Result<MerkleReceipt> {
        let root = self.root_hex();
        let mut inclusion_proofs = HashMap::new();

        for (idx, leaf) in self.leaves.iter().enumerate() {
            let proof = self.inclusion_proof(idx)?;
            inclusion_proofs.insert(leaf.file_name.clone(), proof);
        }

        Ok(MerkleReceipt {
            schema_version: "0.9".to_string(),
            tree_size: self.leaves.len(),
            merkle_root: root,
            leaves: self.leaves.clone(),
            inclusion_proofs,
            attestation: None,
        })
    }
}

/// A serialized Merkle receipt artifact (`merkle_receipt.json`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MerkleReceipt {
    pub schema_version: String,
    pub tree_size: usize,
    pub merkle_root: String,
    pub leaves: Vec<EvidenceLeaf>,
    pub inclusion_proofs: HashMap<String, MerkleInclusionProof>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attestation: Option<AttestationRecord>,
}

impl MerkleReceipt {
    /// Verify all inclusion proofs against the recorded Merkle root.
    pub fn verify_all(&self) -> Result<bool> {
        let root_bytes = from_hex(&self.merkle_root).map_err(|e| {
            CoreError::SecurityViolation(format!("Invalid hex in receipt merkle_root: {}", e))
        })?;
        if root_bytes.len() != 32 {
            return Err(CoreError::SecurityViolation(
                "Merkle root must be exactly 32 bytes".to_string(),
            ));
        }
        let mut root_arr = [0u8; 32];
        root_arr.copy_from_slice(&root_bytes);

        for (idx, leaf) in self.leaves.iter().enumerate() {
            let leaf_hash = leaf.leaf_hash()?;
            let leaf_hash_hex = to_hex(&leaf_hash);

            // Resilient lookup strategy:
            // 1. Primary canonical key: file_name ("01_candidate.diff")
            // 2. Interop key: 0-indexed leaf position string ("0", "1", ...)
            // 3. Interop key: hex-encoded leaf hash
            // 4. Interop key: content SHA-256
            let proof = self
                .inclusion_proofs
                .get(&leaf.file_name)
                .or_else(|| self.inclusion_proofs.get(&idx.to_string()))
                .or_else(|| self.inclusion_proofs.get(&leaf_hash_hex))
                .or_else(|| self.inclusion_proofs.get(&leaf.content_sha256))
                .ok_or_else(|| {
                    CoreError::SecurityViolation(format!(
                        "Missing inclusion proof for leaf '{}' (index {}, leaf hash {})",
                        leaf.file_name, idx, leaf_hash_hex
                    ))
                })?;

            if proof.leaf_index != idx {
                return Err(CoreError::SecurityViolation(format!(
                    "Inclusion proof leaf_index {} does not match leaf position {}",
                    proof.leaf_index, idx
                )));
            }

            let valid = proof.verify(&leaf_hash, &root_arr)?;
            if !valid {
                return Ok(false);
            }
        }

        Ok(true)
    }
}

fn to_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        use std::fmt::Write;
        let _ = write!(s, "{:02x}", b);
    }
    s
}

fn from_hex(hex_str: &str) -> std::result::Result<Vec<u8>, String> {
    let trimmed = hex_str.trim();
    if !trimmed.len().is_multiple_of(2) {
        return Err("Hex string has odd length".to_string());
    }
    let mut bytes = Vec::with_capacity(trimmed.len() / 2);
    for i in (0..trimmed.len()).step_by(2) {
        let byte_str = &trimmed[i..i + 2];
        let byte = u8::from_str_radix(byte_str, 16)
            .map_err(|e| format!("Invalid hex byte '{}': {}", byte_str, e))?;
        bytes.push(byte);
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_leaf(name: &str, sha: &str) -> EvidenceLeaf {
        EvidenceLeaf {
            artifact_type: "DIFF".to_string(),
            file_name: name.to_string(),
            content_sha256: sha.to_string(),
            size_bytes: 128,
            timestamp: "2026-10-29T16:00:00Z".to_string(),
        }
    }

    #[test]
    fn test_power_of_2_calculation() {
        assert_eq!(largest_power_of_2_less_than(0), 0);
        assert_eq!(largest_power_of_2_less_than(1), 0);
        assert_eq!(largest_power_of_2_less_than(2), 1);
        assert_eq!(largest_power_of_2_less_than(3), 2);
        assert_eq!(largest_power_of_2_less_than(4), 2);
        assert_eq!(largest_power_of_2_less_than(5), 4);
        assert_eq!(largest_power_of_2_less_than(6), 4);
        assert_eq!(largest_power_of_2_less_than(7), 4);
        assert_eq!(largest_power_of_2_less_than(8), 4);
        assert_eq!(largest_power_of_2_less_than(9), 8);
    }

    #[test]
    fn test_domain_separation() {
        // Ensure leaf hash prefix 0x00 and node prefix 0x01 produce distinct hashes
        let leaf = dummy_leaf(
            "01_candidate.diff",
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
        );
        let leaf_h = leaf.leaf_hash().unwrap();

        let node_h = hash_internal_node(&[0u8; 32], &[0u8; 32]);
        assert_ne!(leaf_h, node_h);
    }

    #[test]
    fn test_empty_tree_root() {
        let tree = MerkleTree::new(vec![]).unwrap();
        assert_eq!(tree.root_hex(), EMPTY_TREE_ROOT);
    }

    #[test]
    fn test_single_leaf_tree() {
        let leaf = dummy_leaf("01_candidate.diff", "aaa");
        let tree = MerkleTree::new(vec![leaf.clone()]).unwrap();
        assert_eq!(tree.len(), 1);

        let root = tree.root_hash();
        let leaf_h = leaf.leaf_hash().unwrap();
        assert_eq!(root, leaf_h);

        let proof = tree.inclusion_proof(0).unwrap();
        assert!(proof.audit_path.is_empty());
        assert!(proof.verify(&leaf_h, &root).unwrap());
    }

    #[test]
    fn test_inclusion_proofs_and_tamper_detection() {
        for n in [2, 3, 4, 5, 7, 8, 9, 15, 16] {
            let mut leaves = Vec::new();
            for i in 0..n {
                leaves.push(dummy_leaf(&format!("file_{i}.json"), &format!("{i:x}")));
            }

            let tree = MerkleTree::new(leaves.clone()).unwrap();
            let root = tree.root_hash();

            // Verify all legitimate proofs
            for (i, leaf) in leaves.iter().enumerate().take(n) {
                let proof = tree.inclusion_proof(i).unwrap();
                let leaf_h = leaf.leaf_hash().unwrap();
                assert!(proof.verify(&leaf_h, &root).unwrap());

                // Tamper with leaf hash -> must fail
                let mut tampered_leaf_h = leaf_h;
                tampered_leaf_h[0] ^= 0xff;
                assert!(!proof.verify(&tampered_leaf_h, &root).unwrap());

                // Tamper with root -> must fail
                let mut tampered_root = root;
                tampered_root[0] ^= 0xff;
                assert!(!proof.verify(&leaf_h, &tampered_root).unwrap());
            }

            // Test receipt serialization and verify_all
            let receipt = tree.to_receipt().unwrap();
            assert!(receipt.verify_all().unwrap());
        }
    }

    #[test]
    fn test_evidence_leaf_canonical_bytes_order_invariance() {
        let leaf_direct = EvidenceLeaf {
            artifact_type: "candidate_patch".to_string(),
            file_name: "01_candidate.diff".to_string(),
            content_sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
                .to_string(),
            size_bytes: 42,
            timestamp: "2026-10-29T16:00:00.000Z".to_string(),
        };

        // Permuted/reversed property order in raw JSON
        let raw_reversed = r#"{
            "timestamp": "2026-10-29T16:00:00.000Z",
            "size_bytes": 42,
            "file_name": "01_candidate.diff",
            "content_sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            "artifact_type": "candidate_patch"
        }"#;
        let leaf_deserialized: EvidenceLeaf = serde_json::from_str(raw_reversed).unwrap();

        let bytes_direct = leaf_direct.canonical_bytes().unwrap();
        let bytes_deserialized = leaf_deserialized.canonical_bytes().unwrap();
        assert_eq!(bytes_direct, bytes_deserialized);
        assert_eq!(
            leaf_direct.leaf_hash().unwrap(),
            leaf_deserialized.leaf_hash().unwrap()
        );

        // Exact RFC 8785 canonical string representation (UTF-16 key sort, compact delimiters)
        let expected_canonical = r#"{"artifact_type":"candidate_patch","content_sha256":"e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855","file_name":"01_candidate.diff","size_bytes":42,"timestamp":"2026-10-29T16:00:00.000Z"}"#;
        assert_eq!(String::from_utf8(bytes_direct).unwrap(), expected_canonical);
    }

    #[test]
    fn test_inclusion_proofs_alternative_key_lookups() {
        let leaves = vec![
            dummy_leaf("01_candidate.diff", "hash1"),
            dummy_leaf("02_preflight.json", "hash2"),
            dummy_leaf("03_audit_receipt.json", "hash3"),
        ];
        let tree = MerkleTree::new(leaves.clone()).unwrap();
        let base_receipt = tree.to_receipt().unwrap();
        assert!(base_receipt.verify_all().unwrap());

        // 1. Interop: Proofs keyed by index string ("0", "1", "2")
        let mut index_keyed = base_receipt.clone();
        index_keyed.inclusion_proofs.clear();
        for (i, leaf) in leaves.iter().enumerate() {
            let proof = base_receipt
                .inclusion_proofs
                .get(&leaf.file_name)
                .unwrap()
                .clone();
            index_keyed.inclusion_proofs.insert(i.to_string(), proof);
        }
        assert!(index_keyed.verify_all().unwrap());

        // 2. Interop: Proofs keyed by hex leaf hash
        let mut hash_keyed = base_receipt.clone();
        hash_keyed.inclusion_proofs.clear();
        for leaf in &leaves {
            let proof = base_receipt
                .inclusion_proofs
                .get(&leaf.file_name)
                .unwrap()
                .clone();
            let leaf_hex = leaf.leaf_hash_hex().unwrap();
            hash_keyed.inclusion_proofs.insert(leaf_hex, proof);
        }
        assert!(hash_keyed.verify_all().unwrap());

        // 3. Interop: Proofs keyed by content_sha256
        let mut content_keyed = base_receipt.clone();
        content_keyed.inclusion_proofs.clear();
        for leaf in &leaves {
            let proof = base_receipt
                .inclusion_proofs
                .get(&leaf.file_name)
                .unwrap()
                .clone();
            content_keyed
                .inclusion_proofs
                .insert(leaf.content_sha256.clone(), proof);
        }
        assert!(content_keyed.verify_all().unwrap());

        // 4. Tampered index mismatch: proof leaf_index != idx
        let mut tampered_index = base_receipt.clone();
        let mut mismatched_proof = tampered_index
            .inclusion_proofs
            .get("01_candidate.diff")
            .unwrap()
            .clone();
        mismatched_proof.leaf_index = 999;
        tampered_index
            .inclusion_proofs
            .insert("01_candidate.diff".to_string(), mismatched_proof);
        assert!(tampered_index.verify_all().is_err());

        // 5. Missing proof fail-closed
        let mut missing_proof = base_receipt.clone();
        missing_proof.inclusion_proofs.remove("01_candidate.diff");
        assert!(missing_proof.verify_all().is_err());
    }
}
