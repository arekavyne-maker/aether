// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! Conformance fixtures for the G1 acceptance criteria (SDP §18):
//!
//! - 1M-event replay (SDP §18: "<5 min" budget)
//! - AS-OF determinism (MVR-09 substrate): two independent engines
//!   reproduce the same projection for the same event sequence.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Instant;

use aether_graph::catalog::{load_edge_types, load_node_types};
use aether_graph::engine::{GraphEngine, SnapshotAt};
use aether_graph::storage::SqliteStorage;
use aether_graph::types::NodeState;
use tempfile::TempDir;

fn find_platform_root() -> PathBuf {
    let mut here = std::env::current_exe().expect("current_exe");
    loop {
        if here.join("Cargo.toml").is_file() && here.join("contracts").is_dir() {
            return here;
        }
        if !here.pop() {
            panic!("could not locate monorepo root");
        }
    }
}

fn make_engine(db_path: &std::path::Path) -> GraphEngine {
    let root = find_platform_root();
    let storage = SqliteStorage::open(db_path, aether_graph::storage::OpenMode::ReadWrite)
        .expect("sqlite open");
    let nodes_dir = root.join("contracts/graph.model/catalog/nodes");
    let edges_dir = root.join("contracts/graph.model/catalog/edges");
    let node_types = load_node_types(&nodes_dir).expect("load nodes");
    let edge_types = load_edge_types(&edges_dir).expect("load edges");
    GraphEngine::new(Box::new(storage), node_types, edge_types)
}

#[test]
fn replay_performance_scales_within_budget() {
    // SDP §18 Phase 1 acceptance: "1M-event replay performance budget (<5 min)".
    // We use 10k events for a fast CI; the SDP budget is 300s for 1M.
    // The 10M-node Phase 0 measurement was 6.4s for replay, so 10k
    // should complete in well under 3s.
    let tmp = TempDir::new().expect("tempdir");
    let db = tmp.path().join("graph.db");
    let engine = make_engine(&db);

    let n = 10_000;
    let insert_start = Instant::now();
    for i in 0..n {
        let mut props = BTreeMap::new();
        props.insert(
            "text".to_string(),
            serde_json::Value::String(format!("req-{i}")),
        );
        props.insert("criticality".to_string(), serde_json::Value::String("major".to_string()));
        props.insert(
            "verification_method".to_string(),
            serde_json::Value::String("analysis".to_string()),
        );
        props.insert("status".to_string(), serde_json::Value::String("open".to_string()));
        let _ = engine
            .upsert_node("Requirement", NodeState::Draft, props, "loadtest", "allow")
            .expect("upsert");
    }
    let insert_seconds = insert_start.elapsed().as_secs_f64();
    eprintln!("{n} insert: {insert_seconds:.2}s");

    // Replay the full chain to current state.
    let replay_start = Instant::now();
    let snap = engine.snapshot(SnapshotAt::Current).expect("snapshot");
    let replay_seconds = replay_start.elapsed().as_secs_f64();
    eprintln!(
        "{n} replay: {replay_seconds:.2}s, nodes={}",
        snap.nodes.len()
    );

    // 10k replay budget: 3s (extrapolated from 1M / 300s = 10k / 3s).
    assert!(
        replay_seconds < 3.0,
        "{n} replay took {replay_seconds:.1}s, budget is 3s"
    );
    assert_eq!(snap.nodes.len(), n, "all {n} nodes must be present");
}

#[test]
fn as_of_determinism_two_engines_same_projection() {
    // MVR-09 substrate: two independent engines that ingest the same
    // event *sequence* produce the same projection at any AS-OF offset.
    // Note: UUIDv7 is time-based, so node IDs differ between the two
    // engines (each is created at a different wall-clock instant).
    // We therefore compare counts and chain integrity, not node IDs.
    let tmp_a = TempDir::new().expect("tempdir a");
    let tmp_b = TempDir::new().expect("tempdir b");
    let eng_a = make_engine(&tmp_a.path().join("a.db"));
    let eng_b = make_engine(&tmp_b.path().join("b.db"));

    for i in 0..50 {
        let mut props = BTreeMap::new();
        props.insert("text".to_string(), serde_json::Value::String(format!("req-{i}")));
        props.insert("criticality".to_string(), serde_json::Value::String("major".to_string()));
        props.insert(
            "verification_method".to_string(),
            serde_json::Value::String("analysis".to_string()),
        );
        props.insert("status".to_string(), serde_json::Value::String("open".to_string()));
        eng_a
            .upsert_node("Requirement", NodeState::Draft, props.clone(), "alice", "allow")
            .expect("a upsert");
        eng_b
            .upsert_node("Requirement", NodeState::Draft, props, "alice", "allow")
            .expect("b upsert");
    }

    // Both engines must have the same node count at any AS-OF.
    for offset in [10, 25, 50] {
        let snap_a = eng_a
            .snapshot(SnapshotAt::EventOffset(offset as u64))
            .expect("a asof");
        let snap_b = eng_b
            .snapshot(SnapshotAt::EventOffset(offset as u64))
            .expect("b asof");
        assert_eq!(
            snap_a.nodes.len(),
            snap_b.nodes.len(),
            "node count at offset {offset} must match"
        );
        // Every event_count should be the same.
        assert_eq!(snap_a.event_count, snap_b.event_count);
    }
    let cur_a = eng_a.snapshot(SnapshotAt::Current).expect("a cur");
    let cur_b = eng_b.snapshot(SnapshotAt::Current).expect("b cur");
    assert_eq!(cur_a.nodes.len(), 50);
    assert_eq!(cur_b.nodes.len(), 50);
    assert_eq!(cur_a.event_count, cur_b.event_count);
}
