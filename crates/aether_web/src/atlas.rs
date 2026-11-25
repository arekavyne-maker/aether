// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! Project atlas aggregation.
//!
//! Every field is produced by the canonical CLI. This module never reads
//! project state directly and never invents nodes, edges or coverage; it
//! runs a small set of read-only commands and records their provenance so
//! the browser can show exactly what it is rendering.

#![deny(unsafe_code)]
#![warn(missing_docs)]

use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::api::AppState;
use crate::cli::{CliRunner, CommandRequest, CommandState};

/// Maximum nodes returned by the atlas before truncation.
pub const MAX_NODES: usize = 5000;
/// Maximum edges returned by the atlas before truncation.
pub const MAX_EDGES: usize = 10000;

/// Provenance for one CLI call behind an atlas response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AtlasSource {
    /// Human-readable argv, e.g. `aether graph as-of 3 --json --offline`.
    pub command: String,
    /// Whether the underlying CLI exit code was zero.
    pub ok: bool,
    /// Exit code returned by the CLI.
    pub exit_code: i32,
    /// Wall-clock duration in milliseconds.
    pub duration_ms: u128,
    /// Whether the value came from the short-lived response cache.
    pub cached: bool,
    /// stderr when the command failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Project identity and context.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AtlasProject {
    /// Absolute project root used for child CLI processes.
    pub root: String,
    /// Whether `.aether/project.toml` exists.
    pub initialized: bool,
    /// Output of `aether discover project-context`.
    pub context: Value,
}

/// The event-offset graph snapshot.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AtlasGraph {
    /// Event offset rendered.
    pub offset: u64,
    /// Events in the chain at this point.
    pub event_count: u64,
    /// Node count reported by the CLI.
    pub node_count: u64,
    /// Edge count reported by the CLI.
    pub edge_count: u64,
    /// Number of nodes included in `nodes`.
    pub nodes_returned: usize,
    /// Number of edges included in `edges`.
    pub edges_returned: usize,
    /// True when the response was capped.
    pub truncated: bool,
    /// Graph nodes, verbatim from the CLI.
    pub nodes: Vec<Value>,
    /// Graph edges, verbatim from the CLI.
    pub edges: Vec<Value>,
    /// Chain head reported by the snapshot.
    pub chain_head: Option<String>,
}

/// Graph type catalog.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AtlasSchema {
    /// Registered node type names.
    pub node_types: Vec<String>,
    /// Registered edge type names.
    pub edge_types: Vec<String>,
    /// Contract package count.
    pub contract_count: u64,
}

/// The complete aggregate consumed by the Atlas UI.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AtlasResponse {
    /// Project identity.
    pub project: AtlasProject,
    /// Output of `aether status`.
    pub status: Value,
    /// Output of `aether graph chain`.
    pub chain: Value,
    /// Graph snapshot at the selected offset.
    pub graph: AtlasGraph,
    /// Node/edge type catalog.
    pub schema: AtlasSchema,
    /// Output of `aether validation report`, or null when unavailable.
    pub coverage: Value,
    /// Provenance for each CLI call.
    pub sources: Vec<AtlasSource>,
}

