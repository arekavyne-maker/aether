// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! CLI command implementations (G1 subset).
//!
//! The G1 command surface follows SDP §15.2:
//! - `aether init`                  — initialize a project
//! - `aether project {status|open}` — project info
//! - `aether graph {query|mutate|as-of|chain}` — graph operations
//! - `aether requirement {add|link|verify-status}` — requirement management
//! - `aether plugin {list}`         — list installed plugins
//! - `aether package {create|verify}` — package operations
//! - `aether verify-tuf`            — TUF bundle verification (Phase 0)
//! - `aether dark-build-check`      — dark-build smoke (Phase 0)

#![deny(unsafe_code)]
#![warn(missing_docs)]

use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::Value;

use crate::engine_bridge;

/// Output mode (SDP §15.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputMode {
    /// Human-readable text (default).
    Text,
    /// Stable machine-readable JSON.
    Json,
}

/// Emit a command result to stdout. In text mode, the result is
/// pretty-printed; in JSON mode, the result is serialized as JSON.
pub fn emit<T: Serialize>(mode: OutputMode, value: &T) -> Result<()> {
    match mode {
        OutputMode::Text => {
            // For text mode, fall back to JSON; full pretty-printers land
            // in G1.5 (e.g. table formatter for queries).
            let json = serde_json::to_string_pretty(value)?;
            println!("{json}");
        }
        OutputMode::Json => {
            let json = serde_json::to_string(value)?;
            println!("{json}");
        }
    }
    Ok(())
}

/// `aether init` — initialize a project directory with default layout.
pub fn cmd_init(path: PathBuf, mode: OutputMode) -> Result<()> {
    let root = engine_bridge::find_monorepo_root(&path);
    let project = if path.is_absolute() {
        path
    } else {
        std::env::current_dir()?.join(&path)
    };
    let dirs = [
        project.join("graph"),
        project.join("requirements"),
        project.join("workflows"),
        project.join("data"),
        project.join("provenance"),
    ];
    for d in &dirs {
        std::fs::create_dir_all(d)
            .with_context(|| format!("create {}", d.display()))?;
    }
    // Aether project marker (so `aether project open` knows it's here).
    let marker = project.join(".aether/project.toml");
    if let Some(parent) = marker.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let manifest = serde_json::json!({
        "project": {
            "name": project.file_name().and_then(|s| s.to_str()).unwrap_or("unnamed"),
            "monorepo_root": root.display().to_string(),
            "created_at": chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        }
    });
    std::fs::write(&marker, serde_json::to_string_pretty(&manifest)?)?;
    let summary = serde_json::json!({
        "initialized": true,
        "path": project.display().to_string(),
        "marker": marker.display().to_string(),
        "monorepo_root": root.display().to_string(),
    });
    emit(mode, &summary)
}

/// `aether init --profile <p>` — initialize a project with the layout,
/// scaffolds, and profile record of the given capability profile.
/// Profiles (Phase 2):
///   core      — kernel/graph/workflow/provenance scaffold only
///   research  — core + experiment/benchmark scaffolds
///   aerospace — research + aerostructural workflows (wing baseline)
///   developer — aerospace + contract/plugin dev notes
/// The default is `aerospace`: the platform's flagship domain.
pub fn cmd_init_profile(path: PathBuf, profile: &str, mode: OutputMode) -> Result<()> {
    const PROFILES: &[&str] = &["core", "research", "aerospace", "developer"];
    if !PROFILES.contains(&profile) {
        return Err(anyhow::anyhow!(
            "unknown profile {profile:?}; expected one of: {}",
            PROFILES.join(", ")
        ));
    }
    // The base layout is shared; profile adds scaffolds on top.
    cmd_init(path.clone(), mode)?;
    let project = if path.is_absolute() {
        path
    } else {
        std::env::current_dir()?.join(&path)
    };

    // Record the profile in the project marker (JSON body, .toml name —
    // historical; kept for backward compatibility).
    let marker = project.join(".aether/project.toml");
    let mut v: Value = serde_json::from_str(&std::fs::read_to_string(&marker)?)?;
    v["project"]["profile"] = Value::String(profile.to_string());
    v["project"]["profiles_available"] = serde_json::json!(PROFILES);
    std::fs::write(&marker, serde_json::to_string_pretty(&v)?)?;

    // Profile scaffolds. Later profiles include earlier ones.
    match profile {
        "research" | "aerospace" | "developer" => {
            let wf = project.join("workflows");
            std::fs::create_dir_all(&wf)?;
            std::fs::write(
                wf.join("wing-baseline.awl.yaml"),
                crate::spine::ASSET_PROJECT_BASELINE,
            )?;
            std::fs::write(
                project.join("experiment.yaml"),
                crate::spine::ASSET_EXPERIMENT_TEMPLATE,
            )?;
        }
        _ => {}
    }
    match profile {
        "aerospace" | "developer" => {
            std::fs::create_dir_all(project.join("requirements"))?;
            let req = project.join("requirements/REQ-0001-wing-limit-load.yaml");
            if !req.exists() {
                std::fs::write(
                    &req,
                    "id: REQ-0001\ntext: >-\n  The wing shall sustain the 2.5g limit load without structural\n  failure at the wing-box.\nverification_method: analysis\ncriticality: catastrophic\nstatus: open\n",
                )?;
            }
        }
        _ => {}
    }
    match profile {
        "developer" => {
            std::fs::write(
                project.join("DEVELOPER.md"),
                "# Developer notes\n\n- Contract schemas: `contracts/` in the platform tree\n- Plugin authoring guide: `docs/plugin-author/` in the platform tree\n- Run the fitness pipeline: `python3 infra/fitness/run.py`\n- This project was initialized with the `developer` profile.\n",
            )?;
        }
        _ => {}
    }

    if mode == OutputMode::Text {
        // cmd_init already emitted its summary; add the profile line.
        println!("profile: {profile}");
    }
    Ok(())
}

