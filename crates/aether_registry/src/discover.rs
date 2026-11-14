// SPDX-License-Identifier: Aether-VPSL-1.0
// SPDX-FileCopyrightText: 2026 The Aether Consortium
//! Plugin manifest discovery (G1).
//!
//! Scans a directory for `*.yaml` / `*.yml` files, parses them as plugin
//! manifests, and builds a `RegistrySnapshot`. Production G2 backs this
//! with the signed offline registry; G1 uses a directory scan as the
//! substrate for `aether plugin list`.

#![deny(unsafe_code)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{load_manifest, RegistrySnapshot};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestSummary {
    pub path: PathBuf,
    pub plugin_id: String,
    pub plugin_name: String,
    pub plugin_version: String,
    pub vendor: String,
    pub contracts: Vec<String>,
}

/// Discover every plugin manifest in a directory tree. Phase 0/G1
/// implementation: scan `*.yaml` / `*.yml` files; ignore non-manifests.
pub fn discover_manifests(plugins_dir: &Path) -> Vec<ManifestSummary> {
    let mut out = Vec::new();
    if !plugins_dir.is_dir() {
        return out;
    }
    let entries = match std::fs::read_dir(plugins_dir) {
        Ok(e) => e,
        Err(_) => return out,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
        if ext != "yaml" && ext != "yml" {
            continue;
        }
        if let Ok(manifest) = load_manifest(&path) {
            out.push(ManifestSummary {
                path: path.clone(),
                plugin_id: manifest.plugin.id.clone(),
                plugin_name: manifest.plugin.name.clone(),
                plugin_version: manifest.plugin.version.clone(),
                vendor: manifest.plugin.vendor.clone(),
                contracts: manifest.contracts.iter().map(|c| c.id.clone()).collect(),
            });
        }
    }
    out
}

/// Build a `RegistrySnapshot` from a directory.
pub fn build_snapshot(plugins_dir: &Path) -> RegistrySnapshot {
    let mut snap = RegistrySnapshot::default();
    let entries = match std::fs::read_dir(plugins_dir) {
        Ok(e) => e,
        Err(_) => return snap,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
        if ext != "yaml" && ext != "yml" {
            continue;
        }
        if let Ok(m) = load_manifest(&path) {
            snap.manifest_paths.push(path);
            // Index capabilities
            if let Some(caps) = m.capabilities.as_object() {
                for (k, v) in caps {
                    if v.as_bool() == Some(true) {
                        snap.by_capability
                            .entry(k.clone())
                            .or_default()
                            .push(m.plugin.id.clone());
                    }
                }
            }
            // Index contracts
            for c in &m.contracts {
                snap.by_contract
                    .entry(c.id.clone())
                    .or_default()
                    .push(m.plugin.id.clone());
            }
            snap.plugins.push(m);
        }
    }
    snap
}

/// Empty snapshot for tests / dry-runs.
pub fn empty_snapshot() -> RegistrySnapshot {
    RegistrySnapshot::default()
}

/// Helper: number of plugins in a snapshot.
pub fn count(snap: &RegistrySnapshot) -> usize {
    snap.plugins.len()
}

/// Helper: build a snapshot from a map of (plugin_id -> (version, caps))
/// — used by tests to skip disk I/O.
pub fn snapshot_from(plugins: Vec<(&str, &str, BTreeMap<String, bool>)>) -> RegistrySnapshot {
    let mut snap = RegistrySnapshot::default();
    for (id, version, caps) in plugins {
        let id = id.to_string();
        let version = version.to_string();
        let mut caps_json = serde_json::Map::new();
        for (k, v) in caps {
            caps_json.insert(k, serde_json::Value::Bool(v));
        }
        snap.plugins.push(crate::PluginManifest {
            manifest_version: "1.2".to_string(),
            plugin: crate::PluginInfo {
                id: id.clone(),
                name: id.clone(),
                version,
                vendor: "test".to_string(),
                license_ref: None,
                signature: None,
            },
            contracts: vec![crate::ContractRef {
                id: "aether.test.plugin".to_string(),
                version: "1.0.0".to_string(),
                role: "implements".to_string(),
            }],
            capabilities: serde_json::Value::Object(caps_json.clone()),
            resources: serde_json::json!({}),
            isolation: serde_json::json!({}),
            runtime: serde_json::json!({}),
            airgap: serde_json::json!({}),
            provenance_template: None,
            policy: serde_json::json!({}),
        });
        for (k, v) in caps_json {
            if v.as_bool() == Some(true) {
                snap.by_capability
                    .entry(k)
                    .or_default()
                    .push(id.clone());
            }
        }
    }
    snap
}
