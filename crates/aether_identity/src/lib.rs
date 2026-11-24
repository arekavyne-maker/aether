// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! Identity tokens with expiry and revocation.
//!
//! Every user, service, plugin, and worker authenticates independently
//! (A3-I13). This crate issues short-lived, signed bearer tokens and
//! verifies them, with an explicit revocation list. A token that is
//! expired, tampered with, or revoked is rejected — there is no path that
//! treats an unverifiable token as valid.
//!
//! The signature is a SHA-256 MAC over the canonical token body, keyed by
//! a site secret. For a public-key deployment the same interface is
//! backed by Ed25519 in `aether_registry`.

#![deny(unsafe_code)]
#![warn(missing_docs)]

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

/// Token errors.
#[derive(Debug, Error, PartialEq)]
pub enum TokenError {
    /// The token is malformed.
    #[error("malformed token: {0}")]
    Malformed(String),
    /// The signature does not verify.
    #[error("signature mismatch")]
    BadSignature,
    /// The token has expired.
    #[error("token expired at {0}")]
    Expired(String),
    /// The token has been revoked.
    #[error("token {0} is revoked")]
    Revoked(String),
    /// The audience does not match.
    #[error("audience {0:?} does not match {1:?}")]
    Audience(String, String),
}

/// A token's claims.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Claims {
    /// Subject identity.
    pub sub: String,
    /// Role.
    pub role: String,
    /// Audience (service or project).
    pub aud: String,
    /// Issued at (ISO-8601).
    pub iat: String,
    /// Expiry (ISO-8601).
    pub exp: String,
    /// Token id, used for revocation.
    pub jti: String,
}

/// A signed token.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Token {
    /// Claims.
    pub claims: Claims,
    /// Hex signature over the canonical claims.
    pub signature: String,
}

/// The issuer and verifier.
#[derive(Debug, Clone)]
pub struct IdentityService {
    secret: Vec<u8>,
    revoked: BTreeSet<String>,
}

impl IdentityService {
    /// Create a service with a site secret.
    pub fn new(secret: &[u8]) -> Self {
        Self {
            secret: secret.to_vec(),
            revoked: BTreeSet::new(),
        }
    }

    /// Issue a token.
    pub fn issue(&self, claims: Claims) -> Token {
        let signature = self.sign(&claims);
        Token { claims, signature }
    }

    /// Revoke a token id.
    pub fn revoke(&mut self, jti: &str) {
        self.revoked.insert(jti.to_string());
    }

    /// Verify a token for an expected audience at a given time.
    ///
    /// `now` is an ISO-8601 timestamp; the caller supplies it so the check
    /// is deterministic and testable.
    pub fn verify(
        &self,
        token: &Token,
        expected_audience: &str,
        now: &str,
    ) -> Result<Claims, TokenError> {
        if token.signature != self.sign(&token.claims) {
            return Err(TokenError::BadSignature);
        }
        if self.revoked.contains(&token.claims.jti) {
            return Err(TokenError::Revoked(token.claims.jti.clone()));
        }
        if token.claims.exp.as_str() < now {
            return Err(TokenError::Expired(token.claims.exp.clone()));
        }
        if token.claims.aud != expected_audience {
            return Err(TokenError::Audience(
                token.claims.aud.clone(),
                expected_audience.to_string(),
            ));
        }
        Ok(token.claims.clone())
    }

    fn sign(&self, claims: &Claims) -> String {
        let canonical = serde_json::json!({
            "sub": claims.sub,
            "role": claims.role,
            "aud": claims.aud,
            "iat": claims.iat,
            "exp": claims.exp,
            "jti": claims.jti,
        });
        let bytes = serde_json::to_vec(&canonical).unwrap_or_default();
        let mut hasher = Sha256::new();
        hasher.update(&self.secret);
        hasher.update(b"|");
        hasher.update(bytes);
        format!("{:x}", hasher.finalize())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn claims() -> Claims {
        Claims {
            sub: "user:alice".to_string(),
            role: "project-engineer".to_string(),
            aud: "project:falcon".to_string(),
            iat: "2026-11-24T00:00:00Z".to_string(),
            exp: "2026-11-24T01:00:00Z".to_string(),
            jti: "token-1".to_string(),
        }
    }

    #[test]
    fn valid_token_verifies() {
        let service = IdentityService::new(b"secret");
        let token = service.issue(claims());
        let verified = service
            .verify(&token, "project:falcon", "2026-11-24T00:30:00Z")
            .unwrap();
        assert_eq!(verified.sub, "user:alice");
    }

    #[test]
    fn expired_token_is_rejected() {
        let service = IdentityService::new(b"secret");
        let token = service.issue(claims());
        let err = service
            .verify(&token, "project:falcon", "2026-11-24T02:00:00Z")
            .unwrap_err();
        assert!(matches!(err, TokenError::Expired(_)));
    }

    #[test]
    fn tampered_token_is_rejected() {
        let service = IdentityService::new(b"secret");
        let mut token = service.issue(claims());
        token.claims.role = "admin".to_string();
        let err = service
            .verify(&token, "project:falcon", "2026-11-24T00:30:00Z")
            .unwrap_err();
        assert!(matches!(err, TokenError::BadSignature));
    }

    #[test]
    fn revoked_token_is_rejected() {
        let mut service = IdentityService::new(b"secret");
        let token = service.issue(claims());
        service.revoke("token-1");
        let err = service
            .verify(&token, "project:falcon", "2026-11-24T00:30:00Z")
            .unwrap_err();
        assert!(matches!(err, TokenError::Revoked(_)));
    }

    #[test]
    fn wrong_audience_is_rejected() {
        let service = IdentityService::new(b"secret");
        let token = service.issue(claims());
        let err = service
            .verify(&token, "project:other", "2026-11-24T00:30:00Z")
            .unwrap_err();
        assert!(matches!(err, TokenError::Audience(_, _)));
    }
}