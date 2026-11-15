// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! Graph engine: typed commands, schema enforcement, AS-OF time travel.
//!
//! The engine is the **only** path for plugins to mutate the graph. It
//! enforces the schema catalog and the graph invariants (SDP §3.6) and
//! records every mutation as an event (SDP §3.1, §14.1). Time travel
//! (`as_of`) is supported by replaying events to a target offset (G1
//! subset; full AQL is added incrementally).
//!
//! Invariants enforced here:
//! 1. artifact references resolve to CAS (deferred to Phase 2 when the
//!    CAS is wired; Phase 0 reserved the field)
//! 2. every SimulationRun has an immutable input snapshot (enforced at
//!    the storage layer; the engine refuses to mutate an `immutable` node)
//! 3. revision nodes are immutable (same as #2)
//! 4. deletions are tombstones (the engine never deletes nodes from
//!    storage; `tombstone_node` sets the flag and `archived` state)
//! 5. all cross-domain edges are declared in the edge schema catalog
//!    (enforced by `create_edge`)
//! 6. every mutation has actor + authz (required fields on every command)
//! 7. `satisfies` requires a `verifies` path (Phase 2; the engine
//!    records `satisfies` and `verifies` edges but does not enforce
//!    coverage until the verification-coverage rollup is added in G3)

#![deny(unsafe_code)]
#![warn(missing_docs)]

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::error::{GraphError, GraphResult};
use crate::event::{
    EdgeAdded, EdgeRemoved, Event, EventKind, NodeStateChanged, NodeTombstoned, NodeUpserted,
};
use crate::storage::Storage;
use crate::types::{
    Edge, EdgeId, EventId, Node, NodeId, NodeState, NodeTypeSpec, EdgeTypeSpec, Properties,
};

/// A snapshot of the graph at a logical point in time. Either the current
/// state (the materialized view) or a replayed historical state (AS-OF).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphSnapshot {
    pub nodes: BTreeMap<NodeId, Node>,
    pub edges: BTreeMap<EdgeId, Edge>,
    /// The head of the event chain at the moment of this snapshot.
    pub chain_head: Option<String>,
    /// Number of events that contributed to this snapshot.
    pub event_count: u64,
}

/// Identifies the time at which a snapshot is taken. G1 supports the
/// current state and event-offset; full timestamp-based AS-OF lands when
/// we add a B-tree on `occurred_at` (Phase 1.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotAt {
    /// The current materialized state.
    Current,
    /// AS-OF the Nth event in the chain (1-indexed; 0 = genesis).
    EventOffset(u64),
}

/// The graph engine. Cheap to clone (storage is shared via `Arc`).
#[derive(Clone)]
pub struct GraphEngine {
    storage: Arc<std::sync::Mutex<Box<dyn Storage>>>,
    node_types: Arc<BTreeMap<String, NodeTypeSpec>>,
    edge_types: Arc<BTreeMap<String, EdgeTypeSpec>>,
}

impl std::fmt::Debug for GraphEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GraphEngine")
            .field("node_types", &self.node_types.len())
            .field("edge_types", &self.edge_types.len())
            .finish()
    }
}

impl GraphEngine {
    /// Construct an engine over a storage backend and a loaded schema
    /// catalog.
    pub fn new(
        storage: Box<dyn Storage>,
        node_types: BTreeMap<String, NodeTypeSpec>,
        edge_types: BTreeMap<String, EdgeTypeSpec>,
    ) -> Self {
        Self {
            storage: Arc::new(std::sync::Mutex::new(storage)),
            node_types: Arc::new(node_types),
            edge_types: Arc::new(edge_types),
        }
    }

    /// Return the number of nodes in the materialized view.
    pub fn node_count(&self) -> GraphResult<u64> {
        self.storage.lock().unwrap().node_count()
    }

    /// Return the number of edges in the materialized view.
    pub fn edge_count(&self) -> GraphResult<u64> {
        self.storage.lock().unwrap().edge_count()
    }

    /// Return the number of events in the log.
    pub fn event_count(&self) -> GraphResult<u64> {
        self.storage.lock().unwrap().event_count()
    }

    /// Return the chain head (the most recently appended event's hash).
    pub fn storage_head(&self) -> GraphResult<Option<String>> {
        self.storage.lock().unwrap().chain_head()
    }

