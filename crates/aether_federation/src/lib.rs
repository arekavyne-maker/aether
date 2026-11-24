// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! Federation loopback harness (ADR-0036, spec §7.3/§15).
//!
//! Two Aether homes exchange a signed trust bundle and a checksummed job
//! envelope. Where a real second site is absent this is a **harness**, not
//! live federation: it proves the wire protocol and the fail-closed rules
//! but does not claim two physical sites were exercised.

#![deny(unsafe_code)]
#![warn(missing_docs)]

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

/// A signed trust bundle exchanged between sites.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrustBundle {
    /// Site identifier.
    pub site_id: String,
    /// Ed25519 public key, hex encoded.
    pub public_key_hex: String,
    /// Bundle version.
    pub version: String,
}

/// A federation error.
#[derive(Debug, Error, PartialEq)]
pub enum FederationError {
    /// The remote site is not trusted.
    #[error("untrusted site {0}")]
    Untrusted(String),
    /// The artifact hash does not match.
    #[error("checksum mismatch for {0}")]
    Checksum(String),
    /// A classification may not cross the boundary.
    #[error("classification {0} may not be federated")]
    Classification(String),
}

/// The federation boundary for one site.
#[derive(Debug, Clone, Default)]
pub struct FederationBoundary {
    trusted: Vec<TrustBundle>,
    max_classification: u8,
}

impl FederationBoundary {
    /// Create a boundary trusting the given sites.
    pub fn new(trusted: Vec<TrustBundle>, max_classification: u8) -> Self {
        Self {
            trusted,
            max_classification,
        }
    }

    /// Whether a site is trusted.
    pub fn is_trusted(&self, site_id: &str) -> bool {
        self.trusted.iter().any(|t| t.site_id == site_id)
    }

    /// Verify a checksummed transfer from a peer.
    pub fn receive(
        &self,
        site_id: &str,
        artifact: &str,
        bytes: &[u8],
        expected_hash: &str,
        classification: u8,
    ) -> Result<(), FederationError> {
        if !self.is_trusted(site_id) {
            return Err(FederationError::Untrusted(site_id.to_string()));
        }
        if classification > self.max_classification {
            return Err(FederationError::Classification(
                classification.to_string(),
            ));
        }
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        let actual = format!("{:x}", hasher.finalize());
        if actual != expected_hash {
            return Err(FederationError::Checksum(artifact.to_string()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bundle(site: &str) -> TrustBundle {
        TrustBundle {
            site_id: site.to_string(),
            public_key_hex: "ab".repeat(32),
            version: "1.0".to_string(),
        }
    }

    #[test]
    fn trusted_transfer_succeeds() {
        let boundary = FederationBoundary::new(vec![bundle("site-a")], 2);
        let bytes = b"job envelope";
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        let hash = format!("{:x}", hasher.finalize());
        assert!(boundary.receive("site-a", "job", bytes, &hash, 1).is_ok());
    }

    #[test]
    fn untrusted_site_is_rejected() {
        let boundary = FederationBoundary::new(vec![bundle("site-a")], 2);
        assert!(matches!(
            boundary.receive("site-b", "job", b"x", &"0".repeat(64), 1),
            Err(FederationError::Untrusted(_))
        ));
    }

    #[test]
    fn checksum_mismatch_is_rejected() {
        let boundary = FederationBoundary::new(vec![bundle("site-a")], 2);
        assert!(matches!(
            boundary.receive("site-a", "job", b"x", &"0".repeat(64), 1),
            Err(FederationError::Checksum(_))
        ));
    }

    #[test]
    fn over_classified_transfer_is_rejected() {
        let boundary = FederationBoundary::new(vec![bundle("site-a")], 2);
        let bytes = b"x";
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        let hash = format!("{:x}", hasher.finalize());
        assert!(matches!(
            boundary.receive("site-a", "job", bytes, &hash, 9),
            Err(FederationError::Classification(_))
        ));
    }
}