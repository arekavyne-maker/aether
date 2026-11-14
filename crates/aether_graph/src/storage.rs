// SPDX-License-Identifier: Aether-VPSL-1.0
// SPDX-FileCopyrightText: 2026 The Aether Consortium
//! Storage SPI and the SQLite implementation.
//!
//! G1 introduces a single storage trait that the kernel uses for every
//! read and write. The SQLite backend is the default (workstation); a
//! PostgreSQL backend lands in Phase 3 (GA-1) for team mode. Both must
//! honor the same chain semantics (Merkle hash, append-only event log,
//! materialized current state).
//!
//! **Plugins never issue raw SQL.** All graph access flows through the
//! engine, which calls this trait (SDP §2 I-3, §3.1).

#![deny(unsafe_code)]

use std::collections::BTreeMap;
use std::path::Path;

use rusqlite::{Connection, OptionalExtension, params};
use serde_json::Value;
use uuid::Uuid;

use crate::error::{GraphError, GraphResult};
use crate::event::{Event, EventKind};
use crate::types::{
    Cardinality, Edge, EdgeId, EdgeTypeSpec, EventId, Node, NodeId, NodeState, NodeTypeSpec,
    Properties,
};

/// Open mode for SQLite-backed storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenMode {
    /// Read-only access.
    ReadOnly,
    /// Read-write; creates the file if missing.
    ReadWrite,
}

/// A row returned by an AQL-style pattern query (G1 subset).
#[derive(Debug, Clone)]
pub struct MappedRow {
    pub columns: BTreeMap<String, Value>,
}

/// The graph storage SPI. Every kernel read and write goes through this
/// trait. Plugins must not call it directly; the engine mediates.
pub trait Storage: Send {
    /// Append an event to the log; returns the persisted event's id.
    fn append_event(&mut self, event: &Event) -> GraphResult<EventId>;

    /// Return the chain head (the most recently appended event's hash).
    fn chain_head(&self) -> GraphResult<Option<String>>;

    /// Return the most recent event, if any.
    fn last_event(&self) -> GraphResult<Option<Event>>;

    /// Iterate the entire event log in append order.
    fn iter_events(&self) -> GraphResult<Vec<Event>>;

    /// Insert or update a node in the materialized view.
    fn upsert_node(&mut self, node: &Node) -> GraphResult<()>;

    /// Update only the state of a node.
    fn update_node_state(&mut self, id: NodeId, state: NodeState) -> GraphResult<()>;

    /// Mark a node as tombstoned.
    fn tombstone_node(&mut self, id: NodeId) -> GraphResult<()>;

    /// Look up a node by id.
    fn get_node(&self, id: NodeId) -> GraphResult<Option<Node>>;

    /// Insert an edge.
    fn insert_edge(&mut self, edge: &Edge) -> GraphResult<()>;

    /// Remove an edge.
    fn remove_edge(&mut self, id: EdgeId) -> GraphResult<()>;

    /// Look up an edge by id.
    fn get_edge(&self, id: EdgeId) -> GraphResult<Option<Edge>>;

    /// Find all nodes of a given type.
    fn find_nodes_by_type(&self, type_id: &str) -> GraphResult<Vec<Node>>;

    /// Find all edges leaving a given node (out-edges).
    fn out_edges(&self, source: NodeId) -> GraphResult<Vec<Edge>>;

    /// Find all edges entering a given node (in-edges).
    fn in_edges(&self, target: NodeId) -> GraphResult<Vec<Edge>>;

    /// Find edges between two specific nodes (any type).
    fn edges_between(&self, source: NodeId, target: NodeId) -> GraphResult<Vec<Edge>>;

    /// Return the count of nodes.
    fn node_count(&self) -> GraphResult<u64>;

    /// Return the count of edges.
    fn edge_count(&self) -> GraphResult<u64>;

    /// Return the count of events.
    fn event_count(&self) -> GraphResult<u64>;
}

/// Marker trait to keep the dispatch trait-object-free at the engine layer.
pub trait StorageBackend: Storage {}

/// SQLite-backed storage. This is the Phase 0/G1 default.
pub struct SqliteStorage {
    conn: Connection,
}