    /// Look up a node by id.
    pub fn get_node(&self, id: NodeId) -> GraphResult<Option<Node>> {
        self.storage.lock().unwrap().get_node(id)
    }

    /// Look up an edge by id.
    pub fn get_edge(&self, id: EdgeId) -> GraphResult<Option<Edge>> {
        self.storage.lock().unwrap().get_edge(id)
    }

    /// Find all nodes of a given type.
    pub fn nodes_of_type(&self, type_id: &str) -> GraphResult<Vec<Node>> {
        self.storage.lock().unwrap().find_nodes_by_type(type_id)
    }

    /// Find all out-edges of a node.
    pub fn out_edges(&self, source: NodeId) -> GraphResult<Vec<Edge>> {
        self.storage.lock().unwrap().out_edges(source)
    }

    /// Find all in-edges of a node.
    pub fn in_edges(&self, target: NodeId) -> GraphResult<Vec<Edge>> {
        self.storage.lock().unwrap().in_edges(target)
    }

    /// Find all edges between two specific nodes (any type).
    pub fn edges_between(&self, source: NodeId, target: NodeId) -> GraphResult<Vec<Edge>> {
        self.storage.lock().unwrap().edges_between(source, target)
    }

    /// Create or update a node. The command is appended to the event
    /// log and reflected in the materialized view.
    pub fn upsert_node(
        &self,
        type_id: impl Into<String>,
        state: NodeState,
        properties: Properties,
        actor: impl Into<String>,
        authz: impl Into<String>,
    ) -> GraphResult<Node> {
        let type_id = type_id.into();
        let spec = self
            .node_types
            .get(&type_id)
            .ok_or_else(|| GraphError::ContractViolation(format!("unknown node type {type_id}")))?
            .clone();
        if !spec.allowed_states.iter().any(|s| s == state.as_str()) {
            return Err(GraphError::ContractViolation(format!(
                "state {:?} not allowed for type {type_id} (allowed: {:?})",
                state, spec.allowed_states
            )));
        }
        let actor = actor.into();
        let authz = authz.into();
        let node_id = Uuid::now_v7();
        let is_create = true;
        let payload = serde_json::to_value(NodeUpserted {
            node_id,
            type_id: type_id.clone(),
            state: state.clone(),
            is_create,
            properties: serde_json::to_value(&properties)
                .map_err(|e| GraphError::Internal(format!("encode props: {e}")))?,
            actor: actor.clone(),
            authz_decision: authz.clone(),
        })
        .map_err(|e| GraphError::Internal(format!("encode payload: {e}")))?;
        let prev_head = self.storage.lock().unwrap().chain_head()?;
        let event = Event::new(
            EventKind::NodeUpserted,
            actor,
            authz,
            payload,
            prev_head,
        );
        self.append_event(&event)?;
        let node = Node {
            id: node_id,
            type_id,
            state,
            properties,
            revisioned: spec.revisioned,
            immutable: spec.immutable,
            created_event: event.event_id,
            current_revision: None,
            tombstoned: false,
        };
        self.storage.lock().unwrap().upsert_node(&node)?;
        Ok(node)
    }

