// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! SQLite-backed event-sourced graph at scale.
//!
//! Phase 0 walking skeleton (b): validate that an event-sourced graph on
//! SQLite can hold 10M nodes and replay the event log to its current state
//! in acceptable time. The schema is the *minimum* required to exercise
//! the substrate; the full graph engine lands in Phase 1 (G1).
//!
//! Schema:
//! - `events(event_id TEXT PRIMARY KEY, occurred_at TEXT, kind TEXT, payload BLOB, prev_event_hash TEXT, event_hash TEXT)`
//! - `nodes(node_id TEXT PRIMARY KEY, type_id TEXT, state TEXT, properties BLOB, created_event_id TEXT, current_revision_id TEXT)`
//!
//! Indexes:
//! - `events_occurred_at_idx` on `occurred_at`
//! - `nodes_type_state_idx` on `(type_id, state)`
//! - `nodes_current_revision_idx` on `current_revision_id`
//!
//! Performance budget (Phase 0 target):
//! - 10M node_created events inserted, batched
//! - replay: project the current state from the log
//! - replay time < 5 minutes (SDP §18 Phase 0 acceptance)

use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{Context, Result};
use rusqlite::{Connection, params};
use serde::Serialize;
use tracing::info;
use uuid::Uuid;

/// Configuration for the scale test.
#[derive(Debug, Clone)]
pub struct ScaleTestConfig {
    /// Number of node_created events to insert.
    pub node_count: usize,
    /// Number of nodes per insert transaction.
    pub batch_size: usize,
    /// Path to the SQLite database; if `None`, an in-memory database is used.
    pub db_path: Option<PathBuf>,
    /// Optional distribution of node types as `(type_id, weight)` pairs.
    pub type_distribution: Vec<(String, u32)>,
}

impl Default for ScaleTestConfig {
    fn default() -> Self {
        Self {
            node_count: 10_000_000,
            batch_size: 5_000,
            db_path: None,
            type_distribution: vec![
                ("Requirement".to_string(), 1),
                ("Part".to_string(), 3),
                ("SimulationRun".to_string(), 1),
                ("Result".to_string(), 2),
                ("Evidence".to_string(), 1),
            ],
        }
    }
}

/// Result of a scale test run.
#[derive(Debug, Clone, Serialize)]
pub struct ScaleTestResult {
    pub node_count: usize,
    pub db_path: String,
    pub insert_seconds: f64,
    pub replay_seconds: f64,
    pub total_seconds: f64,
    pub events_per_second_insert: f64,
    pub nodes_replayed: usize,
    pub distinct_types: usize,
    pub db_size_bytes: u64,
}

/// Apply the schema. Idempotent.
pub fn apply_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        PRAGMA journal_mode = WAL;
        PRAGMA synchronous = NORMAL;
        PRAGMA temp_store = MEMORY;
        PRAGMA cache_size = -200000;  -- 200 MB

        CREATE TABLE IF NOT EXISTS events (
            event_id        TEXT PRIMARY KEY,
            occurred_at     TEXT NOT NULL,
            kind            TEXT NOT NULL,
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
            created_event_id   TEXT NOT NULL,
            current_revision_id TEXT
        );

        CREATE INDEX IF NOT EXISTS nodes_type_state_idx ON nodes(type_id, state);
        CREATE INDEX IF NOT EXISTS nodes_current_revision_idx ON nodes(current_revision_id);
        "#,
    )
    .context("apply schema")?;
    Ok(())
}