fn strings(value: &Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn display_argv(command: &[String], flags: &[String]) -> String {
    let parts = command.iter().chain(flags.iter()).map(String::as_str);
    std::iter::once("aether").chain(parts).collect::<Vec<_>>().join(" ")
}

/// Run one read-only CLI command, using the shared cache when possible.
async fn fetch(
    state: &AppState,
    key: String,
    ttl: Duration,
    command: Vec<String>,
    flags: Vec<String>,
) -> (Value, AtlasSource) {
    let label = display_argv(&command, &flags);
    if let Some(value) = state.cached(&key, ttl).await {
        return (
            value,
            AtlasSource {
                command: label,
                ok: true,
                exit_code: 0,
                duration_ms: 0,
                cached: true,
                error: None,
            },
        );
    }
    match state.runner.run(&CommandRequest { command, flags }).await {
        Ok(response) => {
            let ok = response.state == CommandState::Ok;
            if ok {
                state.remember(&key, response.value.clone()).await;
            }
            (
                if ok { response.value } else { Value::Null },
                AtlasSource {
                    command: label,
                    ok,
                    exit_code: response.exit_code,
                    duration_ms: response.duration_ms,
                    cached: false,
                    error: if ok {
                        None
                    } else {
                        Some(response.stderr.trim().to_string())
                    },
                },
            )
        }
        Err(error) => (
            Value::Null,
            AtlasSource {
                command: label,
                ok: false,
                exit_code: -1,
                duration_ms: 0,
                cached: false,
                error: Some(error.to_string()),
            },
        ),
    }
}

fn project_root(runner: &CliRunner) -> PathBuf {
    runner
        .project()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
}

/// Build the atlas aggregate.
///
/// `offset` selects an event snapshot; when omitted, the current chain head
/// is used. A requested offset beyond the chain is clamped by the CLI and
/// reported as a failed source rather than silently altered here.
pub async fn build(state: &AppState, offset: Option<u64>) -> AtlasResponse {
    let root = project_root(&state.runner);
    let root_text = root.to_string_lossy().to_string();
    let project_flags = || vec!["--project".into(), root_text.clone()];

    let (chain, chain_source) = fetch(
        state,
        format!("atlas:chain:{root_text}"),
        Duration::from_secs(5),
        vec!["graph".into(), "chain".into()],
        {
            let mut flags = project_flags();
            flags.extend(["--json".into(), "--offline".into()]);
            flags
        },
    )
    .await;

    let event_count = chain.get("event_count").and_then(Value::as_u64).unwrap_or(0);
    let selected = offset.unwrap_or(event_count);
    let mut graph_flags = project_flags();
    graph_flags.extend([
        selected.to_string(),
        "--json".into(),
        "--offline".into(),
    ]);
    let (snapshot, graph_source) = fetch(
        state,
        format!("atlas:graph:{root_text}:{selected}"),
        Duration::from_secs(5),
        vec!["graph".into(), "as-of".into()],
        graph_flags,
    )
    .await;

    let (status, status_source) = fetch(
        state,
        format!("atlas:status:{root_text}"),
        Duration::from_secs(5),
        vec!["status".into()],
        vec!["--json".into(), "--offline".into()],
    )
    .await;

    let (schema_list, schema_source) = fetch(
        state,
        "atlas:schema".to_string(),
        Duration::from_secs(300),
        vec!["schema".into(), "list".into()],
        vec!["--json".into(), "--offline".into()],
    )
    .await;

    let (context, context_source) = fetch(
        state,
        format!("atlas:context:{root_text}"),
        Duration::from_secs(30),
        vec!["discover".into(), "project-context".into()],
        {
            let mut flags = project_flags();
            flags.extend(["--json".into(), "--offline".into()]);
            flags
        },
    )
    .await;

    let (coverage, coverage_source) = fetch(
        state,
        format!("atlas:coverage:{root_text}"),
        Duration::from_secs(10),
        vec!["validation".into(), "report".into()],
        {
            let mut flags = project_flags();
            flags.extend(["--json".into(), "--offline".into()]);
            flags
        },
    )
    .await;

    let node_values = snapshot
        .get("nodes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let edge_values = snapshot
        .get("edges")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let truncated = node_values.len() > MAX_NODES || edge_values.len() > MAX_EDGES;
    let nodes: Vec<Value> = node_values.into_iter().take(MAX_NODES).collect();
    let edges: Vec<Value> = edge_values.into_iter().take(MAX_EDGES).collect();

    let graph = AtlasGraph {
        offset: selected,
        event_count: snapshot
            .get("event_count")
            .and_then(Value::as_u64)
            .unwrap_or(event_count),
        node_count: snapshot
            .get("node_count")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        edge_count: snapshot
            .get("edge_count")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        nodes_returned: nodes.len(),
        edges_returned: edges.len(),
        truncated,
        nodes,
        edges,
        chain_head: snapshot
            .get("chain_head")
            .and_then(Value::as_str)
            .map(str::to_string),
    };

    let schema = AtlasSchema {
        node_types: strings(&schema_list, "graph_node_types"),
        edge_types: strings(&schema_list, "graph_edge_types"),
        contract_count: schema_list
            .get("contract_count")
            .and_then(Value::as_u64)
            .unwrap_or(0),
    };

    AtlasResponse {
        project: AtlasProject {
            root: root_text,
            initialized: root.join(".aether").join("project.toml").is_file(),
            context,
        },
        status,
        chain,
        graph,
        schema,
        coverage,
        sources: vec![
            context_source,
            status_source,
            chain_source,
            graph_source,
            schema_source,
            coverage_source,
        ],
    }
}

/// Build only the graph portion of the atlas, used by the scrubber.
pub async fn graph(state: &AppState, offset: Option<u64>) -> (AtlasGraph, AtlasSource) {
    let root = project_root(&state.runner);
    let root_text = root.to_string_lossy().to_string();
    let (chain, chain_source) = fetch(
        state,
        format!("atlas:chain:{root_text}"),
        Duration::from_secs(5),
        vec!["graph".into(), "chain".into()],
        vec![
            "--project".into(),
            root_text.clone(),
            "--json".into(),
            "--offline".into(),
        ],
    )
    .await;
    let event_count = chain.get("event_count").and_then(Value::as_u64).unwrap_or(0);
    let selected = offset.unwrap_or(event_count);
    let (snapshot, source) = fetch(
        state,
        format!("atlas:graph:{root_text}:{selected}"),
        Duration::from_secs(5),
        vec!["graph".into(), "as-of".into()],
        vec![
            "--project".into(),
            root_text,
            selected.to_string(),
            "--json".into(),
            "--offline".into(),
        ],
    )
    .await;

    let mut node_values = snapshot
        .get("nodes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut edge_values = snapshot
        .get("edges")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let truncated = node_values.len() > MAX_NODES || edge_values.len() > MAX_EDGES;
    node_values.truncate(MAX_NODES);
    edge_values.truncate(MAX_EDGES);

    let graph = AtlasGraph {
        offset: selected,
        event_count: snapshot
            .get("event_count")
            .and_then(Value::as_u64)
            .unwrap_or(event_count),
        node_count: snapshot
            .get("node_count")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        edge_count: snapshot
            .get("edge_count")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        nodes_returned: node_values.len(),
        edges_returned: edge_values.len(),
        truncated,
        nodes: node_values,
        edges: edge_values,
        chain_head: snapshot
            .get("chain_head")
            .and_then(Value::as_str)
            .map(str::to_string),
    };
    let _ = chain_source;
    (graph, source)
}

/// Build only the schema catalog.
pub async fn schema(state: &AppState) -> (AtlasSchema, AtlasSource) {
    let (value, source) = fetch(
        state,
        "atlas:schema".to_string(),
        Duration::from_secs(300),
        vec!["schema".into(), "list".into()],
        vec!["--json".into(), "--offline".into()],
    )
    .await;
    (
        AtlasSchema {
            node_types: strings(&value, "graph_node_types"),
            edge_types: strings(&value, "graph_edge_types"),
            contract_count: value
                .get("contract_count")
                .and_then(Value::as_u64)
                .unwrap_or(0),
        },
        source,
    )
}

/// Health report from `aether doctor`.
pub async fn health(state: &AppState) -> (Value, AtlasSource) {
    fetch(
        state,
        "atlas:health".to_string(),
        Duration::from_secs(30),
        vec!["doctor".into()],
        vec!["--json".into(), "--offline".into()],
    )
    .await
}

/// Convenience: an empty graph response for fail-closed UI states.
pub fn empty_graph(offset: u64) -> AtlasGraph {
    AtlasGraph {
        offset,
        event_count: 0,
        node_count: 0,
        edge_count: 0,
        nodes_returned: 0,
        edges_returned: 0,
        truncated: false,
        nodes: Vec::new(),
        edges: Vec::new(),
        chain_head: None,
    }
}

/// Convert a graph into a compact shape used by the SVG renderer.
pub fn graph_shape(graph: &AtlasGraph) -> Value {
    json!({
        "offset": graph.offset,
        "node_count": graph.node_count,
        "edge_count": graph.edge_count,
        "truncated": graph.truncated,
        "nodes": graph.nodes,
        "edges": graph.edges,
    })
}