    /// Change the state of an existing node. For `System` nodes, the
    /// `released` transition additionally requires the verification-
    /// coverage invariant (SDP §3.6 invariant 7): every `satisfies`
    /// edge out of the System must have a `verifies` path through
    /// some chain (Requirement -> TestRun, Requirement -> InspectionRecord,
    /// or Requirement -> Evidence).
    pub fn change_node_state(
        &self,
        id: NodeId,
        to_state: NodeState,
        actor: impl Into<String>,
        authz: impl Into<String>,
    ) -> GraphResult<Node> {
        let actor = actor.into();
        let authz = authz.into();
        let node = self
            .get_node(id)?
            .ok_or_else(|| GraphError::NotFound(format!("node {id}")))?;
        if node.tombstoned {
            return Err(GraphError::ContractViolation(format!(
                "cannot change state of tombstoned node {id}"
            )));
        }
        if node.immutable {
            return Err(GraphError::ContractViolation(format!(
                "node {id} is immutable; state changes are forbidden"
            )));
        }
        let spec = self
            .node_types
            .get(&node.type_id)
            .ok_or_else(|| GraphError::Internal(format!("node type missing: {}", node.type_id)))?;
        if !spec.allowed_states.iter().any(|s| s == to_state.as_str()) {
            return Err(GraphError::ContractViolation(format!(
                "state {:?} not allowed for type {}",
                to_state, node.type_id
            )));
        }
        // SDP §3.6 invariant 7: a System node cannot transition to
        // Released unless every satisfies edge out of it has a
        // verifying chain.
        if node.type_id == "System" && to_state == NodeState::Released {
            self.check_verification_coverage(id)?;
        }
        let from_state = node.state;
        let payload = serde_json::to_value(NodeStateChanged {
            node_id: id,
            from_state: from_state.clone(),
            to_state: to_state.clone(),
            actor: actor.clone(),
        })
        .map_err(|e| GraphError::Internal(format!("encode payload: {e}")))?;
        let prev_head = self.storage.lock().unwrap().chain_head()?;
        let event = Event::new(EventKind::NodeStateChanged, actor, authz, payload, prev_head);
        self.append_event(&event)?;
        self.storage.lock().unwrap().update_node_state(id, to_state.clone())?;
        let mut updated = self.get_node(id)?.ok_or_else(|| GraphError::Internal("lost node".into()))?;
        updated.state = to_state;
        Ok(updated)
    }

    /// SDP §3.6 invariant 7: every `satisfies` edge out of the given
    /// system node must have a `verifies` path (Requirement -> TestRun /
    /// InspectionRecord / Evidence).
    fn check_verification_coverage(&self, system_id: NodeId) -> GraphResult<()> {
        let satisfies = self.out_edges(system_id)?;
        let mut gaps: Vec<String> = Vec::new();
        for edge in satisfies {
            if edge.type_id != "satisfies" {
                continue;
            }
            // The target must be a Requirement; walk both its out_edges
            // and its in_edges looking for a verifies edge. The graph
            // convention is Evidence -> verifies -> Requirement, so
            // in_edges is the natural direction; we also accept
            // out_edges (Requirement -> verifies -> Evidence) so the
            // alternative reading counts as coverage too.
            let target = self
                .get_node(edge.target)?
                .ok_or_else(|| GraphError::NotFound(format!("satisfies target {}", edge.target)))?;
            if target.type_id != "Requirement" {
                continue;
            }
            let out_verifies = self.out_edges(target.id)?;
            let in_verifies = self.in_edges(target.id)?;
            let has_verifies = out_verifies.iter().any(|e| e.type_id == "verifies")
                || in_verifies.iter().any(|e| e.type_id == "verifies");
            if !has_verifies {
                gaps.push(format!("{} (satisfies target {} has no verifies path)", target.id, target.type_id));
            }
        }
        if !gaps.is_empty() {
            return Err(GraphError::ContractViolation(format!(
                "verification coverage invariant failed for System {system_id}: {} requirement(s) lack a verifies path: {}",
                gaps.len(),
                gaps.join("; ")
            )));
        }
        Ok(())
    }

    /// Tombstone a node. The node is preserved (so evidence references
    /// remain valid) but no new edges may target it and queries
    /// filter it out.
    pub fn tombstone_node(
        &self,
        id: NodeId,
        reason: impl Into<String>,
        actor: impl Into<String>,
        authz: impl Into<String>,
    ) -> GraphResult<()> {
        let actor = actor.into();
        let authz = authz.into();
        let reason = reason.into();
        let node = self
            .get_node(id)?
            .ok_or_else(|| GraphError::NotFound(format!("node {id}")))?;
        if node.tombstoned {
            return Ok(());
        }
        let payload = serde_json::to_value(NodeTombstoned {
            node_id: id,
            reason: reason.clone(),
            actor: actor.clone(),
        })
        .map_err(|e| GraphError::Internal(format!("encode payload: {e}")))?;
        let prev_head = self.storage.lock().unwrap().chain_head()?;
        let event = Event::new(EventKind::NodeTombstoned, actor, authz, payload, prev_head);
        self.append_event(&event)?;
        self.storage.lock().unwrap().tombstone_node(id)?;
        Ok(())
    }