impl SqliteStorage {
    pub fn open(path: &Path, mode: OpenMode) -> GraphResult<Self> {
        let conn = match mode {
            OpenMode::ReadOnly => Connection::open_with_flags(
                path,
                rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY
                    | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
            )
            .map_err(|e| GraphError::ResourceUnavailable(format!("open {}: {e}", path.display())))?,
            OpenMode::ReadWrite => Connection::open(path)
                .map_err(|e| GraphError::ResourceUnavailable(format!("open {}: {e}", path.display())))?,
        };
        let mut s = Self { conn };
        s.apply_schema()?;
        Ok(s)
    }

    pub fn open_in_memory() -> GraphResult<Self> {
        let conn =
            Connection::open_in_memory().map_err(|e| GraphError::ResourceUnavailable(e.to_string()))?;
        let mut s = Self { conn };
        s.apply_schema()?;
        Ok(s)
    }

    fn apply_schema(&mut self) -> GraphResult<()> {
        self.conn
            .execute_batch(
                r#"
                PRAGMA journal_mode = WAL;
                PRAGMA synchronous = NORMAL;
                PRAGMA foreign_keys = ON;

                CREATE TABLE IF NOT EXISTS events (
                    event_id        TEXT PRIMARY KEY,
                    occurred_at     TEXT NOT NULL,
                    kind            TEXT NOT NULL,
                    actor           TEXT NOT NULL,
                    authz_decision  TEXT NOT NULL,
                    payload         BLOB NOT NULL,
                    prev_event_hash TEXT,
                    event_hash      TEXT NOT NULL
                );
                CREATE INDEX IF NOT EXISTS events_occurred_at_idx ON events(occurred_at);

                CREATE TABLE IF NOT EXISTS nodes (
                    node_id            TEXT PRIMARY KEY,
                    type_id            TEXT NOT NULL,
                    state              TEXT NOT NULL,
                    properties         BLOB NOT NULL,
                    revisioned         INTEGER NOT NULL,
                    immutable          INTEGER NOT NULL,
                    created_event      TEXT NOT NULL,
                    current_revision   TEXT,
                    tombstoned         INTEGER NOT NULL DEFAULT 0
                );
                CREATE INDEX IF NOT EXISTS nodes_type_state_idx ON nodes(type_id, state);
                CREATE INDEX IF NOT EXISTS nodes_current_revision_idx ON nodes(current_revision);

                CREATE TABLE IF NOT EXISTS edges (
                    edge_id        TEXT PRIMARY KEY,
                    type_id        TEXT NOT NULL,
                    source_node    TEXT NOT NULL,
                    target_node    TEXT NOT NULL,
                    properties     BLOB NOT NULL,
                    created_event  TEXT NOT NULL
                );
                CREATE INDEX IF NOT EXISTS edges_source_idx ON edges(source_node);
                CREATE INDEX IF NOT EXISTS edges_target_idx ON edges(target_node);
                CREATE INDEX IF NOT EXISTS edges_type_idx ON edges(type_id);
                "#,
            )
            .map_err(|e| GraphError::Internal(format!("apply schema: {e}")))?;
        Ok(())
    }

    fn map_sqlite_err(e: rusqlite::Error) -> GraphError {
        match e {
            rusqlite::Error::QueryReturnedNoRows => GraphError::NotFound("no rows".into()),
            other => GraphError::ResourceUnavailable(other.to_string()),
        }
    }
}

impl StorageBackend for SqliteStorage {}

