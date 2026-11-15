// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! Provenance chain — append-only sequence of `ProvenanceRecord`s
//! (SDP §14.2).
//!
//! The chain is the substrate of the **per-artifact** digital thread:
//! - one chain per project (or per workflow run, in G2+)
//! - records link to each other via `prev_record_hash`
//! - chain continuity is verified by walking the records and checking
//!   each `prev_record_hash` against the previous record's
//!   `record_hash`, plus verifying each record's self-hash.
//!
//! This is distinct from the **graph event chain** (`aether_graph`),
//! which records graph mutations. The provenance chain records
//! artifact production (e.g. a `Result` was produced by `SimulationRun`
//! with these inputs and these parameters).

#![deny(unsafe_code)]

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::record::ProvenanceRecord;

#[derive(Debug, Error)]
pub enum ChainError {
    #[error("chain is empty")]
    Empty,
    #[error("chain broken at record {idx}: prev_record_hash={prev:?} expected={expected:?}")]
    Broken {
        idx: usize,
        prev: Option<String>,
        expected: Option<String>,
    },
    #[error("record {0} failed self-verification: {1}")]
    InvalidRecord(uuid::Uuid, String),
    #[error("duplicate record id: {0}")]
    Duplicate(uuid::Uuid),
}

pub type ChainResult<T> = Result<T, ChainError>;

/// An entry in a `ProvenanceChain`. Phase 0 stores records in memory;
/// Phase 2 persists to a per-project store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceChainEntry {
    pub index: u64,
    pub record: ProvenanceRecord,
}

/// An append-only provenance chain.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProvenanceChain {
    pub entries: Vec<ProvenanceChainEntry>,
}

impl ProvenanceChain {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Append a record to the chain. The record's `prev_record_hash` is
    /// overwritten with the current chain head; the record is then
    /// re-hashed so the chain link is valid.
    pub fn append(&mut self, mut record: ProvenanceRecord) -> ChainResult<()> {
        // Refuse duplicates.
        if self.entries.iter().any(|e| e.record.record_id == record.record_id) {
            return Err(ChainError::Duplicate(record.record_id));
        }
        let prev_hash = self.entries.last().map(|e| e.record.record_hash.clone());
        record.prev_record_hash = prev_hash;
        record.record_hash = record.compute_hash();
        record.verify_self().map_err(|e| {
            ChainError::InvalidRecord(record.record_id, e.to_string())
        })?;
        let index = self.entries.len() as u64;
        self.entries.push(ProvenanceChainEntry { index, record });
        Ok(())
    }

    /// Verify the chain: every record self-verifies, every `prev_record_hash`
    /// matches the previous record's `record_hash`.
    pub fn verify(&self) -> ChainResult<()> {
        if self.entries.is_empty() {
            return Err(ChainError::Empty);
        }
        for (i, entry) in self.entries.iter().enumerate() {
            entry.record.verify_self().map_err(|e| {
                ChainError::InvalidRecord(entry.record.record_id, e.to_string())
            })?;
            let expected_prev = if i == 0 {
                None
            } else {
                Some(self.entries[i - 1].record.record_hash.clone())
            };
            if entry.record.prev_record_hash != expected_prev {
                return Err(ChainError::Broken {
                    idx: i,
                    prev: entry.record.prev_record_hash.clone(),
                    expected: expected_prev,
                });
            }
        }
        Ok(())
    }

    /// Find a record by id.
    pub fn find(&self, record_id: uuid::Uuid) -> Option<&ProvenanceRecord> {
        self.entries
            .iter()
            .find(|e| e.record.record_id == record_id)
            .map(|e| &e.record)
    }

    /// Return the chain head (most recently appended record's hash).
    pub fn head(&self) -> Option<&str> {
        self.entries.last().map(|e| e.record.record_hash.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::record::{Actor, ArtifactRef, AuthorizationDecision, ToolRef};

    fn sample_record(prev: Option<String>) -> ProvenanceRecord {
        let mut r = ProvenanceRecord::new(
            Actor {
                id: "user:test".to_string(),
                role: None,
                plugin_instance: None,
            },
            AuthorizationDecision {
                decision: "allow".to_string(),
                policy_ref: None,
                rationale: None,
            },
            ToolRef {
                plugin_id: "org.test.plugin".to_string(),
                plugin_version: "0.1.0".to_string(),
                contract_versions: Default::default(),
                conformance_report: None,
            },
            vec![],
            vec![ArtifactRef {
                cas_hash: "0".repeat(64),
                name: "out".to_string(),
                size: 0,
                format: None,
            }],
            prev,
        );
        r.record_hash = r.compute_hash();
        r
    }

    #[test]
    fn chain_grows_and_verifies() {
        let mut chain = ProvenanceChain::new();
        for _ in 0..3 {
            chain.append(sample_record(chain.head().map(|s| s.to_string()))).unwrap();
        }
        assert_eq!(chain.len(), 3);
        assert!(chain.verify().is_ok());
    }

    #[test]
    fn broken_chain_detected() {
        let mut chain = ProvenanceChain::new();
        chain.append(sample_record(None)).unwrap();
        chain
            .append(sample_record(chain.head().map(|s| s.to_string())))
            .unwrap();
        chain
            .append(sample_record(chain.head().map(|s| s.to_string())))
            .unwrap();
        // Tamper: change the second record's *record_hash* field directly
        // (not its content). The third record's prev_record_hash will no
        // longer match.
        chain.entries[1].record.record_hash = "deadbeef".to_string();
        // Now the second record's self-verify will fail; we expect InvalidRecord.
        let err = chain.verify().unwrap_err();
        assert!(
            matches!(err, ChainError::InvalidRecord(_, _)),
            "expected InvalidRecord, got: {err:?}"
        );

        // A second test: tamper with the third record's prev_record_hash
        // but leave the second record's record_hash intact; the third
        // record's self-hash will still verify (its prev is part of the
        // hash, but the stored prev has changed). The chain link will
        // detect the mismatch.
        let mut chain = ProvenanceChain::new();
        chain.append(sample_record(None)).unwrap();
        chain
            .append(sample_record(chain.head().map(|s| s.to_string())))
            .unwrap();
        let second_hash = chain.entries[1].record.record_hash.clone();
        // Construct a third record with valid prev pointing at the second.
        chain
            .append(sample_record(Some(second_hash.clone())))
            .unwrap();
        // Now corrupt the third record's prev_record_hash.
        chain.entries[2].record.prev_record_hash = Some("corrupted".to_string());
        chain.entries[2].record.record_hash = chain.entries[2].record.compute_hash();
        let err = chain.verify().unwrap_err();
        assert!(
            matches!(err, ChainError::Broken { .. }),
            "expected Broken, got: {err:?}"
        );
    }

    #[test]
    fn duplicate_record_rejected() {
        let mut chain = ProvenanceChain::new();
        let r = sample_record(None);
        chain.append(r.clone()).unwrap();
        let err = chain.append(r);
        assert!(matches!(err, Err(ChainError::Duplicate(_))));
    }
}
