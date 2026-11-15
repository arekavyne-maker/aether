// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! Engine conformance tests — graph invariants + AS-OF + AQL.
//!
//! These tests exercise the SDP §3.6 invariants on the Phase 1 engine
//! and the MVR-09 substrate (replay restores the same graph).

use std::collections::BTreeMap;
use std::path::PathBuf;

use aether_graph::catalog::{load_edge_types, load_node_types};
use aether_graph::engine::{AqlQuery, GraphEngine, SnapshotAt};
use aether_graph::error::GraphError;
use aether_graph::storage::SqliteStorage;
use aether_graph::types::{NodeState, Properties};
use serde_json::json;

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

fn make_engine() -> GraphEngine {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let db = tmp.path().join("graph.db");
    make_engine_at(db)
}

fn make_engine_at(db_path: std::path::PathBuf) -> GraphEngine {
    let root = find_platform_root();
    let storage = SqliteStorage::open(&db_path, aether_graph::storage::OpenMode::ReadWrite)
        .expect("sqlite open");
    let nodes_dir = root.join("contracts/graph.model/catalog/nodes");
    let edges_dir = root.join("contracts/graph.model/catalog/edges");
    let node_types = load_node_types(&nodes_dir).expect("load nodes");
    let edge_types = load_edge_types(&edges_dir).expect("load edges");
    GraphEngine::new(Box::new(storage), node_types, edge_types)
}

fn actor() -> &'static str {
    "tester"
}
fn authz() -> &'static str {
    "allow"
}

#[test]
fn creates_node_and_edge_with_invariants() {
    let engine = make_engine();
    let system = engine
        .upsert_node("System", NodeState::Draft, default_props(), actor(), authz())
        .expect("create system");
    let part = engine
        .upsert_node("Part", NodeState::Draft, default_props(), actor(), authz())
        .expect("create part");
    let edge = engine
        .create_edge("decomposes", system.id, part.id, default_props(), actor(), authz())
        .expect("create decomposes edge");
    assert_eq!(edge.type_id, "decomposes");
    assert_eq!(engine.node_count().unwrap(), 2);
    assert_eq!(engine.edge_count().unwrap(), 1);
}

#[test]
fn rejects_unknown_node_type() {
    let engine = make_engine();
    let err = engine
        .upsert_node("NotARealType", NodeState::Draft, default_props(), actor(), authz())
        .unwrap_err();
    matches!(err, GraphError::ContractViolation(_));
}

#[test]
fn rejects_unknown_edge_type() {
    let engine = make_engine();
    let a = engine
        .upsert_node("Part", NodeState::Draft, default_props(), actor(), authz())
        .unwrap();
    let b = engine
        .upsert_node("Part", NodeState::Draft, default_props(), actor(), authz())
        .unwrap();
    let err = engine
        .create_edge("not_an_edge", a.id, b.id, default_props(), actor(), authz())
        .unwrap_err();
    matches!(err, GraphError::ContractViolation(_));
}

#[test]
fn enforces_state_transition_allowed_by_catalog() {
    let engine = make_engine();
    let mut props = default_props();
    props.insert("status".to_string(), json!("verified"));
    let req = engine
        .upsert_node("Requirement", NodeState::Draft, props, actor(), authz())
        .expect("create requirement");
    // Draft -> Released is in the catalog's allowed_states.
    let updated = engine
        .change_node_state(req.id, NodeState::Released, actor(), authz())
        .expect("release");
    assert_eq!(updated.state, NodeState::Released);
}

#[test]
fn immutable_node_rejects_state_change() {
    let engine = make_engine();
    // SimulationRun is revisioned + immutable per the catalog.
    let run = engine
        .upsert_node(
            "SimulationRun",
            NodeState::Advisory("running".to_string()),
            default_props(),
            actor(),
            authz(),
        )
        .expect("create run");
    let err = engine
        .change_node_state(run.id, NodeState::Advisory("succeeded".to_string()), actor(), authz())
        .unwrap_err();
    assert!(matches!(err, GraphError::ContractViolation(_)), "got: {err:?}");
}

#[test]
fn tombstone_filters_via_tombstoned_flag() {
    let engine = make_engine();
    let a = engine
        .upsert_node("Part", NodeState::Draft, default_props(), actor(), authz())
        .unwrap();
    let b = engine
        .upsert_node("Part", NodeState::Draft, default_props(), actor(), authz())
        .unwrap();
    engine
        .create_edge("decomposes", a.id, b.id, default_props(), actor(), authz())
        .unwrap();
    engine
        .tombstone_node(b.id, "retired", actor(), authz())
        .unwrap();
    // Tombstoned nodes cannot be the source of new edges.
    let c = engine
        .upsert_node("Part", NodeState::Draft, default_props(), actor(), authz())
        .unwrap();
    let err = engine
        .create_edge("decomposes", b.id, c.id, default_props(), actor(), authz())
        .unwrap_err();
    assert!(matches!(err, GraphError::ContractViolation(_)), "got: {err:?}");
}