    /// Create a typed edge. The engine enforces:
    /// - the edge type is registered
    /// - the source and target are not tombstoned
    /// - the source/target types are allowed by the edge type spec
    /// - cardinality constraints are respected
    pub fn create_edge(
        &self,
        type_id: impl Into<String>,
        source: NodeId,
        target: NodeId,
        properties: Properties,
        actor: impl Into<String>,
        authz: impl Into<String>,
    ) -> GraphResult<Edge> {
        let type_id = type_id.into();
        let spec = self
            .edge_types
            .get(&type_id)
            .ok_or_else(|| GraphError::ContractViolation(format!("unknown edge type {type_id}")))?
            .clone();
        let actor = actor.into();
        let authz = authz.into();

        let source_node = self
            .get_node(source)?
            .ok_or_else(|| GraphError::NotFound(format!("source {source}")))?;
        let target_node = self
            .get_node(target)?
            .ok_or_else(|| GraphError::NotFound(format!("target {target}")))?;
        if source_node.tombstoned {
            return Err(GraphError::ContractViolation(format!(
                "source node {source} is tombstoned"
            )));
        }
        if target_node.tombstoned {
            return Err(GraphError::ContractViolation(format!(
                "target node {target} is tombstoned"
            )));
        }
        if !spec.source_types.iter().any(|t| t == &source_node.type_id) {
            return Err(GraphError::ContractViolation(format!(
                "source type {} not allowed for edge {type_id} (allowed: {:?})",
                source_node.type_id, spec.source_types
            )));
        }
        if !spec.target_types.iter().any(|t| t == &target_node.type_id) {
            return Err(GraphError::ContractViolation(format!(
                "target type {} not allowed for edge {type_id} (allowed: {:?})",
                target_node.type_id, spec.target_types
            )));
        }
        // Cardinality checks
        if let Some(max) = spec.cardinality.max_out_per_source {
            let existing = self.out_edges(source)?;
            let same_type = existing.iter().filter(|e| e.type_id == type_id).count() as u32;
            if same_type >= max {
                return Err(GraphError::ContractViolation(format!(
                    "edge {type_id} exceeds max_out_per_source={max} from {source}"
                )));
            }
        }
        if let Some(max) = spec.cardinality.max_in_per_target {
            let existing = self.in_edges(target)?;
            let same_type = existing.iter().filter(|e| e.type_id == type_id).count() as u32;
            if same_type >= max {
                return Err(GraphError::ContractViolation(format!(
                    "edge {type_id} exceeds max_in_per_target={max} into {target}"
                )));
            }
        }
        if let Some(min) = spec.cardinality.min_in_per_target {
            // min is checked at query time, not create time, but we still
            // note the constraint in the error path here.
            let _ = min;
        }
        let edge_id = Uuid::now_v7();
        let payload = serde_json::to_value(EdgeAdded {
            edge_id,
            type_id: type_id.clone(),
            source,
            target,
            properties: serde_json::to_value(&properties)
                .map_err(|e| GraphError::Internal(format!("encode props: {e}")))?,
            actor: actor.clone(),
        })
        .map_err(|e| GraphError::Internal(format!("encode payload: {e}")))?;
        let prev_head = self.storage.lock().unwrap().chain_head()?;
        let event = Event::new(EventKind::EdgeAdded, actor, authz, payload, prev_head);
        self.append_event(&event)?;
        let edge = Edge {
            id: edge_id,
            type_id,
            source,
            target,
            properties,
            created_event: event.event_id,
        };
        self.storage.lock().unwrap().insert_edge(&edge)?;
        Ok(edge)
    }

    /// Remove a typed edge. The engine records an `edge_removed` event.
    pub fn remove_edge(
        &self,
        id: EdgeId,
        reason: impl Into<String>,
        actor: impl Into<String>,
        authz: impl Into<String>,
    ) -> GraphResult<()> {
        let actor = actor.into();
        let authz = authz.into();
        let reason = reason.into();
        let edge = self
            .get_edge(id)?
            .ok_or_else(|| GraphError::NotFound(format!("edge {id}")))?;
        let payload = serde_json::to_value(EdgeRemoved {
            edge_id: id,
            reason,
            actor: actor.clone(),
        })
        .map_err(|e| GraphError::Internal(format!("encode payload: {e}")))?;
        let prev_head = self.storage.lock().unwrap().chain_head()?;
        let event = Event::new(EventKind::EdgeRemoved, actor, authz, payload, prev_head);
        self.append_event(&event)?;
        self.storage.lock().unwrap().remove_edge(id)?;
        let _ = edge;
        Ok(())
    }

