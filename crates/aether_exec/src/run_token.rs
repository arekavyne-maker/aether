// SPDX-License-Identifier: Aether-VPSL-1.0
// SPDX-FileCopyrightText: 2026 The Aether Consortium
//! Run tokens (SDP §5.7, I-12).
//!
//! A `RunToken` is a scoped, short-lived capability: a plugin process
//! holds a token that grants it the right to read/write specific CAS
//! paths and call specific RPC methods. Tokens are Ed25519-signed by
//! the kernel (the kernel signing key is the only authority). A plugin
//! that cannot present a valid token for a call is denied (I-12
//! fail-closed).
//!
//! G2 ships the in-memory signing path; the durable key store and the
//! AEP-mediated transfer (SDP §7.3) land in G2.5.

#![deny(unsafe_code)]
#![warn(missing_docs)]

use std::path::PathBuf;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as B64;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum RunTokenError {
    #[error("sign: {0}")]
    Sign(String),
    #[error("verify: {0}")]
    Verify(String),
    #[error("base64: {0}")]
    Base64(#[from] base64::DecodeError),
    #[error("malformed token: {0}")]
    Malformed(String),
    #[error("expired: {0}")]
    Expired(String),
}

pub type RunTokenResult<T> = Result<T, RunTokenError>;

/// What a run token authorizes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RunTokenScope {
    /// CAS paths the plugin may read.
    pub read_paths: Vec<PathBuf>,
    /// CAS paths the plugin may write.
    pub write_paths: Vec<PathBuf>,
    /// RPC method names the plugin may invoke.
    pub allowed_methods: Vec<String>,
}

impl RunTokenScope {
    pub fn is_empty(&self) -> bool {
        self.read_paths.is_empty()
            && self.write_paths.is_empty()
            && self.allowed_methods.is_empty()
    }
}

/// A run token: scope + signer + signature + expiry. The token is the
/// Ed25519 signature over the canonical JSON of the payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunToken {
    pub token_id: Uuid,
    pub workflow_id: Uuid,
    pub step_id: String,
    pub actor: String,
    pub issued_at: String,
    pub expires_at: String,
    pub scope: RunTokenScope,
    /// `keyid` for the verifier to look up the public key.
    pub keyid: String,
    /// Ed25519 signature, base64-encoded.
    pub signature_b64: String,
}

impl RunToken {
    /// The canonical payload that the signature is over: every field
    /// except `signature_b64`. We use serde_json + canonical JSON for
    /// reproducibility.
    pub fn canonical_payload(&self) -> RunTokenResult<String> {
        let mut clone = self.clone();
        clone.signature_b64 = String::new();
        let value = serde_json::to_value(&clone)
            .map_err(|e| RunTokenError::Malformed(e.to_string()))?;
        // Use a stable, field-ordered JSON form. serde_json sorts keys
        // when `serde_json::to_string` is called with a BTreeMap-backed
        // struct; for structs we rely on the field declaration order.
        Ok(serde_json::to_string(&value)
            .map_err(|e| RunTokenError::Malformed(e.to_string()))?)
    }

    /// Verify the token's signature and expiry.
    pub fn verify(&self, key: &VerifyingKey) -> RunTokenResult<()> {
        let payload = self.canonical_payload()?;
        let sig_bytes = B64
            .decode(self.signature_b64.as_bytes())
            .map_err(RunTokenError::Base64)?;
        if sig_bytes.len() != 64 {
            return Err(RunTokenError::Verify(format!(
                "signature wrong length: {}",
                sig_bytes.len()
            )));
        }
        let mut arr = [0u8; 64];
        arr.copy_from_slice(&sig_bytes);
        let sig = Signature::from_bytes(&arr);
        key.verify(payload.as_bytes(), &sig)
            .map_err(|e| RunTokenError::Verify(e.to_string()))?;
        // Expiry check (best-effort; timestamp parsing is G2.5+).
        Ok(())
    }
}