#[test]
fn system_release_requires_verification_coverage() {
    // SDP §3.6 invariant 7: a System node cannot transition to Released
    // if any of its `satisfies` edges targets a Requirement without a
    // verifies path.
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let db = tmp.path().join("graph.db");
    let engine = make_engine_at(db);

    let sys = engine
        .upsert_node("System", NodeState::Draft, default_props(), "alice", "allow")
        .expect("system");
    let mut req_props = BTreeMap::new();
    req_props.insert("text".to_string(), serde_json::Value::String("r".to_string()));
    req_props.insert("criticality".to_string(), serde_json::Value::String("major".to_string()));
    req_props.insert(
        "verification_method".to_string(),
        serde_json::Value::String("test".to_string()),
    );
    req_props.insert("status".to_string(), serde_json::Value::String("open".to_string()));
    let req = engine
        .upsert_node("Requirement", NodeState::Draft, req_props, "alice", "allow")
        .expect("requirement");
    engine
        .create_edge("satisfies", sys.id, req.id, default_props(), "alice", "allow")
        .expect("satisfies");

    // Without a verifies path, the System -> Released transition must fail.
    let err = engine
        .change_node_state(sys.id, NodeState::Released, "alice", "allow")
        .unwrap_err();
    assert!(matches!(err, GraphError::ContractViolation(_)), "got: {err:?}");
    assert!(err.to_string().contains("verification coverage"), "got: {err}");

    // Add a verifies path: Evidence -> verifies -> Requirement.
    let mut ev_props = BTreeMap::new();
    ev_props.insert(
        "claim".to_string(),
        serde_json::Value::String("verified by test".to_string()),
    );
    ev_props.insert(
        "verification_status".to_string(),
        serde_json::Value::String("verified".to_string()),
    );
    ev_props.insert(
        "source_refs".to_string(),
        serde_json::Value::Array(vec![serde_json::Value::String("test-1".to_string())]),
    );
    let ev = engine
        .upsert_node("Evidence", NodeState::Released, ev_props, "alice", "allow")
        .expect("evidence");
    // The verifies edge is from the evidence (or test/inspection) TO
    // the requirement it verifies; the invariant walks `out_edges`
    // of the requirement looking for a `verifies` edge (SDP §3.6 inv 7).
    // We model that by adding a reverse edge from the requirement
    // to the evidence as a `verifies` of the evidence (in SDP terms,
    // the requirement references the verifying evidence via the
    // `verifies` edge in the Evidence -> Requirement direction; the
    // invariant treats any `verifies` edge on the requirement as a
    // coverage path).
    engine
        .create_edge("verifies", ev.id, req.id, default_props(), "alice", "allow")
        .expect("verifies");

    // Now the transition must succeed.
    let updated = engine
        .change_node_state(sys.id, NodeState::Released, "alice", "allow")
        .expect("release");
    assert_eq!(updated.state, NodeState::Released);
}

#[test]
fn cardinality_max_out_per_source_enforced() {
    let engine = make_engine();
    // `has_geometry` declares max_out_per_source: 1
    let part = engine
        .upsert_node("Part", NodeState::Draft, default_props(), actor(), authz())
        .unwrap();
    let g1 = engine
        .upsert_node("GeometryRevision", NodeState::Released, default_props(), actor(), authz())
        .unwrap();
    let g2 = engine
        .upsert_node("GeometryRevision", NodeState::Released, default_props(), actor(), authz())
        .unwrap();
    engine
        .create_edge("has_geometry", part.id, g1.id, default_props(), actor(), authz())
        .unwrap();
    let err = engine
        .create_edge("has_geometry", part.id, g2.id, default_props(), actor(), authz())
        .unwrap_err();
    assert!(matches!(err, GraphError::ContractViolation(_)), "got: {err:?}");
}

#[test]
fn aql_filters_by_type_and_property() {
    let engine = make_engine();
    let mut a_props = default_props();
    a_props.insert("status".to_string(), json!("open"));
    engine
        .upsert_node("Requirement", NodeState::Draft, a_props, actor(), authz())
        .unwrap();
    let mut b_props = default_props();
    b_props.insert("status".to_string(), json!("verified"));
    engine
        .upsert_node("Requirement", NodeState::Draft, b_props, actor(), authz())
        .unwrap();
    engine
        .upsert_node("Part", NodeState::Draft, default_props(), actor(), authz())
        .unwrap();
    let open = AqlQuery {
        type_id: Some("Requirement".to_string()),
        property_filters: BTreeMap::from([("status".to_string(), json!("open"))]),
        limit: None,
    };
    let res = open.execute(&engine).unwrap();
    assert_eq!(res.len(), 1);
}

#[test]
fn as_of_replay_restores_history() {
    let engine = make_engine();
    let a = engine
        .upsert_node("Part", NodeState::Draft, default_props(), actor(), authz())
        .unwrap();
    let b = engine
        .upsert_node("Part", NodeState::Draft, default_props(), actor(), authz())
        .unwrap();
    engine
        .change_node_state(b.id, NodeState::Released, actor(), authz())
        .unwrap();
    let after_release = engine.snapshot(SnapshotAt::Current).unwrap();
    assert_eq!(after_release.event_count, 3);
    // Replay to event 1 (just the first upsert).
    let one = engine.snapshot(SnapshotAt::EventOffset(1)).unwrap();
    assert_eq!(one.event_count, 1);
    assert!(one.nodes.contains_key(&a.id));
    assert!(!one.nodes.contains_key(&b.id));
}

#[test]
fn chain_verifies_after_many_mutations() {
    let engine = make_engine();
    let sys = engine
        .upsert_node("System", NodeState::Draft, default_props(), actor(), authz())
        .unwrap();
    for _ in 0..10 {
        let p = engine
            .upsert_node("Part", NodeState::Draft, default_props(), actor(), authz())
            .unwrap();
        engine
            .create_edge("decomposes", sys.id, p.id, default_props(), actor(), authz())
            .unwrap();
    }
    let v = engine.verify_chain().unwrap();
    assert!(v.valid, "chain must verify: {:?}", v.errors);
    // 1 system + 10 parts = 11 node_upserted events; 10 edges = 10 edge_added events.
    assert_eq!(v.event_count, 21);
}

fn default_props() -> Properties {
    BTreeMap::new()
}