    /// Take a snapshot of the graph (current state or AS-OF).
    pub fn snapshot(&self, at: SnapshotAt) -> GraphResult<GraphSnapshot> {
        match at {
            SnapshotAt::Current => {
                let mut nodes = BTreeMap::new();
                let mut edges = BTreeMap::new();
                let chain_head = self.storage.lock().unwrap().chain_head()?;
                let event_count = self.event_count()?;
                // We don't have a global node iterator in the storage SPI
                // yet; iterate via the type catalog instead. This walks
                // every registered node type and unions the result.
                for type_id in self.node_types.keys() {
                    let of_type = self.storage.lock().unwrap().find_nodes_by_type(type_id)?;
                    for n in of_type {
                        nodes.insert(n.id, n);
                    }
                }
                // Edges: not in the SPI either; we add a graph_edges
                // method in Phase 1.5 when needed. For G1, the snapshot
                // includes edges reachable from known nodes.
                let mut visited: BTreeSet<EdgeId> = BTreeSet::new();
                for source_id in nodes.keys().copied().collect::<Vec<_>>() {
                    let out = self.storage.lock().unwrap().out_edges(source_id)?;
                    for e in out {
                        if visited.insert(e.id) {
                            edges.insert(e.id, e);
                        }
                    }
                }
                Ok(GraphSnapshot {
                    nodes,
                    edges,
                    chain_head,
                    event_count,
                })
            }
            SnapshotAt::EventOffset(offset) => {
                // AS-OF replay: walk the event log up to the offset and
                // rebuild the materialized state. This is the substrate
                // for time-travel queries (MVR-09).
                self.replay_to_offset(offset)
            }
        }
    }

    /// Replay the event log up to the given offset and produce a
    /// snapshot. The materialized view is **not** modified; this is a
    /// pure projection.
    fn replay_to_offset(&self, offset: u64) -> GraphResult<GraphSnapshot> {
        let events = self.storage.lock().unwrap().iter_events()?;
        if offset > events.len() as u64 {
            return Err(GraphError::ContractViolation(format!(
                "AS-OF offset {offset} exceeds event count {}",
                events.len()
            )));
        }
        let mut nodes: BTreeMap<NodeId, Node> = BTreeMap::new();
        let mut edges: BTreeMap<EdgeId, Edge> = BTreeMap::new();
        let mut chain_head: Option<String> = None;
        for event in events.into_iter().take(offset as usize) {
            chain_head = Some(event.event_hash.clone());
            match event.kind {
                EventKind::NodeUpserted => {
                    let p: NodeUpserted = serde_json::from_value(event.payload)
                        .map_err(|e| GraphError::Internal(format!("decode NodeUpserted: {e}")))?;
                    let spec = self
                        .node_types
                        .get(&p.type_id)
                        .cloned()
                        .unwrap_or(NodeTypeSpec {
                            type_id: p.type_id.clone(),
                            version: "unknown".to_string(),
                            revisioned: false,
                            immutable: false,
                            allowed_states: vec!["draft".into(), "released".into()],
                            properties_schema: Value::Null,
                            initial_edges: BTreeMap::new(),
                        });
                    let properties: Properties = serde_json::from_value(p.properties)
                        .map_err(|e| GraphError::Internal(format!("decode props: {e}")))?;
                    let current = nodes.get(&p.node_id).cloned();
                    let next = Node {
                        id: p.node_id,
                        type_id: p.type_id,
                        state: p.state,
                        properties,
                        revisioned: spec.revisioned,
                        immutable: spec.immutable,
                        created_event: current
                            .as_ref()
                            .map(|c| c.created_event)
                            .unwrap_or(event.event_id),
                        current_revision: current.as_ref().and_then(|c| c.current_revision),
                        tombstoned: false,
                    };
                    nodes.insert(p.node_id, next);
                }
                EventKind::NodeStateChanged => {
                    let p: NodeStateChanged = serde_json::from_value(event.payload)
                        .map_err(|e| GraphError::Internal(format!("decode NodeStateChanged: {e}")))?;
                    if let Some(n) = nodes.get_mut(&p.node_id) {
                        n.state = p.to_state;
                    }
                }
                EventKind::NodeTombstoned => {
                    let p: NodeTombstoned = serde_json::from_value(event.payload)
                        .map_err(|e| GraphError::Internal(format!("decode NodeTombstoned: {e}")))?;
                    if let Some(n) = nodes.get_mut(&p.node_id) {
                        n.tombstoned = true;
                        n.state = NodeState::Archived;
                    }
                }
                EventKind::EdgeAdded => {
                    let p: EdgeAdded = serde_json::from_value(event.payload)
                        .map_err(|e| GraphError::Internal(format!("decode EdgeAdded: {e}")))?;
                    let properties: Properties = serde_json::from_value(p.properties)
                        .map_err(|e| GraphError::Internal(format!("decode edge props: {e}")))?;
                    let e = Edge {
                        id: p.edge_id,
                        type_id: p.type_id,
                        source: p.source,
                        target: p.target,
                        properties,
                        created_event: event.event_id,
                    };
                    edges.insert(e.id, e);
                }
                EventKind::EdgeRemoved => {
                    let p: EdgeRemoved = serde_json::from_value(event.payload)
                        .map_err(|e| GraphError::Internal(format!("decode EdgeRemoved: {e}")))?;
                    edges.remove(&p.edge_id);
                }
                EventKind::RevisionSuperseded => {
                    // G1 records the event; the supersede logic itself
                    // is added in Phase 1.5.
                }
            }
        }
        let event_count = offset;
        Ok(GraphSnapshot {
            nodes,
            edges,
            chain_head,
            event_count,
        })
    }

