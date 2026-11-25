// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! Loopback token authentication.
//!
//! The server binds to 127.0.0.1 only. On first launch a random token is
//! written to `~/.aether/web.token` with mode 0600 and the URL is printed
//! with the token in the fragment. The browser stores it in sessionStorage
//! and sends it as a bearer token on every API request.

#![deny(unsafe_code)]
#![warn(missing_docs)]

use std::io::Read;
use std::path::PathBuf;

use anyhow::{Context, Result, anyhow};
use thiserror::Error;

/// Token errors.
#[derive(Debug, Error)]
pub enum AuthError {
    /// The supplied token is invalid.
    #[error("invalid token")]
    Invalid,
    /// No Authorization header was supplied.
    #[error("missing token")]
    Missing,
}

/// A bearer token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenAuth {
    secret: Vec<u8>,
}

impl TokenAuth {
    /// Build an auth instance from a raw token.
    pub fn new(secret: Vec<u8>) -> Self {
        Self { secret }
    }

    /// The token as a display string.
    pub fn as_str(&self) -> String {
        String::from_utf8_lossy(&self.secret).to_string()
    }

    /// Constant-time comparison against a supplied bearer value.
    pub fn verify(&self, supplied: &str) -> Result<(), AuthError> {
        let provided = supplied.as_bytes();
        if provided.len() != self.secret.len() {
            return Err(AuthError::Invalid);
        }
        let mut diff = 0_u8;
        for (a, b) in provided.iter().zip(self.secret.iter()) {
            diff |= a ^ b;
        }
        if diff == 0 {
            Ok(())
        } else {
            Err(AuthError::Invalid)
        }
    }
}

/// Path to the local token file.
pub fn token_path() -> Result<PathBuf> {
    let home = std::env::var("HOME").context("HOME is not set")?;
    Ok(PathBuf::from(home).join(".aether").join("web.token"))
}

fn random_token() -> Result<Vec<u8>> {
    let mut file = std::fs::File::open("/dev/urandom")
        .context("cannot read /dev/urandom for token generation")?;
    let mut bytes = vec![0_u8; 32];
    file.read_exact(&mut bytes)?;
    Ok(bytes
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>()
        .into_bytes())
}

/// True when a token has the generated form: 64 lower-case hex characters.
pub fn is_valid_token(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

/// Load an existing token or create one with restrictive permissions.
pub fn load_or_create_token() -> Result<TokenAuth> {
    if let Ok(override_token) = std::env::var("AETHER_WEB_TOKEN") {
        if !is_valid_token(&override_token) {
            return Err(anyhow!(
                "AETHER_WEB_TOKEN must be exactly 64 lower-case hex characters"
            ));
        }
        return Ok(TokenAuth::new(override_token.into_bytes()));
    }
    let path = token_path()?;
    if let Ok(existing) = std::fs::read(&path) {
        let token = String::from_utf8(existing)?.trim().to_string();
        if !token.is_empty() {
            return Ok(TokenAuth::new(token.into_bytes()));
        }
    }
    let token = random_token()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create(true).truncate(true).mode(0o600);
        let mut file = options.open(&path)?;
        use std::io::Write;
        file.write_all(&token)?;
    }
    #[cfg(not(unix))]
    std::fs::write(&path, &token)?;
    Ok(TokenAuth::new(token))
}

/// Extract a bearer token from an Authorization header value.
pub fn bearer(header: &str) -> Result<&str, AuthError> {
    let value = header.strip_prefix("Bearer ").ok_or(AuthError::Missing)?;
    if value.is_empty() {
        return Err(AuthError::Missing);
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verifies_exact_token() {
        let auth = TokenAuth::new(b"abc123".to_vec());
        assert!(auth.verify("abc123").is_ok());
        assert!(matches!(auth.verify("abc124"), Err(AuthError::Invalid)));
        assert!(matches!(auth.verify("abc"), Err(AuthError::Invalid)));
    }

    #[test]
    fn extracts_bearer() {
        assert_eq!(bearer("Bearer xyz").unwrap(), "xyz");
        assert!(bearer("xyz").is_err());
    }

    #[test]
    fn token_form_is_enforced() {
        assert!(is_valid_token(&"a".repeat(64)));
        assert!(!is_valid_token("short"));
        assert!(!is_valid_token(&"A".repeat(64)));
    }

    #[test]
    fn random_token_is_hex_and_64_chars() {
        let token = random_token().unwrap();
        assert_eq!(token.len(), 64);
        assert!(token.iter().all(|b| b.is_ascii_hexdigit()));
    }
}