impl Storage for SqliteStorage {
    fn append_event(&mut self, event: &Event) -> GraphResult<EventId> {
        let payload = serde_json::to_vec(&event.payload)
            .map_err(|e| GraphError::Internal(format!("encode payload: {e}")))?;
        self.conn
            .execute(
                "INSERT INTO events(event_id, occurred_at, kind, actor, authz_decision, payload, prev_event_hash, event_hash) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    event.event_id.to_string(),
                    event.occurred_at,
                    event_kind_str(event.kind),
                    event.actor,
                    event.authz_decision,
                    payload,
                    event.prev_event_hash,
                    event.event_hash,
                ],
            )
            .map_err(Self::map_sqlite_err)?;
        Ok(event.event_id)
    }

    fn chain_head(&self) -> GraphResult<Option<String>> {
        let head: Option<String> = self
            .conn
            .query_row(
                "SELECT event_hash FROM events ORDER BY occurred_at DESC, event_id DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(Self::map_sqlite_err)?;
        Ok(head)
    }

    fn last_event(&self) -> GraphResult<Option<Event>> {
        let row: Option<(String, String, String, String, String, Vec<u8>, Option<String>, String)> = self
            .conn
            .query_row(
                "SELECT event_id, occurred_at, kind, actor, authz_decision, payload, prev_event_hash, event_hash FROM events ORDER BY occurred_at DESC, event_id DESC LIMIT 1",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                        row.get(7)?,
                    ))
                },
            )
            .optional()
            .map_err(Self::map_sqlite_err)?;
        let Some((id, occurred_at, kind, actor, authz, payload, prev, hash)) = row else {
            return Ok(None);
        };
        let event_id = Uuid::parse_str(&id)
            .map_err(|e| GraphError::Internal(format!("event_id parse: {e}")))?;
        let kind = parse_event_kind(&kind)
            .ok_or_else(|| GraphError::Internal(format!("unknown event kind {kind}")))?;
        let payload: Value = serde_json::from_slice(&payload)
            .map_err(|e| GraphError::Internal(format!("payload decode: {e}")))?;
        Ok(Some(Event {
            event_id,
            occurred_at,
            kind,
            actor,
            authz_decision: authz,
            payload,
            prev_event_hash: prev,
            event_hash: hash,
        }))
    }

    fn iter_events(&self) -> GraphResult<Vec<Event>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT event_id, occurred_at, kind, actor, authz_decision, payload, prev_event_hash, event_hash FROM events ORDER BY occurred_at, event_id",
            )
            .map_err(Self::map_sqlite_err)?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Vec<u8>>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, String>(7)?,
                ))
            })
            .map_err(Self::map_sqlite_err)?;
        let mut out = Vec::new();
        for r in rows {
            let (id, occurred_at, kind, actor, authz, payload, prev, hash) =
                r.map_err(Self::map_sqlite_err)?;
            let event_id = Uuid::parse_str(&id)
                .map_err(|e| GraphError::Internal(format!("event_id parse: {e}")))?;
            let kind = parse_event_kind(&kind)
                .ok_or_else(|| GraphError::Internal(format!("unknown event kind {kind}")))?;
            let payload: Value = serde_json::from_slice(&payload)
                .map_err(|e| GraphError::Internal(format!("payload decode: {e}")))?;
            out.push(Event {
                event_id,
                occurred_at,
                kind,
                actor,
                authz_decision: authz,
                payload,
                prev_event_hash: prev,
                event_hash: hash,
            });
        }
        Ok(out)
    }

    fn upsert_node(&mut self, node: &Node) -> GraphResult<()> {
        let props = serde_json::to_vec(&node.properties)
            .map_err(|e| GraphError::Internal(format!("encode props: {e}")))?;
        self.conn
            .execute(
                "INSERT INTO nodes(node_id, type_id, state, properties, revisioned, immutable, created_event, current_revision, tombstoned) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                 ON CONFLICT(node_id) DO UPDATE SET type_id=excluded.type_id, state=excluded.state, properties=excluded.properties, revisioned=excluded.revisioned, immutable=excluded.immutable, current_revision=excluded.current_revision, tombstoned=excluded.tombstoned",
                params![
                    node.id.to_string(),
                    node.type_id,
                    node.state.as_str(),
                    props,
                    node.revisioned as i32,
                    node.immutable as i32,
                    node.created_event.to_string(),
                    node.current_revision.map(|n| n.to_string()),
                    node.tombstoned as i32,
                ],
            )
            .map_err(Self::map_sqlite_err)?;
        Ok(())
    }

    fn update_node_state(&mut self, id: NodeId, state: NodeState) -> GraphResult<()> {
        let updated = self
            .conn
            .execute(
                "UPDATE nodes SET state = ?1 WHERE node_id = ?2 AND tombstoned = 0",
                params![state.as_str(), id.to_string()],
            )
            .map_err(Self::map_sqlite_err)?;
        if updated == 0 {
            return Err(GraphError::NotFound(format!("node {id}")));
        }
        Ok(())
    }

    fn tombstone_node(&mut self, id: NodeId) -> GraphResult<()> {
        let updated = self
            .conn
            .execute(
                "UPDATE nodes SET tombstoned = 1, state = 'archived' WHERE node_id = ?1",
                params![id.to_string()],
            )
            .map_err(Self::map_sqlite_err)?;
        if updated == 0 {
            return Err(GraphError::NotFound(format!("node {id}")));
        }
        Ok(())
    }

    fn get_node(&self, id: NodeId) -> GraphResult<Option<Node>> {
        let row: Option<(String, String, Vec<u8>, i32, i32, String, Option<String>, i32)> = self
            .conn
            .query_row(
                "SELECT type_id, state, properties, revisioned, immutable, created_event, current_revision, tombstoned FROM nodes WHERE node_id = ?1",
                params![id.to_string()],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                        row.get(7)?,
                    ))
                },
            )
            .optional()
            .map_err(Self::map_sqlite_err)?;
        let Some((type_id, state, props, revisioned, immutable, created, current_rev, tomb)) = row
        else {
            return Ok(None);
        };
        let properties: Properties = serde_json::from_slice(&props)
            .map_err(|e| GraphError::Internal(format!("decode props: {e}")))?;
        let state = parse_state(&state)
            .ok_or_else(|| GraphError::Internal(format!("unknown state {state}")))?;
        let created_event = Uuid::parse_str(&created)
            .map_err(|e| GraphError::Internal(format!("created_event parse: {e}")))?;
        let current_revision = match current_rev {
            Some(s) => Some(
                Uuid::parse_str(&s)
                    .map_err(|e| GraphError::Internal(format!("current_revision parse: {e}")))?,
            ),
            None => None,
        };
        Ok(Some(Node {
            id,
            type_id,
            state,
            properties,
            revisioned: revisioned != 0,
            immutable: immutable != 0,
            created_event,
            current_revision,
            tombstoned: tomb != 0,
        }))
    }

    fn insert_edge(&mut self, edge: &Edge) -> GraphResult<()> {
        let props = serde_json::to_vec(&edge.properties)
            .map_err(|e| GraphError::Internal(format!("encode edge props: {e}")))?;
        self.conn
            .execute(
                "INSERT INTO edges(edge_id, type_id, source_node, target_node, properties, created_event) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(edge_id) DO NOTHING",
                params![
                    edge.id.to_string(),
                    edge.type_id,
                    edge.source.to_string(),
                    edge.target.to_string(),
                    props,
                    edge.created_event.to_string(),
                ],
            )
            .map_err(Self::map_sqlite_err)?;
        Ok(())
    }

    fn remove_edge(&mut self, id: EdgeId) -> GraphResult<()> {
        let updated = self
            .conn
            .execute(
                "DELETE FROM edges WHERE edge_id = ?1",
                params![id.to_string()],
            )
            .map_err(Self::map_sqlite_err)?;
        if updated == 0 {
            return Err(GraphError::NotFound(format!("edge {id}")));
        }
        Ok(())
    }

    fn get_edge(&self, id: EdgeId) -> GraphResult<Option<Edge>> {
        let row: Option<(String, String, String, Vec<u8>, String)> = self
            .conn
            .query_row(
                "SELECT type_id, source_node, target_node, properties, created_event FROM edges WHERE edge_id = ?1",
                params![id.to_string()],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .optional()
            .map_err(Self::map_sqlite_err)?;
        let Some((type_id, source, target, props, created)) = row else {
            return Ok(None);
        };
        let properties: Properties = serde_json::from_slice(&props)
            .map_err(|e| GraphError::Internal(format!("decode edge props: {e}")))?;
        let source = Uuid::parse_str(&source)
            .map_err(|e| GraphError::Internal(format!("source parse: {e}")))?;
        let target = Uuid::parse_str(&target)
            .map_err(|e| GraphError::Internal(format!("target parse: {e}")))?;
        let created_event = Uuid::parse_str(&created)
            .map_err(|e| GraphError::Internal(format!("created_event parse: {e}")))?;
        Ok(Some(Edge {
            id,
            type_id,
            source,
            target,
            properties,
            created_event,
        }))
    }

    fn find_nodes_by_type(&self, type_id: &str) -> GraphResult<Vec<Node>> {
        let mut stmt = self
            .conn
            .prepare("SELECT node_id FROM nodes WHERE type_id = ?1")
            .map_err(Self::map_sqlite_err)?;
        let ids: Vec<String> = stmt
            .query_map(params![type_id], |row| row.get::<_, String>(0))
            .map_err(Self::map_sqlite_err)?
            .filter_map(|r| r.ok())
            .collect();
        let mut out = Vec::new();
        for id in ids {
            if let Some(n) = self.get_node(Uuid::parse_str(&id).map_err(|e| GraphError::Internal(e.to_string()))?)? {
                out.push(n);
            }
        }
        Ok(out)
    }

    fn out_edges(&self, source: NodeId) -> GraphResult<Vec<Edge>> {
        self.edges_with_source_or_target("source_node = ?1", source)
    }

    fn in_edges(&self, target: NodeId) -> GraphResult<Vec<Edge>> {
        self.edges_with_source_or_target("target_node = ?1", target)
    }

    fn edges_between(&self, source: NodeId, target: NodeId) -> GraphResult<Vec<Edge>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT edge_id FROM edges WHERE source_node = ?1 AND target_node = ?2",
            )
            .map_err(Self::map_sqlite_err)?;
        let ids: Vec<String> = stmt
            .query_map(params![source.to_string(), target.to_string()], |row| {
                row.get::<_, String>(0)
            })
            .map_err(Self::map_sqlite_err)?
            .filter_map(|r| r.ok())
            .collect();
        let mut out = Vec::new();
        for id in ids {
            if let Some(e) = self.get_edge(Uuid::parse_str(&id).map_err(|e| GraphError::Internal(e.to_string()))?)? {
                out.push(e);
            }
        }
        Ok(out)
    }

    fn node_count(&self) -> GraphResult<u64> {
        let n: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM nodes", [], |row| row.get(0))
            .map_err(Self::map_sqlite_err)?;
        Ok(n as u64)
    }

    fn edge_count(&self) -> GraphResult<u64> {
        let n: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM edges", [], |row| row.get(0))
            .map_err(Self::map_sqlite_err)?;
        Ok(n as u64)
    }

    fn event_count(&self) -> GraphResult<u64> {
        let n: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM events", [], |row| row.get(0))
            .map_err(Self::map_sqlite_err)?;
        Ok(n as u64)
    }
}