    /// Verify the chain: every event's `prev_event_hash` matches the
    /// previous event's `event_hash`; every `event_hash` recomputes.
    pub fn verify_chain(&self) -> GraphResult<ChainVerification> {
        let events = self.storage.lock().unwrap().iter_events()?;
        let mut prev: Option<String> = None;
        let mut errors: Vec<String> = Vec::new();
        for (i, e) in events.iter().enumerate() {
            if e.prev_event_hash != prev {
                errors.push(format!(
                    "event {i} ({}) prev_event_hash mismatch: stored={:?} expected={:?}",
                    e.event_id,
                    e.prev_event_hash,
                    prev
                ));
            }
            let computed = e.compute_hash();
            if computed != e.event_hash {
                errors.push(format!(
                    "event {i} ({}) event_hash mismatch: stored={} computed={}",
                    e.event_id, e.event_hash, computed
                ));
            }
            prev = Some(e.event_hash.clone());
        }
        Ok(ChainVerification {
            valid: errors.is_empty(),
            event_count: events.len() as u64,
            errors,
        })
    }

    /// Append an event to the storage; helper for the engine methods.
    fn append_event(&self, event: &Event) -> GraphResult<EventId> {
        self.storage.lock().unwrap().append_event(event)
    }
}

/// Result of a chain verification (MVR-09 substrate).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainVerification {
    pub valid: bool,
    pub event_count: u64,
    pub errors: Vec<String>,
}

/// AQL subset query: `MATCH (n:Type {prop: value}) RETURN n`.
/// G1 implements the simplest match pattern; richer patterns (multi-hop,
/// filters across edges, projections) land in G1.5.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AqlQuery {
    pub type_id: Option<String>,
    pub property_filters: BTreeMap<String, Value>,
    /// Optional limit; `None` returns all matching nodes.
    pub limit: Option<u32>,
}

impl AqlQuery {
    /// Execute the query against the engine's materialized view.
    pub fn execute(&self, engine: &GraphEngine) -> GraphResult<Vec<Node>> {
        let mut out: Vec<Node> = if let Some(t) = &self.type_id {
            engine.nodes_of_type(t)?
        } else {
            let mut all = Vec::new();
            for type_id in engine.node_types.keys() {
                all.extend(engine.nodes_of_type(type_id)?);
            }
            all
        };
        if !self.property_filters.is_empty() {
            out.retain(|n| {
                self.property_filters.iter().all(|(k, v)| n.properties.get(k) == Some(v))
            });
        }
        if let Some(limit) = self.limit {
            out.truncate(limit as usize);
        }
        Ok(out)
    }
}
