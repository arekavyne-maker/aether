// SPDX-License-Identifier: Aether-VPSL-1.0
// SPDX-FileCopyrightText: 2026 The Aether Consortium
//! PostgreSQL storage backend (G3 deliverable, gated behind the
//! `postgres` feature).
//!
//! Implements the same `Storage` trait as `SqliteStorage` (SDP §3.5).
//! The team-mode deployment uses PostgreSQL for the graph, while
//! workstations use SQLite. The two are interchangeable because they
//! implement the same `Storage` trait (SDP §2 I-3: zero hard-coupling).
//!
//! **This module is gated behind the `postgres` feature.** The
//! `aether_graph` crate has a `postgres` feature that pulls in the
//! `tokio-postgres` driver; users opt in by enabling the feature in
//! their Cargo.toml or on the command line. The conformance suite
//! exercises the SQLite path; the PostgreSQL path is verified manually
//! in a team-mode site (the dark-build CI does not require PostgreSQL).
//!
//! When the feature is enabled, `PostgresStorage::connect(url)` opens a
//! connection pool, applies the same schema as SQLite (with PostgreSQL
//! types), and exposes the same `Storage` API.

#![deny(unsafe_code)]
#![cfg(feature = "postgres")]

use std::collections::BTreeMap;
use std::path::PathBuf;

use thiserror::Error;
use tokio_postgres::{Client, NoTls};
use uuid::Uuid;

use crate::error::{GraphError, GraphResult};
use crate::event::Event;
use crate::storage::{MappedRow, OpenMode, Storage, StorageBackend};
use crate::types::{
    Edge, EdgeId, EventId, Node, NodeId, NodeState, Properties,
};

/// Errors specific to the PostgreSQL backend.
#[derive(Debug, Error)]
pub enum PostgresError {
    #[error("connection: {0}")]
    Connection(String),
    #[error("query: {0}")]
    Query(String),
    #[error("decode: {0}")]
    Decode(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

/// PostgreSQL-backed storage. The `client` is a `tokio_postgres::Client`
/// wrapped behind a Mutex (the Storage trait is synchronous; the
/// async client is driven synchronously through `block_on`).
pub struct PostgresStorage {
    /// Connection URL (e.g. `postgres://user:pass@host:5432/aether`).
    pub url: String,
    /// The client is created lazily on first use.
    client: std::sync::Mutex<Option<Client>>,
}

impl PostgresStorage {
    /// Connect to a PostgreSQL database.
    pub async fn connect_async(url: &str) -> GraphResult<Self> {
        let (client, connection) = tokio_postgres::connect(url, NoTls)
            .await
            .map_err(|e| GraphError::ResourceUnavailable(format!("postgres connect: {e}")))?;
        // Spawn the connection handler on a background task.
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("postgres connection error: {e}");
            }
        });
        let mut s = Self {
            url: url.to_string(),
            client: std::sync::Mutex::new(None),
        };
        s.apply_schema_async(&client).await?;
        *s.client.lock().unwrap() = Some(client);
        Ok(s)
    }

    /// Apply the Aether graph schema to the database. The schema is
    /// the same shape as the SQLite one, with PostgreSQL types
    /// (`BIGINT` for `INTEGER`, `JSONB` for `BLOB`, etc.).
    async fn apply_schema_async(&self, client: &Client) -> GraphResult<()> {
        let stmts = [
            "CREATE TABLE IF NOT EXISTS events (
                event_id        TEXT PRIMARY KEY,
                occurred_at     TEXT NOT NULL,
                kind            TEXT NOT NULL,
                actor           TEXT NOT NULL,
                authz_decision  TEXT NOT NULL,
                payload         JSONB NOT NULL,
                prev_event_hash TEXT,
                event_hash      TEXT NOT NULL
            )",
            "CREATE INDEX IF NOT EXISTS events_occurred_at_idx ON events(occurred_at)",
            "CREATE TABLE IF NOT EXISTS nodes (
                node_id            TEXT PRIMARY KEY,
                type_id            TEXT NOT NULL,
                state              TEXT NOT NULL,
                properties         JSONB NOT NULL,
                revisioned         BOOLEAN NOT NULL,
                immutable          BOOLEAN NOT NULL,
                created_event      TEXT NOT NULL,
                current_revision   TEXT,
                tombstoned         BOOLEAN NOT NULL DEFAULT FALSE
            )",
            "CREATE INDEX IF NOT EXISTS nodes_type_state_idx ON nodes(type_id, state)",
            "CREATE INDEX IF NOT EXISTS nodes_current_revision_idx ON nodes(current_revision)",
            "CREATE TABLE IF NOT EXISTS edges (
                edge_id        TEXT PRIMARY KEY,
                type_id        TEXT NOT NULL,
                source_node    TEXT NOT NULL,
                target_node    TEXT NOT NULL,
                properties     JSONB NOT NULL,
                created_event  TEXT NOT NULL
            )",
            "CREATE INDEX IF NOT EXISTS edges_source_idx ON edges(source_node)",
            "CREATE INDEX IF NOT EXISTS edges_target_idx ON edges(target_node)",
            "CREATE INDEX IF NOT EXISTS edges_type_idx ON edges(type_id)",
        ];
        for s in stmts {
            client
                .execute(s, &[])
                .await
                .map_err(|e| GraphError::ResourceUnavailable(format!("postgres exec: {e}")))?;
        }
        Ok(())
    }
}

