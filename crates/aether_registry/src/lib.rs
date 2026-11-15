// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! Aether capability registry (G1).
//!
//! The registry is the **only** place that resolves capability
//! declarations (SDP §5.3) and produces candidate sets for
//! capability-matching. In G2 it is backed by a signed offline
//! registry; in G1 it is a directory scan of `plugins/*.yaml`
//! manifests. The structural surface — `Requirement`, `Candidate`,
//! `Match` — is stable across phases.

#![deny(unsafe_code)]
#![warn(missing_docs)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod discover;
pub mod match_capability;

pub use discover::{discover_manifests, ManifestSummary};
pub use match_capability::{
    CapabilityRequirement, Candidate, RankedMatches, RegistryError, RegistryResult,
};

/// A plugin's manifest as published per SDP §5.2.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub manifest_version: String,
    pub plugin: PluginInfo,
    pub contracts: Vec<ContractRef>,
    #[serde(default)]
    pub capabilities: serde_json::Value,
    #[serde(default)]
    pub resources: serde_json::Value,
    #[serde(default)]
    pub isolation: serde_json::Value,
    #[serde(default)]
    pub runtime: serde_json::Value,
    #[serde(default)]
    pub airgap: serde_json::Value,
    #[serde(default)]
    pub provenance_template: Option<String>,
    #[serde(default)]
    pub policy: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub vendor: String,
    #[serde(default)]
    pub license_ref: Option<String>,
    #[serde(default)]
    pub signature: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractRef {
    pub id: String,
    pub version: String,
    pub role: String,
}

#[derive(Debug, Error)]
pub enum ManifestError {
    #[error("read {0}: {1}")]
    Read(String, std::io::Error),
    #[error("parse {0}: {1}")]
    Parse(String, serde_yaml::Error),
    #[error("missing required field: {0}")]
    Missing(&'static str),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

pub type ManifestResult<T> = Result<T, ManifestError>;

/// Parse a manifest from a YAML file.
pub fn load_manifest(path: &Path) -> ManifestResult<PluginManifest> {
    let raw = std::fs::read_to_string(path)
        .map_err(|e| ManifestError::Read(path.display().to_string(), e))?;
    let m: PluginManifest = serde_yaml::from_str(&raw)
        .map_err(|e| ManifestError::Parse(path.display().to_string(), e))?;
    if m.plugin.id.is_empty() {
        return Err(ManifestError::Missing("plugin.id"));
    }
    if m.plugin.version.is_empty() {
        return Err(ManifestError::Missing("plugin.version"));
    }
    Ok(m)
}

/// A snapshot of the installed plugin set.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RegistrySnapshot {
    pub manifest_paths: Vec<PathBuf>,
    pub plugins: Vec<PluginManifest>,
    pub by_capability: BTreeMap<String, Vec<String>>, // capability name -> plugin ids
    pub by_contract: BTreeMap<String, Vec<String>>,    // contract id -> plugin ids
}

impl RegistrySnapshot {
    pub fn plugins_for_capability(&self, capability: &str) -> Vec<&PluginManifest> {
        let ids: BTreeSet<&str> = self
            .by_capability
            .get(capability)
            .map(|v| v.iter().map(|s| s.as_str()).collect())
            .unwrap_or_default();
        self.plugins
            .iter()
            .filter(|p| ids.contains(p.plugin.id.as_str()))
            .collect()
    }
}