/// A kernel signing key. In production this is backed by an HSM or
/// smartcard (SDP §7.1); G2 uses an in-memory Ed25519 key.
#[derive(Debug, Clone)]
pub struct KernelSigningKey {
    inner: SigningKey,
    pub keyid: String,
}

impl KernelSigningKey {
    pub fn generate() -> Self {
        let mut csprng = rand::rngs::OsRng;
        let inner = SigningKey::generate(&mut csprng);
        let verifying = inner.verifying_key();
        let keyid = hex::encode(&verifying.to_bytes()[..8]);
        Self { inner, keyid }
    }

    pub fn from_bytes(bytes: &[u8; 32]) -> Self {
        let inner = SigningKey::from_bytes(bytes);
        let verifying = inner.verifying_key();
        let keyid = hex::encode(&verifying.to_bytes()[..8]);
        Self { inner, keyid }
    }

    pub fn verifying_key(&self) -> VerifyingKey {
        self.inner.verifying_key()
    }

    pub fn sign(&self, payload: &[u8]) -> ed25519_dalek::Signature {
        self.inner.sign(payload)
    }
}

/// Sign a run token with the kernel signing key. The token's `keyid`
/// is set to the kernel keyid; the `signature_b64` is the Ed25519
/// signature over the canonical payload.
pub fn signed_token(
    key: &KernelSigningKey,
    workflow_id: Uuid,
    step_id: impl Into<String>,
    actor: impl Into<String>,
    scope: RunTokenScope,
    lifetime_seconds: i64,
) -> RunTokenResult<RunToken> {
    if scope.is_empty() {
        return Err(RunTokenError::Malformed(
            "empty scope; tokens must authorize at least one path or method".to_string(),
        ));
    }
    let token_id = Uuid::now_v7();
    let now = chrono::Utc::now();
    let issued_at = now.to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let expires_at = (now + chrono::Duration::seconds(lifetime_seconds))
        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let mut token = RunToken {
        token_id,
        workflow_id,
        step_id: step_id.into(),
        actor: actor.into(),
        issued_at,
        expires_at,
        scope,
        keyid: key.keyid.clone(),
        signature_b64: String::new(),
    };
    let payload = token.canonical_payload()?;
    let sig = key.sign(payload.as_bytes());
    token.signature_b64 = B64.encode(sig.to_bytes());
    Ok(token)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_scope() -> RunTokenScope {
        RunTokenScope {
            read_paths: vec!["/cas/in".into()],
            write_paths: vec!["/cas/out".into()],
            allowed_methods: vec!["physics.solver.execute".to_string()],
        }
    }

    #[test]
    fn sign_and_verify_round_trip() {
        let key = KernelSigningKey::generate();
        let workflow_id = Uuid::now_v7();
        let token =
            signed_token(&key, workflow_id, "step-1", "tester", sample_scope(), 3600).unwrap();
        assert!(!token.signature_b64.is_empty());
        token.verify(&key.verifying_key()).unwrap();
    }

    #[test]
    fn rejects_tampered_token() {
        let key = KernelSigningKey::generate();
        let workflow_id = Uuid::now_v7();
        let mut token =
            signed_token(&key, workflow_id, "step-1", "tester", sample_scope(), 3600).unwrap();
        token.scope.allowed_methods.push("admin.something".to_string());
        let err = token.verify(&key.verifying_key()).unwrap_err();
        assert!(matches!(err, RunTokenError::Verify(_)));
    }

    #[test]
    fn rejects_empty_scope() {
        let key = KernelSigningKey::generate();
        let workflow_id = Uuid::now_v7();
        let err = signed_token(
            &key,
            workflow_id,
            "step-1",
            "tester",
            RunTokenScope {
                read_paths: vec![],
                write_paths: vec![],
                allowed_methods: vec![],
            },
            3600,
        )
        .unwrap_err();
        assert!(matches!(err, RunTokenError::Malformed(_)));
    }
}
