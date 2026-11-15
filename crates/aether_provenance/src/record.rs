// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! Provenance records (SDP §14.2).
//!
//! A `ProvenanceRecord` is the per-artifact derivation chain. It records
//! actor, tool, inputs, parameters, environment, placement, outputs, and
//! verification outcomes. Records are content-addressed: their `record_hash`
//! is the canonical SHA-256 of the record's JSON form (without the
//! `record_hash` field itself).

#![deny(unsafe_code)]

use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::{canonical_hash, now_rfc3339, sha256_hex};

#[derive(Debug, Error)]
pub enum RecordError {
    #[error("missing required field: {0}")]
    MissingField(&'static str),
    #[error("invalid hash: {0}")]
    InvalidHash(String),
    #[error("verification mismatch: {0}")]
    VerificationMismatch(String),
}

pub type RecordResult<T> = Result<T, RecordError>;

/// The actor that produced a record. Mirrors the `actor` field on the
/// graph event envelope.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Actor {
    /// Stable actor id (person or service).
    pub id: String,
    /// Optional role for policy attribution.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    /// Optional plugin instance id when the actor is a plugin.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugin_instance: Option<String>,
}

/// Authorization decision attached to the record.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuthorizationDecision {
    /// `allow` or `deny`; the kernel records the actual outcome.
    pub decision: String,
    /// Policy bundle reference (e.g. policy version + hash).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policy_ref: Option<String>,
    /// Optional rationale for the decision.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rationale: Option<String>,
}

/// A provenance record (SDP §14.2).
///
/// The schema mirrors `prov/record.v1` (in the G2 deliverable; Phase 0 has
/// the same fields inline). G1's conformance contract is:
/// - `record_id`, `produced_at`, `tool`, `inputs`, `outputs` are required.
/// - `record_hash` is computed; tampering causes chain verification to fail.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceRecord {
    /// UUIDv7; stable for the lifetime of the record.
    pub record_id: Uuid,
    /// RFC 3339 timestamp.
    pub produced_at: String,
    /// Optional run reference (CAS hash of the SimulationRun, etc.).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    /// Actor who produced this record.
    pub actor: Actor,
    /// Authorization decision.
    pub authz: AuthorizationDecision,
    /// Tool that produced the artifact.
    pub tool: ToolRef,
    /// Inputs (CAS content hashes) keyed by their role.
    pub inputs: Vec<ArtifactRef>,
    /// Parameters (canonical JSON, free form).
    #[serde(default)]
    pub parameters: serde_json::Value,
    /// Environment manifest hash (SDP §17.4).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub environment: Option<String>,
    /// Placement (which backend, which node class).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placement: Option<Placement>,
    /// Outputs (CAS content hashes).
    pub outputs: Vec<ArtifactRef>,
    /// Verification outcomes (e.g. solver `verify()` report).
    #[serde(default)]
    pub verification: serde_json::Value,
    /// Signatures (one per signer; at least the platform signing key).
    #[serde(default)]
    pub signatures: Vec<RecordSignature>,
    /// Hash of the previous record in the chain; `None` for genesis.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prev_record_hash: Option<String>,
    /// Hash of this record (SHA-256 of canonical JSON without this field).
    pub record_hash: String,
}

/// Reference to a tool that produced the record.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolRef {
    /// Plugin id (reverse-DNS, immutable).
    pub plugin_id: String,
    /// Plugin version (semver).
    pub plugin_version: String,
    /// Contract versions the plugin claimed to implement.
    #[serde(default)]
    pub contract_versions: std::collections::BTreeMap<String, String>,
    /// Conformance report hash (proves the plugin passed the contract
    /// conformance suite; SDP §5.8).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conformance_report: Option<String>,
}

/// Reference to an artifact: where it lives and what it is.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArtifactRef {
    /// CAS content hash (SHA-256).
    pub cas_hash: String,
    /// Logical name for the artifact (e.g. "geometry", "result.bundle").
    pub name: String,
    /// Size in bytes; 0 if not applicable.
    #[serde(default)]
    pub size: u64,
    /// MIME or format hint.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
}

/// Compute backend placement (SDP §6).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Placement {
    /// Backend id (e.g. "slurm", "local", "apptainer").
    pub backend: String,
    /// Backend-reported node class (e.g. "cpu-x86_64-epyc-rome").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_class: Option<String>,
}

/// A signature over the record.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecordSignature {
    /// `ed25519` (the only algorithm G1 supports; ADR-0020).
    pub alg: String,
    /// Signing key identifier (resolves to a trusted key).
    pub keyid: String,
    /// Base64-encoded signature bytes.
    pub sig: String,
}

