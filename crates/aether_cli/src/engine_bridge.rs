// SPDX-License-Identifier: Aether-VPSL-1.0
// SPDX-FileCopyrightText: 2026 The Aether Consortium
//! Engine bridge — wires the CLI commands to the graph engine and
//! provenance subsystem.

#![deny(unsafe_code)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use aether_graph::catalog::{load_edge_types, load_node_types};
use aether_graph::engine::{GraphEngine, SnapshotAt};
use aether_graph::error::GraphError;
use aether_graph::storage::SqliteStorage;
use aether_graph::types::{NodeState, Properties};

use aether_provenance::chain::ProvenanceChain;
use aether_provenance::record::{
    Actor, ArtifactRef, AuthorizationDecision, ProvenanceRecord, ToolRef,
};

/// Resolve the catalog directory from the monorepo root.
pub fn catalog_root(monorepo: &Path) -> Result<(PathBuf, PathBuf)> {
    let nodes = monorepo.join("contracts/graph.model/catalog/nodes");
    let edges = monorepo.join("contracts/graph.model/catalog/edges");
    if !nodes.is_dir() || !edges.is_dir() {
        return Err(anyhow!(
            "catalog not found at {} / {}; ensure you are inside the aether-platform monorepo",
            nodes.display(),
            edges.display()
        ));
    }
    Ok((nodes, edges))
}

/// Construct a `GraphEngine` backed by SQLite.
///
/// State location (Phase 2 — project-local engineering state):
///
/// 1. `AETHER_STATE_DIR` (env override, set => use it): the DB is
///    `<state_dir>/<safe_name>.aether.db`.
/// 2. Projects inside the monorepo keep the historical location
///    `<monorepo>/infra/run/<safe_name>.aether.db` for backward
///    compatibility with existing tests and the dark-build.
/// 3. Standalone projects are self-contained: the DB lives at
///    `<project>/.aether/state/graph.db` so a project directory is a
///    unit that can be exported, replayed, and verified offline.
///
/// The graph catalog (node/edge types) still loads from the monorepo
/// the binary belongs to; see [`find_monorepo_root`].
pub fn open_engine(project_root: &Path) -> Result<(GraphEngine, PathBuf)> {
    let monorepo = find_monorepo_root(project_root);
    let (nodes_dir, edges_dir) = catalog_root(&monorepo)?;
    let node_types = load_node_types(&nodes_dir)
        .map_err(|e| anyhow!("load node types: {e}"))?;
    let edge_types = load_edge_types(&edges_dir)
        .map_err(|e| anyhow!("load edge types: {e}"))?;
    // Per-project DB name (disambiguates the shared infra/run location).
    let safe_name = project_root
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("default")
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect::<String>();
    let monorepo_found =
        monorepo.join("contracts").is_dir() && monorepo.join("Cargo.toml").is_file();
    let in_monorepo = monorepo_found && project_root.starts_with(&monorepo);
    let db_path = if let Ok(state_dir) = std::env::var("AETHER_STATE_DIR") {
        PathBuf::from(state_dir).join(format!("{safe_name}.aether.db"))
    } else if in_monorepo {
        monorepo
            .join("infra/run")
            .join(format!("{safe_name}.aether.db"))
    } else {
        project_root.join(".aether/state/graph.db")
    };
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let storage = SqliteStorage::open(&db_path, aether_graph::storage::OpenMode::ReadWrite)
        .map_err(|e| anyhow!("open sqlite: {e}"))?;
    Ok((
        GraphEngine::new(Box::new(storage), node_types, edge_types),
        db_path,
    ))
}

