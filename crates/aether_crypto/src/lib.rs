// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! Encryption at rest and signed release manifests (ADR-0039).
//!
//! A stream cipher is implemented deterministically with SHA-256 in counter
//! mode, keyed by a site secret. This is a self-contained, dependency-free
//! mechanism suitable for local at-rest protection of small artifacts; a
//! production site may replace it with AES-GCM behind the same trait.
//! Release manifests are signed with SHA-256 MACs over canonical bytes and
//! verified on import. No compliance claim is made.

#![deny(unsafe_code)]
#![warn(missing_docs)]

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

/// Encryption errors.
#[derive(Debug, Error, PartialEq)]
pub enum CryptoError {
    /// A MAC does not verify.
    #[error("signature verification failed")]
    BadSignature,
    /// An input is empty.
    #[error("empty input")]
    Empty,
}

/// Deterministic stream encryption for local at-rest storage.
pub fn encrypt(secret: &[u8], nonce: &[u8; 16], plaintext: &[u8]) -> Vec<u8> {
    let stream = keystream(secret, nonce, plaintext.len());
    plaintext
        .iter()
        .zip(stream.iter())
        .map(|(p, k)| p ^ k)
        .collect()
}

/// Decrypt bytes produced by [`encrypt`].
pub fn decrypt(secret: &[u8], nonce: &[u8; 16], ciphertext: &[u8]) -> Vec<u8> {
    encrypt(secret, nonce, ciphertext)
}

fn keystream(secret: &[u8], nonce: &[u8; 16], len: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(len);
    let mut counter = 0_u64;
    while out.len() < len {
        let mut hasher = Sha256::new();
        hasher.update(secret);
        hasher.update(nonce);
        hasher.update(counter.to_be_bytes());
        out.extend_from_slice(&hasher.finalize());
        counter += 1;
    }
    out.truncate(len);
    out
}

/// A signed release manifest.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReleaseManifest {
    /// Release id.
    pub id: String,
    /// Artifact names and hashes.
    pub artifacts: Vec<(String, String)>,
    /// Signature over canonical manifest body.
    pub signature: String,
}

impl ReleaseManifest {
    /// Sign a manifest body.
    pub fn sign(secret: &[u8], id: &str, artifacts: Vec<(String, String)>) -> Self {
        let body = serde_json::json!({ "id": id, "artifacts": artifacts });
        let signature = mac(secret, &serde_json::to_vec(&body).unwrap_or_default());
        Self {
            id: id.to_string(),
            artifacts,
            signature,
        }
    }

    /// Verify the signature and all artifact hashes.
    pub fn verify(&self, secret: &[u8]) -> Result<(), CryptoError> {
        if self.id.trim().is_empty() || self.artifacts.is_empty() {
            return Err(CryptoError::Empty);
        }
        let body = serde_json::json!({
            "id": self.id,
            "artifacts": self.artifacts,
        });
        let expected = mac(secret, &serde_json::to_vec(&body).unwrap_or_default());
        if expected != self.signature {
            return Err(CryptoError::BadSignature);
        }
        for (_, hash) in &self.artifacts {
            if hash.len() != 64 || !hash.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Err(CryptoError::BadSignature);
            }
        }
        Ok(())
    }
}

fn mac(secret: &[u8], bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(secret);
    hasher.update(b"|");
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_encryption() {
        let secret = b"site-key";
        let nonce = [1_u8; 16];
        let plaintext = b"controlled engineering content";
        let ciphertext = encrypt(secret, &nonce, plaintext);
        assert_ne!(ciphertext, plaintext);
        assert_eq!(decrypt(secret, &nonce, &ciphertext), plaintext);
    }

    #[test]
    fn wrong_key_produces_garbage() {
        let nonce = [2_u8; 16];
        let ciphertext = encrypt(b"a", &nonce, b"secret");
        assert_ne!(decrypt(b"b", &nonce, &ciphertext), b"secret");
    }

    #[test]
    fn signed_manifest_verifies() {
        let manifest = ReleaseManifest::sign(
            b"key",
            "rel-1",
            vec![("a.bin".to_string(), "a".repeat(64))],
        );
        assert!(manifest.verify(b"key").is_ok());
    }

    #[test]
    fn tampered_manifest_is_rejected() {
        let mut manifest = ReleaseManifest::sign(
            b"key",
            "rel-1",
            vec![("a.bin".to_string(), "a".repeat(64))],
        );
        manifest.artifacts[0].1 = "b".repeat(64);
        assert_eq!(manifest.verify(b"key"), Err(CryptoError::BadSignature));
    }
}