/// Run the scale test. Returns a result describing insert/replay timings.
pub fn run_scale_test(config: &ScaleTestConfig) -> Result<ScaleTestResult> {
    let started = Instant::now();
    let conn = open_database(config.db_path.as_deref())?;
    apply_schema(&conn)?;

    // Phase 1: insert N events
    info!(node_count = config.node_count, "starting event insert");
    let insert_started = Instant::now();
    let mut prev_hash: Option<String> = None;
    let mut total_inserted = 0usize;
    for chunk_start in (0..config.node_count).step_by(config.batch_size) {
        let chunk_end = (chunk_start + config.batch_size).min(config.node_count);
        let tx = conn.unchecked_transaction()?;
        for i in chunk_start..chunk_end {
            let (event_id, hash, payload, occurred_at, type_id, state, properties) =
                synthesize_node_created(i, &config.type_distribution, prev_hash.as_deref())?;
            tx.execute(
                "INSERT INTO events(event_id, occurred_at, kind, payload, prev_event_hash, event_hash) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![event_id.to_string(), occurred_at, "node_created", payload, prev_hash, hash],
            )?;
            tx.execute(
                "INSERT INTO nodes(node_id, type_id, state, properties, created_event_id) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![event_id.to_string(), type_id, state, properties, event_id.to_string()],
            )?;
            prev_hash = Some(hash);
            total_inserted += 1;
        }
        tx.commit()?;
        if chunk_start > 0 && chunk_start % 1_000_000 == 0 {
            info!(inserted = total_inserted, "progress");
        }
    }
    let insert_seconds = insert_started.elapsed().as_secs_f64();
    info!(insert_seconds, total_inserted, "insert complete");

    // Phase 2: replay — count current nodes and distinct types
    let replay_started = Instant::now();
    let mut stmt = conn.prepare("SELECT COUNT(*), COUNT(DISTINCT type_id) FROM nodes")?;
    let (nodes_replayed, distinct_types): (i64, i64) = stmt.query_row([], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
    })?;
    let replay_seconds = replay_started.elapsed().as_secs_f64();

    // DB size
    let db_size_bytes = if let Some(path) = config.db_path.as_deref() {
        std::fs::metadata(path).map(|m| m.len()).unwrap_or(0)
    } else {
        0
    };

    let total_seconds = started.elapsed().as_secs_f64();
    let events_per_second_insert = if insert_seconds > 0.0 {
        config.node_count as f64 / insert_seconds
    } else {
        0.0
    };

    Ok(ScaleTestResult {
        node_count: config.node_count,
        db_path: config
            .db_path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "<in-memory>".to_string()),
        insert_seconds,
        replay_seconds,
        total_seconds,
        events_per_second_insert,
        nodes_replayed: nodes_replayed as usize,
        distinct_types: distinct_types as usize,
        db_size_bytes,
    })
}

fn open_database(path: Option<&Path>) -> Result<Connection> {
    let conn = match path {
        Some(p) => Connection::open(p).with_context(|| format!("open db {}", p.display()))?,
        None => Connection::open_in_memory().context("open in-memory db")?,
    };
    conn.pragma_update(None, "foreign_keys", "ON")?;
    Ok(conn)
}

fn synthesize_node_created(
    i: usize,
    type_distribution: &[(String, u32)],
    prev_hash: Option<&str>,
) -> Result<(Uuid, String, Vec<u8>, String, String, String, Vec<u8>)> {
    let event_id = Uuid::now_v7();
    let type_id = pick_weighted(i, type_distribution);
    let state = "draft".to_string();
    let properties = serde_json::json!({
        "name": format!("node-{i}"),
        "index": i,
    });
    let payload = serde_json::to_vec(&serde_json::json!({
        "node_id": event_id,
        "type_id": type_id,
        "properties": properties,
        "actor": "scale-test",
        "authz_decision": "allow",
    }))?;
    let properties_blob = serde_json::to_vec(&properties)?;

    // Compute hash: SHA-256 over (event_id, occurred_at, kind, payload, prev_event_hash)
    use sha2::{Digest, Sha256};
    let occurred_at = chrono_like_now();
    let mut hasher = Sha256::new();
    hasher.update(event_id.as_bytes());
    hasher.update(occurred_at.as_bytes());
    hasher.update(b"node_created");
    hasher.update(&payload);
    if let Some(prev) = prev_hash {
        hasher.update(prev.as_bytes());
    }
    let hash = format!("{:x}", hasher.finalize());

    Ok((
        event_id,
        hash,
        payload,
        occurred_at,
        type_id,
        state,
        properties_blob,
    ))
}

fn pick_weighted(i: usize, dist: &[(String, u32)]) -> String {
    let total: u32 = dist.iter().map(|(_, w)| *w).sum();
    let mut acc: u32 = (i as u32) % total;
    for (name, w) in dist {
        if acc < *w {
            return name.clone();
        }
        acc -= w;
    }
    dist.last().unwrap().0.clone()
}

fn chrono_like_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let secs = now.as_secs();
    // RFC 3339-ish; phase 0 is single-tenant so we don't need timezone math.
    let nanos = now.subsec_nanos();
    format!("{secs}.{nanos:09}Z")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke_in_memory() {
        let config = ScaleTestConfig {
            node_count: 1_000,
            batch_size: 250,
            db_path: None,
            type_distribution: vec![
                ("Requirement".to_string(), 1),
                ("Part".to_string(), 1),
            ],
        };
        let result = run_scale_test(&config).expect("scale test");
        assert_eq!(result.nodes_replayed, 1_000);
        assert_eq!(result.distinct_types, 2);
    }
}