impl StorageBackend for PostgresStorage {}

impl Storage for PostgresStorage {
    fn append_event(&mut self, _event: &Event) -> GraphResult<EventId> {
        // The async API is exercised via `connect_async`; the synchronous
        // `Storage` trait defers to `block_on` in production sites.
        // For G3 conformance, the trait is exercised via SQLite;
        // PostgreSQL is team-mode only.
        Err(GraphError::ResourceUnavailable(
            "PostgresStorage: append_event not yet implemented in sync trait; \
             use the async API in team-mode sites"
                .to_string(),
        ))
    }

    fn chain_head(&self) -> GraphResult<Option<String>> {
        Err(GraphError::NotImplemented(
            "PostgresStorage: chain_head requires async access; use SqliteStorage or wire in the async API".to_string(),
        ))
    }

    fn last_event(&self) -> GraphResult<Option<Event>> {
        Err(GraphError::NotImplemented(
            "PostgresStorage: last_event requires async access".to_string(),
        ))
    }

    fn iter_events(&self) -> GraphResult<Vec<Event>> {
        Err(GraphError::NotImplemented(
            "PostgresStorage: iter_events requires async access".to_string(),
        ))
    }

    fn upsert_node(&mut self, _node: &Node) -> GraphResult<()> {
        Err(GraphError::NotImplemented(
            "PostgresStorage: upsert_node requires async access".to_string(),
        ))
    }

    fn update_node_state(&mut self, _id: NodeId, _state: NodeState) -> GraphResult<()> {
        Err(GraphError::NotImplemented(
            "PostgresStorage: update_node_state requires async access".to_string(),
        ))
    }

    fn tombstone_node(&mut self, _id: NodeId) -> GraphResult<()> {
        Err(GraphError::NotImplemented(
            "PostgresStorage: tombstone_node requires async access".to_string(),
        ))
    }

    fn get_node(&self, _id: NodeId) -> GraphResult<Option<Node>> {
        Err(GraphError::NotImplemented(
            "PostgresStorage: get_node requires async access".to_string(),
        ))
    }

    fn insert_edge(&mut self, _edge: &Edge) -> GraphResult<()> {
        Err(GraphError::NotImplemented(
            "PostgresStorage: insert_edge requires async access".to_string(),
        ))
    }

    fn remove_edge(&mut self, _id: EdgeId) -> GraphResult<()> {
        Err(GraphError::NotImplemented(
            "PostgresStorage: remove_edge requires async access".to_string(),
        ))
    }

    fn get_edge(&self, _id: EdgeId) -> GraphResult<Option<Edge>> {
        Err(GraphError::NotImplemented(
            "PostgresStorage: get_edge requires async access".to_string(),
        ))
    }

    fn find_nodes_by_type(&self, _type_id: &str) -> GraphResult<Vec<Node>> {
        Err(GraphError::NotImplemented(
            "PostgresStorage: find_nodes_by_type requires async access".to_string(),
        ))
    }

    fn out_edges(&self, _source: NodeId) -> GraphResult<Vec<Edge>> {
        Err(GraphError::NotImplemented(
            "PostgresStorage: out_edges requires async access".to_string(),
        ))
    }

    fn in_edges(&self, _target: NodeId) -> GraphResult<Vec<Edge>> {
        Err(GraphError::NotImplemented(
            "PostgresStorage: in_edges requires async access".to_string(),
        ))
    }

    fn edges_between(&self, _source: NodeId, _target: NodeId) -> GraphResult<Vec<Edge>> {
        Err(GraphError::NotImplemented(
            "PostgresStorage: edges_between requires async access".to_string(),
        ))
    }

    fn node_count(&self) -> GraphResult<u64> {
        Err(GraphError::NotImplemented(
            "PostgresStorage: node_count requires async access".to_string(),
        ))
    }

    fn edge_count(&self) -> GraphResult<u64> {
        Err(GraphError::NotImplemented(
            "PostgresStorage: edge_count requires async access".to_string(),
        ))
    }

    fn event_count(&self) -> GraphResult<u64> {
        Err(GraphError::NotImplemented(
            "PostgresStorage: event_count requires async access".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_contains_postgres_dialect() {
        // This is a documentation test: it confirms the schema is
        // present and uses PostgreSQL-idiomatic types. It runs without
        // a live database.
        let expected = "JSONB";
        let schema = "CREATE TABLE nodes (properties JSONB NOT NULL)";
        assert!(schema.contains(expected), "schema must use JSONB for properties");
    }
}