/// Locate the monorepo root by walking up until `contracts/` is found.
///
/// Resolution order: the `AETHER_MONOREPO` env override (lets a
/// standalone project resolve the catalog against an explicitly
/// installed platform), then the executable's ancestors (the usual
/// `target/{debug,release}/aether` case), then the filesystem walk
/// from `start`. Returns `start` unchanged as a last resort; callers
/// re-check the monorepo predicate and produce actionable errors.
pub fn find_monorepo_root(start: &Path) -> PathBuf {
    if let Ok(root) = std::env::var("AETHER_MONOREPO") {
        let p = PathBuf::from(root);
        if p.join("contracts").is_dir() {
            return p;
        }
    }
    let mut here = if start.is_file() {
        start.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| start.to_path_buf())
    } else {
        start.to_path_buf()
    };
    // First, try the executable's location: `target/debug/aether` lives
    // in the monorepo's `target/` directory, so `parent.parent.parent` is
    // the monorepo root.
    if let Ok(exe) = std::env::current_exe() {
        let mut ancestor = exe.clone();
        for _ in 0..6 {
            if ancestor.join("contracts").is_dir() && ancestor.join("Cargo.toml").is_file() {
                return ancestor;
            }
            if !ancestor.pop() {
                break;
            }
        }
    }
    loop {
        if here.join("contracts").is_dir() && here.join("Cargo.toml").is_file() {
            return here;
        }
        if !here.pop() {
            return start.to_path_buf();
        }
    }
}

/// Result of `aether graph query` (G1 AQL subset).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQueryResult {
    pub nodes: Vec<Value>,
    pub count: usize,
    pub chain_head: Option<String>,
    pub event_count: u64,
}

/// Result of `aether graph as-of` (G1 AS-OF replay).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphAsOfResult {
    pub offset: u64,
    pub event_count: u64,
    pub node_count: usize,
    pub edge_count: usize,
    pub chain_head: Option<String>,
    pub nodes: Vec<Value>,
    pub edges: Vec<Value>,
}

/// Result of `aether requirement add`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequirementAdded {
    pub node_id: Uuid,
    pub type_id: String,
    pub state: String,
    pub properties: Value,
}

/// Result of `aether graph chain` (audit chain verification).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainStatus {
    pub valid: bool,
    pub event_count: u64,
    pub node_count: u64,
    pub edge_count: u64,
    pub head: Option<String>,
    pub errors: Vec<String>,
}

/// Convert a node to its JSON representation for `--json` output.
pub fn node_to_value(node: &aether_graph::types::Node) -> Value {
    let mut v = serde_json::Map::new();
    v.insert("id".to_string(), Value::String(node.id.to_string()));
    v.insert("type_id".to_string(), Value::String(node.type_id.clone()));
    v.insert("state".to_string(), Value::String(node.state.as_str().to_string()));
    v.insert("revisioned".to_string(), Value::Bool(node.revisioned));
    v.insert("immutable".to_string(), Value::Bool(node.immutable));
    v.insert("tombstoned".to_string(), Value::Bool(node.tombstoned));
    let props: Value = serde_json::to_value(&node.properties).unwrap_or(Value::Null);
    v.insert("properties".to_string(), props);
    Value::Object(v)
}

/// Convert an edge to its JSON representation for `--json` output.
pub fn edge_to_value(edge: &aether_graph::types::Edge) -> Value {
    let mut v = serde_json::Map::new();
    v.insert("id".to_string(), Value::String(edge.id.to_string()));
    v.insert("type_id".to_string(), Value::String(edge.type_id.clone()));
    v.insert("source".to_string(), Value::String(edge.source.to_string()));
    v.insert("target".to_string(), Value::String(edge.target.to_string()));
    let props: Value = serde_json::to_value(&edge.properties).unwrap_or(Value::Null);
    v.insert("properties".to_string(), props);
    Value::Object(v)
}

/// `aether graph query` (G1 AQL subset): match by type and property filters.
pub fn run_query(
    engine: &GraphEngine,
    type_id: Option<&str>,
    property_filters: &BTreeMap<String, Value>,
    limit: Option<u32>,
) -> Result<GraphQueryResult> {
    let mut nodes: Vec<_> = if let Some(t) = type_id {
        engine
            .nodes_of_type(t)
            .map_err(|e| anyhow!("graph query: {e}"))?
    } else {
        let mut all = Vec::new();
        for type_id in [
            "Requirement",
            "System",
            "Part",
            "GeometryRevision",
            "SimulationStudy",
            "SimulationRun",
            "Result",
            "Evidence",
            "Decision",
        ] {
            if let Ok(found) = engine.nodes_of_type(type_id) {
                all.extend(found);
            }
        }
        all
    };
    if !property_filters.is_empty() {
        nodes.retain(|n| {
            property_filters
                .iter()
                .all(|(k, v)| n.properties.get(k) == Some(v))
        });
    }
    if let Some(limit) = limit {
        nodes.truncate(limit as usize);
    }
    let chain_head = engine
        .verify_chain()
        .ok()
        .and_then(|v| if v.valid { Some(v.event_count.to_string()) } else { None });
    let event_count = engine.event_count().unwrap_or(0);
    let count = nodes.len();
    let values: Vec<Value> = nodes.iter().map(node_to_value).collect();
    Ok(GraphQueryResult {
        nodes: values,
        count,
        chain_head,
        event_count,
    })
}

