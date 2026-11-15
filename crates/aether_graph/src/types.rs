// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! Core graph types: identity, nodes, edges, properties, type catalog.
//!
//! All identifiers are UUIDv7 (time-ordered, sortable). All node and edge
//! types are registered through the schema catalog (`aether.graph.model`),
//! so the kernel's generic mechanics can validate, revision, and index
//! them without knowing what they mean (SDP §2 I-2, §3.1).

#![deny(unsafe_code)]

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// A node's permanent identity. UUIDv7 so a primary-key index is naturally
/// time-ordered (SDP §3.4).
pub type NodeId = Uuid;

/// A directed edge identity. UUIDv7.
pub type EdgeId = Uuid;

/// An event-log identity. UUIDv7.
pub type EventId = Uuid;

/// A node's lifecycle state (SDP §3.1).
///
/// The four canonical states participate in revisioning and kernel
/// enforcement. Domain types may declare additional *advisory* states
/// (e.g. `proposed`, `running`, `succeeded`) in `allowed_states`; the
/// engine accepts them as opaque strings when they are declared.
///
/// See `contracts/graph.model/schema/node-type.schema.json`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(tag = "state", content = "value")]
pub enum NodeState {
    /// Canonical: editable; not yet visible outside the author's view.
    Draft,
    /// Canonical: frozen; visible to dependents.
    Released,
    /// Canonical: replaced by a successor; retained for history.
    Superseded,
    /// Canonical: soft-deleted; no new edges may target this node, but
    /// references from evidence are preserved (SDP §3.6 invariant 4).
    Archived,
    /// Advisory: domain-specific state declared by the node-type spec.
    /// Examples: `proposed`, `running`, `succeeded`, `failed`,
    /// `preempted`, `accepted`, `reversed`. The engine treats these as
    /// opaque — domain code interprets them.
    Advisory(String),
}

impl NodeState {
    pub fn as_str(&self) -> &str {
        match self {
            NodeState::Draft => "draft",
            NodeState::Released => "released",
            NodeState::Superseded => "superseded",
            NodeState::Archived => "archived",
            NodeState::Advisory(s) => s.as_str(),
        }
    }

    /// Parse a state from a string. Returns `None` if the string is empty.
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "draft" => NodeState::Draft,
            "released" => NodeState::Released,
            "superseded" => NodeState::Superseded,
            "archived" => NodeState::Archived,
            other if !other.is_empty() => NodeState::Advisory(other.to_string()),
            _ => return None,
        })
    }
}

/// Edge cardinality constraints. `None` means "unlimited" (SDP §3.3).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Cardinality {
    pub min_out_per_source: Option<u32>,
    pub max_out_per_source: Option<u32>,
    pub min_in_per_target: Option<u32>,
    pub max_in_per_target: Option<u32>,
}

/// A typed, attributed graph edge. Edges are directed, with cardinality
/// constraints enforced by the graph engine (SDP §3.3, §3.6 invariant 5).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub id: EdgeId,
    pub type_id: String,
    pub source: NodeId,
    pub target: NodeId,
    pub properties: Properties,
    pub created_event: EventId,
}

/// A typed, attributed graph node. The `type_id` is registered in the
/// schema catalog; the `properties` are validated against that catalog.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: NodeId,
    pub type_id: String,
    pub state: NodeState,
    pub properties: Properties,
    pub revisioned: bool,
    /// Set for nodes that are immutable (e.g., `SimulationRun`, `Result`).
    /// Once written, properties cannot change.
    pub immutable: bool,
    pub created_event: EventId,
    pub current_revision: Option<NodeId>,
    pub tombstoned: bool,
}

/// Properties are arbitrary JSON, validated against the node-type schema.
/// We keep an ordered BTreeMap shape for stable iteration, but the wire
/// form is `Value` for compatibility with the contract schemas.
pub type Properties = BTreeMap<String, Value>;

/// Schema-catalog entry for a node type. Loaded from
/// `contracts/graph.model/catalog/nodes/*.json` at startup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeTypeSpec {
    pub type_id: String,
    pub version: String,
    pub revisioned: bool,
    #[serde(default)]
    pub immutable: bool,
    pub allowed_states: Vec<String>,
    /// JSON Schema for the node's `properties` field.
    pub properties_schema: Value,
    /// Initial allowed edges when a node is created. Keys are edge type
    /// ids; values are lists of allowed target node type ids.
    #[serde(default)]
    pub initial_edges: BTreeMap<String, Vec<String>>,
}

/// Schema-catalog entry for an edge type. Loaded from
/// `contracts/graph.model/catalog/edges/*.json` at startup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeTypeSpec {
    pub type_id: String,
    pub version: String,
    pub directed: bool,
    pub source_types: Vec<String>,
    pub target_types: Vec<String>,
    #[serde(default)]
    pub cardinality: Cardinality,
}