impl SqliteStorage {
    fn edges_with_source_or_target(
        &self,
        clause: &str,
        id: NodeId,
    ) -> GraphResult<Vec<Edge>> {
        let sql = format!("SELECT edge_id FROM edges WHERE {clause}");
        let mut stmt = self.conn.prepare(&sql).map_err(Self::map_sqlite_err)?;
        let ids: Vec<String> = stmt
            .query_map(params![id.to_string()], |row| row.get::<_, String>(0))
            .map_err(Self::map_sqlite_err)?
            .filter_map(|r| r.ok())
            .collect();
        let mut out = Vec::new();
        for id in ids {
            if let Some(e) = self.get_edge(Uuid::parse_str(&id).map_err(|e| GraphError::Internal(e.to_string()))?)? {
                out.push(e);
            }
        }
        Ok(out)
    }
}

fn event_kind_str(k: EventKind) -> &'static str {
    match k {
        EventKind::NodeUpserted => "node_upserted",
        EventKind::NodeStateChanged => "node_state_changed",
        EventKind::NodeTombstoned => "node_tombstoned",
        EventKind::EdgeAdded => "edge_added",
        EventKind::EdgeRemoved => "edge_removed",
        EventKind::RevisionSuperseded => "revision_superseded",
    }
}

fn parse_event_kind(s: &str) -> Option<crate::event::EventKind> {
    use crate::event::EventKind::*;
    match s {
        "node_upserted" => Some(NodeUpserted),
        "node_state_changed" => Some(NodeStateChanged),
        "node_tombstoned" => Some(NodeTombstoned),
        "edge_added" => Some(EdgeAdded),
        "edge_removed" => Some(EdgeRemoved),
        "revision_superseded" => Some(RevisionSuperseded),
        _ => None,
    }
}

fn parse_state(s: &str) -> Option<NodeState> {
    NodeState::parse(s)
}

// Suppress unused warnings for types reserved for Phase 2 use.
#[allow(dead_code)]
fn _types_reserved(_n: &NodeTypeSpec, _e: &EdgeTypeSpec, _c: &Cardinality) {}
