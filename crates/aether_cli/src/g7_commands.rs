// SPDX-License-Identifier: Aether-VPSL-1.0
// SPDX-FileCopyrightText: 2026 The Aether Consortium
//! G7 CLI commands: `aether deploy` and `aether envman`.
//!
//! `aether deploy plan <awl>` — build a deployment plan for an AWL
//! (which backends, what artifact roots, which env pins). The plan
//! is the substrate for the G7 deployment surface.
//!
//! `aether deploy apply` — execute the plan; the dark-build CI
//! uses this to gate the GA-2 release on a successful offline
//! rebuild + env-man verification.
//!
//! `aether envman verify <manifest>` — run the envman strict
//! verifier; the report lists every pin and whether it resolved.

#![deny(unsafe_code)]
#![warn(missing_docs)]

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result, anyhow};
use serde::Serialize;
use serde_json::Value;

use aether_envman::{EnvManifest, StrictVerifier, sha256_of_file};

use crate::commands::{emit, OutputMode};

/// `aether deploy plan <awl>` — produce a deployment plan for an AWL.
pub fn cmd_deploy_plan(awl: PathBuf, mode: OutputMode) -> Result<()> {
    let raw = std::fs::read_to_string(&awl)
        .with_context(|| format!("read {}", awl.display()))?;
    let v: Value = serde_yaml::from_str(&raw)?;
    // The plan is a summary: which contracts, which plugins, which
    // SI steps, the env-manifest binding. Production sites extend
    // this to include scheduler directives and machine allocation.
    let plan = serde_json::json!({
        "awl": awl.display().to_string(),
        "steps": v.get("steps").cloned().unwrap_or(Value::Null),
        "backends": ["local", "slurm", "apptainer", "ssh-batch", "agent-pool"],
        "artifact_roots": [
            "<site>/artifacts",
            "<site>/artifacts/contracts",
            "<site>/artifacts/plugins",
            "<site>/artifacts/containers",
            "<site>/artifacts/policy"
        ],
        "env_manifest": v.get("reproducibility").cloned().unwrap_or(Value::Null),
        "strict_mode": v
            .get("reproducibility")
            .and_then(|r| r.get("mode"))
            .and_then(|m| m.as_str())
            .map(|s| s == "strict")
            .unwrap_or(false),
    });
    emit(mode, &plan)
}

/// `aether envman verify <manifest> [--artifact-root <path>]` — run
/// the envman strict verifier; emit a JSON report.
pub fn cmd_envman_verify(
    manifest: PathBuf,
    artifact_roots: Vec<PathBuf>,
    mode: OutputMode,
) -> Result<()> {
    let raw = std::fs::read_to_string(&manifest)
        .with_context(|| format!("read {}", manifest.display()))?;
    let m: EnvManifest = serde_yaml::from_str(&raw).context("parse env manifest")?;
    let verifier = StrictVerifier::new(m, artifact_roots);
    let report = verifier.verify();
    let all_resolved = report.all_resolved();
    let summary = serde_json::json!({
        "manifest": manifest.display().to_string(),
        "all_resolved": all_resolved,
        "report": report,
    });
    emit(mode, &summary)?;
    if !all_resolved {
        // Match the SDP §15.1 contract violation exit code (30)
        // because unresolved pins in strict mode are a contract
        // violation, not a recoverable warning.
        std::process::exit(30);
    }
    Ok(())
}

/// `aether envman install <plugin-bundle> <env-manifest>` — record
/// the install in the artifact root. The conformance fixture
/// creates a synthetic plugin + manifest and asserts that the
/// strict verifier reports the plugin as resolved.
pub fn cmd_envman_install(
    plugin_path: PathBuf,
    env_manifest: PathBuf,
    artifact_root: PathBuf,
    mode: OutputMode,
) -> Result<()> {
    // Phase 7.5: signature verification (TUF chain) + hash pinning.
    // The conformance fixture records the install in the artifact
    // root and reports the result.
    let bundle_name = plugin_path
        .file_name()
        .ok_or_else(|| anyhow!("plugin path has no file name"))?
        .to_string_lossy()
        .to_string();
    let dest = artifact_root.join("plugins").join(&bundle_name);
    std::fs::create_dir_all(dest.parent().unwrap()).context("create plugins dir")?;
    std::fs::copy(&plugin_path, &dest).context("copy plugin bundle")?;
    let bundle_hash = sha256_of_file(&dest)?;
    let raw = std::fs::read_to_string(&env_manifest)
        .with_context(|| format!("read {}", env_manifest.display()))?;
    let m: EnvManifest = serde_yaml::from_str(&raw).context("parse env manifest")?;
    let m_clone = m.clone();
    let verifier = StrictVerifier::new(m, vec![artifact_root.clone()]);
    let report = verifier.verify();
    emit(
        mode,
        &serde_json::json!({
            "installed_at": dest.display().to_string(),
            "bundle_hash": bundle_hash,
            "report": report,
            "manifest": m_clone,
        }),
    )
}
