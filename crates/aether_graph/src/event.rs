// SPDX-License-Identifier: Aether-VPSL-1.0
// SPDX-FileCopyrightText: 2026 The Aether Consortium
//! Graph event model — Phase 0 walking skeleton (b); G1 expansion.
//!
//! The event log is the substrate of the digital thread (SDP §14.1). Every
//! graph mutation is first appended as an event; graph state is a materialized
//! view of the log. The contract for events is in
//! `contracts/graph.model/schema/event-envelope.schema.json`.
//!
//! G1 additions:
//! - typed variants for `node_upserted` and `edge_added` / `edge_removed`
//!   (the Phase 0 `node_created` becomes `node_upserted` with an `is_create` flag)
//! - canonical hash over the JSON representation (sorted keys, no whitespace)
//!   so the chain is reproducible across implementations

#![deny(unsafe_code)]

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::types::{EdgeId, EventId, NodeId, NodeState};

/// All event kinds recognized by the graph engine. Mirrors the
/// `event_kind` vocabulary in `contracts/graph.model/manifest.json` (Phase 0 v0).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    /// A new node was added, or an existing draft node's properties changed.
    /// Distinguished by the `is_create` flag in the payload.
    NodeUpserted,
    /// A node's state changed (e.g., draft -> released).
    NodeStateChanged,
    /// A node was soft-deleted (tombstoned).
    NodeTombstoned,
    /// A typed edge was added.
    EdgeAdded,
    /// A typed edge was removed.
    EdgeRemoved,
    /// A new revision superseded a prior revision.
    RevisionSuperseded,
}

/// A canonical graph event.
///
/// Fields mirror `event-envelope.schema.json`. `prev_event_hash` is the
/// SHA-256 of the previous event's canonical JSON, forming the Merkle
/// chain (SDP §7.6, §14.1).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub event_id: EventId,
    pub occurred_at: String,
    pub kind: EventKind,
    pub actor: String,
    pub authz_decision: String,
    pub payload: serde_json::Value,
    /// SHA-256 of the previous event's canonical JSON; `None` for genesis.
    pub prev_event_hash: Option<String>,
    /// SHA-256 of this event's canonical JSON (chain continuity).
    pub event_hash: String,
}

impl Event {
    /// Compute the canonical hash of this event. The `event_hash` field
    /// is excluded from the input; chain integrity is over the unsigned
    /// payload.
    pub fn compute_hash(&self) -> String {
        let mut clone = self.clone();
        clone.event_hash = String::new();
        let json = crate::aql::canonical_json(&serde_json::to_value(&clone).expect("event encodes"));
        let mut hasher = Sha256::new();
        hasher.update(json.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Convenience: construct an event with computed hash and chain link.
    pub fn new(
        kind: EventKind,
        actor: impl Into<String>,
        authz: impl Into<String>,
        payload: serde_json::Value,
        prev_event_hash: Option<String>,
    ) -> Self {
        let event_id = Uuid::now_v7();
        let occurred_at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let mut e = Self {
            event_id,
            occurred_at,
            kind,
            actor: actor.into(),
            authz_decision: authz.into(),
            payload,
            prev_event_hash,
            event_hash: String::new(),
        };
        e.event_hash = e.compute_hash();
        e
    }
}

/// Payload for a `node_upserted` event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeUpserted {
    pub node_id: NodeId,
    pub type_id: String,
    pub state: NodeState,
    #[serde(default)]
    pub is_create: bool,
    pub properties: serde_json::Value,
    pub actor: String,
    pub authz_decision: String,
}

/// Payload for a `node_state_changed` event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeStateChanged {
    pub node_id: NodeId,
    #[serde(with = "node_state_str")]
    pub from_state: NodeState,
    #[serde(with = "node_state_str")]
    pub to_state: NodeState,
    pub actor: String,
}

mod node_state_str {
    use serde::{Deserialize, Deserializer, Serializer};
    use crate::types::NodeState;
    pub fn serialize<S: Serializer>(s: &NodeState, ser: S) -> Result<S::Ok, S::Error> {
        ser.serialize_str(s.as_str())
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(de: D) -> Result<NodeState, D::Error> {
        let s = String::deserialize(de)?;
        NodeState::parse(&s).ok_or_else(|| serde::de::Error::custom(format!("invalid state {s}")))
    }
}

/// Payload for a `node_tombstoned` event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeTombstoned {
    pub node_id: NodeId,
    pub reason: String,
    pub actor: String,
}

/// Payload for an `edge_added` event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeAdded {
    pub edge_id: EdgeId,
    pub type_id: String,
    pub source: NodeId,
    pub target: NodeId,
    pub properties: serde_json::Value,
    pub actor: String,
}

/// Payload for an `edge_removed` event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeRemoved {
    pub edge_id: EdgeId,
    pub reason: String,
    pub actor: String,
}
