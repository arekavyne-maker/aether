// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! Portable job envelope (ADR-0036, spec §7.1).
//!
//! Every backend consumes this envelope. It binds the job to an exact
//! configuration snapshot, input hashes, solver identity and version,
//! environment digest, classification, timeout and validation rules.
//! Idempotency keys make retries safe across backends.

#![deny(unsafe_code)]
#![warn(missing_docs)]

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

/// Data classification for a job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobClassification {
    Public,
    Internal,
    Confidential,
    Restricted,
    ExportControlled,
}

/// A reference to an input artifact.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InputRef {
    /// Artifact identifier or path.
    pub artifact: String,
    /// SHA-256 of the artifact bytes.
    pub hash: String,
}

/// Solver identity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolverRef {
    /// Plugin id.
    pub plugin_id: String,
    /// Version.
    pub version: String,
}

/// Resource requirements.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct JobResources {
    /// Memory in GB.
    #[serde(default)]
    pub memory_gb: f64,
    /// CPU cores.
    #[serde(default)]
    pub cores: u32,
}

/// The portable job envelope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JobEnvelope {
    /// Schema marker.
    pub schema: String,
    /// Job id.
    pub job_id: String,
    /// Idempotency key; retries reuse it.
    pub idempotency_key: String,
    /// Configuration snapshot id.
    pub configuration_snapshot: String,
    /// Input artifacts and hashes.
    pub inputs: Vec<InputRef>,
    /// Solver identity.
    pub solver: SolverRef,
    /// Environment digest.
    pub environment_digest: String,
    /// Parameter set.
    #[serde(default)]
    pub parameters: BTreeMap<String, serde_json::Value>,
    /// Resource requirements.
    #[serde(default)]
    pub resources: JobResources,
    /// Data classification.
    pub classification: JobClassification,
    /// Timeout in seconds.
    pub timeout_seconds: u64,
    /// Expected outputs.
    #[serde(default)]
    pub expected_outputs: Vec<String>,
    /// Validation rules.
    #[serde(default)]
    pub validation: Vec<String>,
    /// Provenance policy.
    pub provenance_policy: String,
}

/// Envelope errors.
#[derive(Debug, Error, PartialEq)]
pub enum EnvelopeError {
    /// A required field is malformed.
    #[error("invalid envelope: {0}")]
    Invalid(String),
    /// An input hash is not a 64-character hex string.
    #[error("invalid hash for {0}")]
    InvalidHash(String),
}

impl JobEnvelope {
    /// Validate the envelope before submission.
    pub fn validate(&self) -> Result<(), EnvelopeError> {
        if self.schema != "aether.job-envelope/v1" {
            return Err(EnvelopeError::Invalid(format!(
                "unknown schema {}",
                self.schema
            )));
        }
        if self.job_id.trim().is_empty()
            || self.idempotency_key.trim().is_empty()
            || self.configuration_snapshot.trim().is_empty()
        {
            return Err(EnvelopeError::Invalid(
                "job_id, idempotency_key and configuration_snapshot are required".into(),
            ));
        }
        if self.timeout_seconds == 0 {
            return Err(EnvelopeError::Invalid("timeout_seconds must be > 0".into()));
        }
        for input in &self.inputs {
            if input.hash.len() != 64 || !input.hash.bytes().all(|c| c.is_ascii_hexdigit()) {
                return Err(EnvelopeError::InvalidHash(input.artifact.clone()));
            }
        }
        Ok(())
    }

    /// Canonical hash of the envelope for provenance.
    pub fn content_hash(&self) -> String {
        let bytes = serde_json::to_vec(self).unwrap_or_default();
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        format!("{:x}", hasher.finalize())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn envelope() -> JobEnvelope {
        JobEnvelope {
            schema: "aether.job-envelope/v1".to_string(),
            job_id: "job-1".to_string(),
            idempotency_key: "key-1".to_string(),
            configuration_snapshot: "snap-1".to_string(),
            inputs: vec![InputRef {
                artifact: "mesh.inp".to_string(),
                hash: "a".repeat(64),
            }],
            solver: SolverRef {
                plugin_id: "org.aether.fe.spar".to_string(),
                version: "0.1.0".to_string(),
            },
            environment_digest: "sha256:00".to_string(),
            parameters: BTreeMap::new(),
            resources: JobResources::default(),
            classification: JobClassification::Internal,
            timeout_seconds: 3600,
            expected_outputs: vec!["result.bundle".to_string()],
            validation: vec!["units".to_string()],
            provenance_policy: "full".to_string(),
        }
    }

    #[test]
    fn valid_envelope_passes() {
        assert!(envelope().validate().is_ok());
    }

    #[test]
    fn bad_hash_is_rejected() {
        let mut e = envelope();
        e.inputs[0].hash = "bad".to_string();
        assert!(matches!(e.validate(), Err(EnvelopeError::InvalidHash(_))));
    }

    #[test]
    fn missing_idempotency_key_is_rejected() {
        let mut e = envelope();
        e.idempotency_key = String::new();
        assert!(e.validate().is_err());
    }

    #[test]
    fn hash_is_stable() {
        let e = envelope();
        assert_eq!(e.content_hash(), e.content_hash());
    }
}