/// `aether project status` — show project info.
pub fn cmd_project_status(path: PathBuf, mode: OutputMode) -> Result<()> {
    let marker = path.join(".aether/project.toml");
    if !marker.is_file() {
        return Err(anyhow::anyhow!(
            "no Aether project at {} (missing .aether/project.toml)",
            path.display()
        ));
    }
    let raw = std::fs::read_to_string(&marker)?;
    let v: Value = serde_json::from_str(&raw)?;
    emit(mode, &v)
}

/// `aether graph query` — AQL subset.
pub fn cmd_graph_query(
    project: PathBuf,
    type_id: Option<String>,
    property: Vec<String>,
    limit: Option<u32>,
    mode: OutputMode,
) -> Result<()> {
    let (engine, _) = engine_bridge::open_engine(&project)?;
    let mut filters = std::collections::BTreeMap::new();
    for kv in property {
        if let Some((k, v)) = kv.split_once('=') {
            let parsed: Value = serde_json::from_str(v)
                .unwrap_or(Value::String(v.to_string()));
            filters.insert(k.to_string(), parsed);
        } else {
            return Err(anyhow::anyhow!(
                "property filter must be key=value (got {kv:?})"
            ));
        }
    }
    let result = engine_bridge::run_query(&engine, type_id.as_deref(), &filters, limit)?;
    emit(mode, &result)
}

/// `aether graph as-of` — replay to a given event offset.
pub fn cmd_graph_as_of(project: PathBuf, offset: u64, mode: OutputMode) -> Result<()> {
    let (engine, _) = engine_bridge::open_engine(&project)?;
    let result = engine_bridge::run_as_of(&engine, offset)?;
    emit(mode, &result)
}

/// `aether graph chain` — verify the event chain.
pub fn cmd_graph_chain(project: PathBuf, mode: OutputMode) -> Result<()> {
    let (engine, _) = engine_bridge::open_engine(&project)?;
    let status = engine_bridge::run_chain_status(&engine)?;
    emit(mode, &status)
}

/// `aether requirement add` — create a new Requirement node.
pub fn cmd_requirement_add(
    project: PathBuf,
    text: String,
    verification_method: String,
    criticality: String,
    actor: String,
    mode: OutputMode,
) -> Result<()> {
    let (engine, _) = engine_bridge::open_engine(&project)?;
    let added = engine_bridge::run_requirement_add(
        &engine,
        &text,
        &verification_method,
        &criticality,
        &actor,
    )?;
    emit(mode, &added)
}

/// `aether plugin list` — list installed plugin manifests (G1: scan a
/// `plugins/` directory; G2 reads from the registry).
pub fn cmd_plugin_list(project: PathBuf, mode: OutputMode) -> Result<()> {
    let root = engine_bridge::find_monorepo_root(&project);
    let plugins_dir = root.join("plugins");
    let mut found: Vec<Value> = Vec::new();
    if plugins_dir.is_dir() {
        for entry in std::fs::read_dir(&plugins_dir)
            .with_context(|| format!("read_dir {}", plugins_dir.display()))?
        {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("yaml")
                || path.extension().and_then(|s| s.to_str()) == Some("yml")
            {
                let raw = std::fs::read_to_string(&path)?;
                let v: Value = serde_yaml::from_str(&raw).unwrap_or(Value::Null);
                found.push(serde_json::json!({
                    "path": path.display().to_string(),
                    "manifest": v,
                }));
            }
        }
    }
    let summary = serde_json::json!({
        "plugins": found,
        "count": found.len(),
        "note": "G1: directory scan. G2: registry-backed query with capability matching.",
    });
    emit(mode, &summary)
}

/// `aether package verify` — verify an .aetherpkg manifest's signature
/// and hashes (G1: stub that reads the package layout and emits a
/// summary; full implementation lands in G2).
pub fn cmd_package_verify(project: PathBuf, package: PathBuf, mode: OutputMode) -> Result<()> {
    let root = engine_bridge::find_monorepo_root(&project);
    let manifest_path = package.join("manifest.json");
    if !manifest_path.is_file() {
        return Err(anyhow::anyhow!(
            "no manifest.json at {}",
            manifest_path.display()
        ));
    }
    let raw = std::fs::read_to_string(&manifest_path)?;
    let v: Value = serde_json::from_str(&raw)?;
    let summary = serde_json::json!({
        "package": package.display().to_string(),
        "monorepo": root.display().to_string(),
        "manifest_present": true,
        "manifest": v,
        "note": "G1: manifest is read and rendered. Full hash-chained verification is G2.",
    });
    emit(mode, &summary)
}

/// Re-export the workflow commands (defined in `workflow_cmd.rs`).
pub mod workflow {
    pub use crate::workflow_cmd::*;
}
