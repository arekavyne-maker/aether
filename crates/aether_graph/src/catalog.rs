// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! Schema catalog loader.
//!
//! Loads the `aether.graph.model` node-type and edge-type catalogs from
//! `contracts/graph.model/catalog/{nodes,edges}/*.json` into the engine.
//! The catalog is **data**; the engine's generic mechanics (validation,
//! revisioning, indexing) work over any registered type (SDP §2 I-2).

#![deny(unsafe_code)]

use std::collections::BTreeMap;
use std::path::Path;

use crate::error::{GraphError, GraphResult};
use crate::types::{EdgeTypeSpec, NodeTypeSpec};

/// Load every node-type JSON file from the given directory.
pub fn load_node_types(dir: &Path) -> GraphResult<BTreeMap<String, NodeTypeSpec>> {
    let mut out = BTreeMap::new();
    if !dir.is_dir() {
        return Err(GraphError::ResourceUnavailable(format!(
            "node-type catalog directory not found: {}",
            dir.display()
        )));
    }
    for entry in std::fs::read_dir(dir)
        .map_err(|e| GraphError::ResourceUnavailable(format!("read_dir {}: {e}", dir.display())))?
    {
        let entry = entry.map_err(|e| GraphError::ResourceUnavailable(e.to_string()))?;
        let path = entry.path();
        if !path.is_file() || path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let raw = std::fs::read_to_string(&path)
            .map_err(|e| GraphError::ResourceUnavailable(format!("read {}: {e}", path.display())))?;
        let spec: NodeTypeSpec = serde_json::from_str(&raw)
            .map_err(|e| GraphError::ContractViolation(format!("parse {}: {e}", path.display())))?;
        if out.insert(spec.type_id.clone(), spec).is_some() {
            return Err(GraphError::ContractViolation(format!(
                "duplicate node type in catalog: {}",
                path.display()
            )));
        }
    }
    Ok(out)
}

/// Load every edge-type JSON file from the given directory.
pub fn load_edge_types(dir: &Path) -> GraphResult<BTreeMap<String, EdgeTypeSpec>> {
    let mut out = BTreeMap::new();
    if !dir.is_dir() {
        return Err(GraphError::ResourceUnavailable(format!(
            "edge-type catalog directory not found: {}",
            dir.display()
        )));
    }
    for entry in std::fs::read_dir(dir)
        .map_err(|e| GraphError::ResourceUnavailable(format!("read_dir {}: {e}", dir.display())))?
    {
        let entry = entry.map_err(|e| GraphError::ResourceUnavailable(e.to_string()))?;
        let path = entry.path();
        if !path.is_file() || path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let raw = std::fs::read_to_string(&path)
            .map_err(|e| GraphError::ResourceUnavailable(format!("read {}: {e}", path.display())))?;
        let spec: EdgeTypeSpec = serde_json::from_str(&raw)
            .map_err(|e| GraphError::ContractViolation(format!("parse {}: {e}", path.display())))?;
        if out.insert(spec.type_id.clone(), spec).is_some() {
            return Err(GraphError::ContractViolation(format!(
                "duplicate edge type in catalog: {}",
                path.display()
            )));
        }
    }
    Ok(out)
}
