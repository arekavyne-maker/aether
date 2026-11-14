// SPDX-License-Identifier: Aether-VPSL-1.0
// SPDX-FileCopyrightText: 2026 The Aether Consortium
//! Aether provenance subsystem (G1).
//!
//! A **provenance record** is the per-artifact derivation chain
//! (SDP §14.2). Every contract method that produces artifacts emits one.
//! Records are content-addressed and chained: the chain anchors into
//! the audit log; PROV-O export maps records/edges to standard RDF.
//!
//! The graph event log itself is hash-chained via `aether_graph::Event`'s
//! `prev_event_hash` field (SDP §7.6). This module adds a second chain:
//! the **provenance chain**, which is per-record (one record per artifact
//! produced by a contract method) rather than per-graph-mutation.
//!
//! Invariants:
//! - Every contract method that produces artifacts emits a record (I-8).
//! - Records are content-addressed (the record's hash is the canonical
//!   SHA-256 of its canonical JSON).
//! - The chain is append-only; corrections are new records, not edits.

#![deny(unsafe_code)]
#![warn(missing_docs)]

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use uuid::Uuid;

pub mod record;
pub mod chain;
pub mod export;

pub use chain::{ProvenanceChain, ProvenanceChainEntry, ChainError, ChainResult};
pub use record::{
    Actor, AuthorizationDecision, ProvenanceRecord, RecordError, RecordResult,
};
pub use export::{prov_o_record, ProvOError, ProvOResult};

/// Canonical hash of any JSON value: SHA-256 of the canonical JSON form
/// (sorted keys, no whitespace, null-fields omitted). The Aether platform
/// uses this everywhere chain integrity matters.
pub fn canonical_hash(value: &serde_json::Value) -> String {
    let s = aether_graph::aql::canonical_json(value);
    let mut hasher = Sha256::new();
    hasher.update(s.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Convenience: SHA-256 of raw bytes.
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

/// A UUIDv7 for new provenance records.
pub fn new_record_id() -> Uuid {
    Uuid::now_v7()
}

/// RFC 3339 timestamp with millisecond precision.
pub fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

#[derive(Debug, Error)]
pub enum ProvError {
    #[error("serialization: {0}")]
    Serde(#[from] serde_json::Error),
}

pub type ProvResult<T> = Result<T, ProvError>;

#[allow(dead_code)]
fn _ensure_used() {
    let _ = sha256_hex(&[]);
}