/// `aether graph as-of <offset>` — replay to the Nth event and project.
pub fn run_as_of(engine: &GraphEngine, offset: u64) -> Result<GraphAsOfResult> {
    let snap = engine
        .snapshot(SnapshotAt::EventOffset(offset))
        .map_err(|e| anyhow!("as-of: {e}"))?;
    let nodes: Vec<Value> = snap.nodes.values().map(node_to_value).collect();
    let edges: Vec<Value> = snap.edges.values().map(edge_to_value).collect();
    Ok(GraphAsOfResult {
        offset,
        event_count: snap.event_count,
        node_count: snap.nodes.len(),
        edge_count: snap.edges.len(),
        chain_head: snap.chain_head,
        nodes,
        edges,
    })
}

/// `aether requirement add` — create a Requirement node.
pub fn run_requirement_add(
    engine: &GraphEngine,
    text: &str,
    verification_method: &str,
    criticality: &str,
    actor: &str,
) -> Result<RequirementAdded> {
    let mut properties = Properties::new();
    properties.insert("text".to_string(), Value::String(text.to_string()));
    properties.insert(
        "verification_method".to_string(),
        Value::String(verification_method.to_string()),
    );
    properties.insert(
        "criticality".to_string(),
        Value::String(criticality.to_string()),
    );
    properties.insert("status".to_string(), Value::String("open".to_string()));
    let node = engine
        .upsert_node("Requirement", NodeState::Draft, properties, actor, "allow")
        .map_err(|e| anyhow!("requirement add: {e}"))?;
    Ok(RequirementAdded {
        node_id: node.id,
        type_id: node.type_id,
        state: node.state.as_str().to_string(),
        properties: serde_json::to_value(&node.properties)?,
    })
}

/// `aether graph chain` — verify the event chain and emit a status.
pub fn run_chain_status(engine: &GraphEngine) -> Result<ChainStatus> {
    let v = engine
        .verify_chain()
        .map_err(|e| anyhow!("chain verify: {e}"))?;
    let node_count = engine.node_count().unwrap_or(0);
    let edge_count = engine.edge_count().unwrap_or(0);
    let head = engine
        .storage_head()
        .map_err(|e| anyhow!("chain head: {e}"))?;
    Ok(ChainStatus {
        valid: v.valid,
        event_count: v.event_count,
        node_count,
        edge_count,
        head,
        errors: v.errors,
    })
}

/// Build an empty in-memory provenance chain for the current run.
pub fn new_provenance_chain() -> ProvenanceChain {
    ProvenanceChain::new()
}

/// Append a synthetic `provenance record` produced by the CLI itself
/// (e.g. for `aether graph mutate`). The record's actor is the CLI
/// identity; the tool is the CLI build.
pub fn append_cli_record(
    chain: &mut ProvenanceChain,
    actor: &str,
    inputs: Vec<ArtifactRef>,
    outputs: Vec<ArtifactRef>,
) -> Result<()> {
    let tool = ToolRef {
        plugin_id: "org.aether.cli".to_string(),
        plugin_version: env!("CARGO_PKG_VERSION").to_string(),
        contract_versions: Default::default(),
        conformance_report: None,
    };
    let authz = AuthorizationDecision {
        decision: "allow".to_string(),
        policy_ref: None,
        rationale: Some("cli operator action".to_string()),
    };
    let record_actor = Actor {
        id: actor.to_string(),
        role: Some("cli-operator".to_string()),
        plugin_instance: None,
    };
    let prev = chain.head().map(|s| s.to_string());
    let record = ProvenanceRecord::new(record_actor, authz, tool, inputs, outputs, prev);
    chain
        .append(record)
        .map_err(|e| anyhow!("append provenance record: {e}"))?;
    Ok(())
}

// Suppress unused warning for the Arc import; it's used by future
// concurrent CLI modes.
#[allow(dead_code)]
fn _arc_used(_: Arc<()>) {}
