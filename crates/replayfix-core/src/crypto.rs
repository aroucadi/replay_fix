//! Pure Ed25519 cryptographic signing, attestation records, and public-key trust policies.

use crate::error::{CoreError, Result};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand_core::OsRng;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Canonical attestation record embedded in receipts.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AttestationRecord {
    pub schema_version: String,
    pub algorithm: String,
    pub public_key: String,
    pub signature: String,
    pub signing_payload: String,
    pub created_at: String,
}

/// Construct the canonical, versioned signing payload.
#[must_use]
pub fn build_signing_payload(
    schema_version: &str,
    merkle_root: &str,
    tree_size: usize,
    created_at: &str,
) -> String {
    format!(
        "REPLAYFIX-ATTESTATION:v0.9:{}:{}:{}:{}",
        schema_version, merkle_root, tree_size, created_at
    )
}

/// Public key trust policy for receipt verification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrustPolicy {
    /// Any structurally and cryptographically valid signature is accepted.
    AllowAny,
    /// Signature must be signed by one of the pinned public keys (in hex).
    PinnedKeys(HashSet<String>),
}

/// Outcome of attestation verification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerificationOutcome {
    VerifiedTrusted,
    VerifiedUntrustedSigner,
}

/// Manager for generating or loading Ed25519 signing keys.
pub struct KeypairManager {
    signing_key: SigningKey,
}

impl std::fmt::Debug for KeypairManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KeypairManager")
            .field("public_key", &self.public_key_hex())
            .finish()
    }
}

impl KeypairManager {
    /// Generate an ephemeral keypair using system randomness.
    #[must_use]
    pub fn generate() -> Self {
        let signing_key = SigningKey::generate(&mut OsRng);
        Self { signing_key }
    }

    /// Load a signing key from raw 32-byte secret key bytes.
    pub fn from_bytes(bytes: &[u8; 32]) -> Self {
        let signing_key = SigningKey::from_bytes(bytes);
        Self { signing_key }
    }

