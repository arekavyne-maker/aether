// SPDX-License-Identifier: Aether-VPSL-1.0
// SPDX-FileCopyrightText: 2026 The Aether Consortium
//! Workflow commands (G2).
//!
//! `aether workflow run <file.awl.yaml>` — compile an AWL document, run
//! it through the executor with the configured step handlers, and emit
//! an `ExecutionResult`. The local backend and the reference solver
//! adapters (`OpenFOAMAdapterHandler`, `CalculiXAdapterHandler`) are
//! the default handler set.

#![deny(unsafe_code)]
#![warn(missing_docs)]

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result, anyhow};
use serde::Serialize;

use aether_exec::handlers::{
    monorepo_root, CalculiXAdapterHandler, OpenFOAMAdapterHandler,
};
use aether_exec::run_token::KernelSigningKey;
use aether_workflow::awl::Awl;
use aether_workflow::checkpoint::InMemoryCheckpointStore;
use aether_workflow::dag::CompiledWorkflow;
use aether_workflow::execute::{ExecutionResult, Executor, ExecutorConfig, StepHandler};

use crate::commands::{emit, OutputMode};

/// Run an AWL workflow.
pub fn cmd_workflow_run(
    workflow_file: PathBuf,
    solver: Option<String>,
    mode: OutputMode,
) -> Result<()> {
    let raw = std::fs::read_to_string(&workflow_file)
        .with_context(|| format!("read {}", workflow_file.display()))?;
    let awl = Awl::parse(&raw).with_context(|| format!("parse {}", workflow_file.display()))?;
    let compiled = CompiledWorkflow::from_awl(&awl)?;

    let root = monorepo_root();
    let python = std::env::var("AETHER_PYTHON").unwrap_or_else(|_| "python3".to_string());
    let key = Arc::new(KernelSigningKey::generate());

    let mut ex = Executor::new(
        ExecutorConfig::default(),
        Box::new(InMemoryCheckpointStore::new()),
    );
    match solver.as_deref() {
        Some("calculix") | Some("calculix-adapter") => {
            ex.register_handler(Arc::new(CalculiXAdapterHandler::new(
                &root, &python, Arc::clone(&key),
            )));
        }
        Some("openfoam") | Some("openfoam-adapter") | None => {
            // Default: OpenFOAM.
            ex.register_handler(Arc::new(OpenFOAMAdapterHandler::new(
                &root, &python, Arc::clone(&key),
            )));
        }
        Some(other) => {
            return Err(anyhow!("unknown solver: {other} (use 'openfoam' or 'calculix')"));
        }
    }

    let result = ex
        .execute(&compiled)
        .map_err(|e| anyhow!("workflow execution failed: {e}"))?;
    emit(
        mode,
        &WorkflowRunResult {
            workflow_id: result.workflow_id,
            status: result.status.clone(),
            started_at: result.started_at.clone(),
            ended_at: result.ended_at.clone(),
            chain_head: result.chain_head.clone(),
            step_results: result.step_results.clone(),
            success: result.success(),
        },
    )?;
    if !result.success() {
        // Match SDP §15.1: 30 = contract violation (workflow failed).
        std::process::exit(30);
    }
    Ok(())
}

#[derive(Debug, Serialize)]
struct WorkflowRunResult {
    workflow_id: uuid::Uuid,
    status: aether_workflow::execute::ExecutionStatus,
    started_at: String,
    ended_at: String,
    chain_head: String,
    step_results: std::collections::BTreeMap<String, aether_workflow::checkpoint::StepResult>,
    success: bool,
}

/// Validate an AWL document without running it.
pub fn cmd_workflow_validate(workflow_file: PathBuf, mode: OutputMode) -> Result<()> {
    let raw = std::fs::read_to_string(&workflow_file)
        .with_context(|| format!("read {}", workflow_file.display()))?;
    let awl = Awl::parse(&raw).with_context(|| format!("parse {}", workflow_file.display()))?;
    let compiled = CompiledWorkflow::from_awl(&awl)?;
    let summary = serde_json::json!({
        "valid": true,
        "workflow_id": compiled.workflow_id,
        "awl_version": compiled.awl_version,
        "step_count": compiled.steps.len(),
        "topological_order": compiled.topological_order,
        "reproducibility": compiled.reproducibility,
        "env_manifest_hash": compiled.env_manifest_hash,
    });
    emit(mode, &summary)
}