impl ProvenanceRecord {
    /// Build a record with required fields and compute its `record_hash`.
    pub fn new(
        actor: Actor,
        authz: AuthorizationDecision,
        tool: ToolRef,
        inputs: Vec<ArtifactRef>,
        outputs: Vec<ArtifactRef>,
        prev_record_hash: Option<String>,
    ) -> Self {
        let record_id = Uuid::now_v7();
        let produced_at = now_rfc3339();
        let mut r = Self {
            record_id,
            produced_at,
            run_id: None,
            actor,
            authz,
            tool,
            inputs,
            parameters: serde_json::Value::Null,
            environment: None,
            placement: None,
            outputs,
            verification: serde_json::Value::Null,
            signatures: Vec::new(),
            prev_record_hash,
            record_hash: String::new(),
        };
        r.record_hash = r.compute_hash();
        r
    }

    /// Compute the canonical hash of the record, with `record_hash` zeroed.
    pub fn compute_hash(&self) -> String {
        let mut clone = self.clone();
        clone.record_hash = String::new();
        let value = serde_json::to_value(&clone).expect("record encodes");
        canonical_hash(&value)
    }

    /// Verify the record's own `record_hash` matches its content.
    pub fn verify_self(&self) -> RecordResult<()> {
        let computed = self.compute_hash();
        if computed != self.record_hash {
            return Err(RecordError::InvalidHash(format!(
                "stored={} computed={}",
                self.record_hash, computed
            )));
        }
        Ok(())
    }

    /// Validate that every input and output hash looks like a SHA-256 hex
    /// digest. Phase 0: structural check; Phase 2 will resolve hashes
    /// against the CAS.
    pub fn validate_hashes(&self) -> RecordResult<()> {
        for input in &self.inputs {
            if input.cas_hash.len() != 64 || !input.cas_hash.chars().all(|c| c.is_ascii_hexdigit()) {
                return Err(RecordError::InvalidHash(format!(
                    "input {} has non-sha256 hash {}",
                    input.name, input.cas_hash
                )));
            }
        }
        for output in &self.outputs {
            if output.cas_hash.len() != 64 || !output.cas_hash.chars().all(|c| c.is_ascii_hexdigit()) {
                return Err(RecordError::InvalidHash(format!(
                    "output {} has non-sha256 hash {}",
                    output.name, output.cas_hash
                )));
        }
        }
        Ok(())
    }

    /// Convenience: hash a content buffer (e.g. to compute a CAS hash
    /// before constructing the record).
    pub fn hash_bytes(bytes: &[u8]) -> String {
        sha256_hex(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_tool() -> ToolRef {
        ToolRef {
            plugin_id: "com.example.solver".to_string(),
            plugin_version: "1.0.0".to_string(),
            contract_versions: std::collections::BTreeMap::from([(
                "aether.physics.solver".to_string(),
                "1.0.0".to_string(),
            )]),
            conformance_report: Some("cert/abc".to_string()),
        }
    }

    fn sample_actor() -> Actor {
        Actor {
            id: "user:alice".to_string(),
            role: Some("analyst".to_string()),
            plugin_instance: None,
        }
    }

    fn sample_authz() -> AuthorizationDecision {
        AuthorizationDecision {
            decision: "allow".to_string(),
            policy_ref: Some("policy@v1".to_string()),
            rationale: None,
        }
    }

    fn hash64(s: &str) -> String {
        ProvenanceRecord::hash_bytes(s.as_bytes())
    }

    #[test]
    fn hash_self_verifies() {
        let r = ProvenanceRecord::new(
            sample_actor(),
            sample_authz(),
            sample_tool(),
            vec![ArtifactRef {
                cas_hash: hash64("input"),
                name: "geometry".to_string(),
                size: 6,
                format: None,
            }],
            vec![ArtifactRef {
                cas_hash: hash64("output"),
                name: "result".to_string(),
                size: 7,
                format: None,
            }],
            None,
        );
        assert!(r.verify_self().is_ok());
    }

    #[test]
    fn hash_mismatch_detected() {
        let mut r = ProvenanceRecord::new(
            sample_actor(),
            sample_authz(),
            sample_tool(),
            vec![],
            vec![],
            None,
        );
        r.record_hash = "deadbeef".to_string();
        let err = r.verify_self().unwrap_err();
        assert!(matches!(err, RecordError::InvalidHash(_)));
    }

    #[test]
    fn invalid_hashes_rejected() {
        let r = ProvenanceRecord::new(
            sample_actor(),
            sample_authz(),
            sample_tool(),
            vec![ArtifactRef {
                cas_hash: "not-a-hash".to_string(),
                name: "geometry".to_string(),
                size: 0,
                format: None,
            }],
            vec![],
            None,
        );
        let err = r.validate_hashes().unwrap_err();
        assert!(matches!(err, RecordError::InvalidHash(_)));
    }
}