    /// Load a signing key from a 64-character hexadecimal string.
    pub fn from_hex(hex_str: &str) -> Result<Self> {
        let bytes = from_hex(hex_str).map_err(|e| {
            CoreError::SecurityViolation(format!("Invalid hex in signing key: {}", e))
        })?;
        if bytes.len() != 32 {
            return Err(CoreError::SecurityViolation(
                "Ed25519 secret key must be exactly 32 bytes".to_string(),
            ));
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(Self::from_bytes(&arr))
    }

    /// Verifying (public) key for this keypair.
    #[must_use]
    pub fn verifying_key(&self) -> VerifyingKey {
        self.signing_key.verifying_key()
    }

    /// Hex-encoded 32-byte public key string.
    #[must_use]
    pub fn public_key_hex(&self) -> String {
        to_hex(self.verifying_key().as_bytes())
    }

    /// Sign a canonical signing payload and emit an `AttestationRecord`.
    pub fn sign(
        &self,
        schema_version: &str,
        merkle_root: &str,
        tree_size: usize,
        created_at: &str,
    ) -> Result<AttestationRecord> {
        let signing_payload =
            build_signing_payload(schema_version, merkle_root, tree_size, created_at);
        let signature: Signature = self.signing_key.sign(signing_payload.as_bytes());

        Ok(AttestationRecord {
            schema_version: schema_version.to_string(),
            algorithm: "Ed25519".to_string(),
            public_key: self.public_key_hex(),
            signature: to_hex(&signature.to_bytes()),
            signing_payload,
            created_at: created_at.to_string(),
        })
    }
}

/// Verify an `AttestationRecord` against an expected Merkle root and trust policy.
pub fn verify_attestation(
    attestation: &AttestationRecord,
    expected_merkle_root: &str,
    expected_tree_size: usize,
    trust_policy: &TrustPolicy,
) -> Result<VerificationOutcome> {
    if attestation.algorithm != "Ed25519" {
        return Err(CoreError::SecurityViolation(format!(
            "Unsupported signature algorithm '{}': expected 'Ed25519'",
            attestation.algorithm
        )));
    }

    // Verify expected signing payload structure
    let expected_payload = build_signing_payload(
        &attestation.schema_version,
        expected_merkle_root,
        expected_tree_size,
        &attestation.created_at,
    );
    if attestation.signing_payload != expected_payload {
        return Err(CoreError::SecurityViolation(
            "Signing payload mismatch: attestation does not bind expected Merkle root or tree size"
                .to_string(),
        ));
    }

    // Decode public key
    let pubkey_bytes = from_hex(&attestation.public_key)
        .map_err(|e| CoreError::SecurityViolation(format!("Invalid hex in public key: {}", e)))?;
    if pubkey_bytes.len() != 32 {
        return Err(CoreError::SecurityViolation(
            "Ed25519 public key must be exactly 32 bytes".to_string(),
        ));
    }
    let mut pubkey_arr = [0u8; 32];
    pubkey_arr.copy_from_slice(&pubkey_bytes);
    let verifying_key = VerifyingKey::from_bytes(&pubkey_arr).map_err(|e| {
        CoreError::SecurityViolation(format!("Invalid Ed25519 public key bytes: {}", e))
    })?;

    // Decode signature
    let sig_bytes = from_hex(&attestation.signature)
        .map_err(|e| CoreError::SecurityViolation(format!("Invalid hex in signature: {}", e)))?;
    if sig_bytes.len() != 64 {
        return Err(CoreError::SecurityViolation(
            "Ed25519 signature must be exactly 64 bytes".to_string(),
        ));
    }
    let mut sig_arr = [0u8; 64];
    sig_arr.copy_from_slice(&sig_bytes);
    let signature = Signature::from_bytes(&sig_arr);

    // Cryptographic verification
    verifying_key
        .verify(attestation.signing_payload.as_bytes(), &signature)
        .map_err(|e| {
            CoreError::SecurityViolation(format!(
                "Ed25519 cryptographic signature verification failed: {}",
                e
            ))
        })?;

    // Policy evaluation
    match trust_policy {
        TrustPolicy::AllowAny => Ok(VerificationOutcome::VerifiedUntrustedSigner),
        TrustPolicy::PinnedKeys(pinned) => {
            let pub_clean = attestation.public_key.to_ascii_lowercase();
            if pinned.contains(&pub_clean) {
                Ok(VerificationOutcome::VerifiedTrusted)
            } else {
                Err(CoreError::SecurityViolation(format!(
                    "Signer public key '{}' is not present in the pinned trust policy",
                    attestation.public_key
                )))
            }
        }
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

    #[test]
    fn test_sign_and_verify_roundtrip() {
        let kp = KeypairManager::generate();
        let merkle_root = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
        let tree_size = 5;
        let created_at = "2026-10-29T16:00:00Z";

        let attestation = kp.sign("0.9", merkle_root, tree_size, created_at).unwrap();
        assert_eq!(attestation.algorithm, "Ed25519");
        assert_eq!(attestation.public_key.len(), 64);
        assert_eq!(attestation.signature.len(), 128);

        // Verification with AllowAny
        let outcome =
            verify_attestation(&attestation, merkle_root, tree_size, &TrustPolicy::AllowAny)
                .unwrap();
        assert_eq!(outcome, VerificationOutcome::VerifiedUntrustedSigner);

        // Verification with PinnedKeys
        let mut pinned = HashSet::new();
        pinned.insert(kp.public_key_hex());
        let outcome = verify_attestation(
            &attestation,
            merkle_root,
            tree_size,
            &TrustPolicy::PinnedKeys(pinned),
        )
        .unwrap();
        assert_eq!(outcome, VerificationOutcome::VerifiedTrusted);
    }

    #[test]
    fn test_tampered_payload_or_root_fails_closed() {
        let kp = KeypairManager::generate();
        let merkle_root = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
        let attestation = kp
            .sign("0.9", merkle_root, 5, "2026-10-29T16:00:00Z")
            .unwrap();

        // 1. Root mismatch
        let wrong_root = "0000000000000000000000000000000000000000000000000000000000000000";
        assert!(verify_attestation(&attestation, wrong_root, 5, &TrustPolicy::AllowAny).is_err());

        // 2. Tree size mismatch
        assert!(verify_attestation(&attestation, merkle_root, 6, &TrustPolicy::AllowAny).is_err());

        // 3. Corrupted signature
        let mut tampered = attestation.clone();
        tampered.signature = "aa".repeat(64);
        assert!(verify_attestation(&tampered, merkle_root, 5, &TrustPolicy::AllowAny).is_err());
    }

    #[test]
    fn test_untrusted_signer_rejected_under_pinned_policy() {
        let kp1 = KeypairManager::generate();
        let kp2 = KeypairManager::generate();

        let attestation = kp1.sign("0.9", "abc", 1, "2026-10-29T16:00:00Z").unwrap();

        let mut pinned = HashSet::new();
        pinned.insert(kp2.public_key_hex()); // kp2 is pinned, but kp1 signed!

        let err = verify_attestation(&attestation, "abc", 1, &TrustPolicy::PinnedKeys(pinned));
        assert!(err.is_err());
    }
}
