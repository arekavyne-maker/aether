// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! Multiphysics + UQ + SI CLI commands (G4).
//!
//! - `aether uq propagate <problem.yaml>` — run a UQ propagation
//!   through the reference UQ plugin and emit the result.
//! - `aether simulate compose <coupling.yaml>` — run a multiphysics
//!   composition through `aether_compose::CompositionEngine` and
//!   emit the result.

#![deny(unsafe_code)]
#![warn(missing_docs)]

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result, anyhow};
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

use aether_compose::{CompositionEngine, Coupling};
use aether_exec::handlers::{monorepo_root, StdioPluginHandler};
use aether_exec::run_token::KernelSigningKey;
use aether_workflow::checkpoint::InMemoryCheckpointStore;
use aether_workflow::dag::CompiledWorkflow;
use aether_workflow::execute::{Executor, ExecutorConfig, StepHandler};
use crate::commands::{emit, OutputMode};

/// `aether uq propagate <problem.yaml>` — run UQ propagation through
/// the `aether.uq.method` reference plugin.
pub fn cmd_uq_propagate(problem: PathBuf, mode: OutputMode) -> Result<()> {
    let problem_value = load_uq_problem(&problem)?;
    let result = uq_dispatch(&problem_value, None, mode)?;
    emit(mode, &result)
}

/// `aether uq sobol <problem.yaml>` — compute Sobol indices via the
/// `aether.uq.method` reference plugin's `sobol_indices` method.
pub fn cmd_uq_sobol(problem: PathBuf, mode: OutputMode) -> Result<()> {
    let problem_value = load_uq_problem(&problem)?;
    let result = uq_dispatch(&problem_value, Some("sobol_indices"), mode)?;
    emit(mode, &result)
}

fn load_uq_problem(problem: &PathBuf) -> Result<Value> {
    let raw = std::fs::read_to_string(problem)
        .with_context(|| format!("read {}", problem.display()))?;
    let v: Value = serde_yaml::from_str(&raw)?;
    Ok(strip_schema_id(v))
}

fn uq_dispatch(
    problem_value: &Value,
    override_method: Option<&str>,
    mode: OutputMode,
) -> Result<serde_json::Value> {
    let root = monorepo_root();
    let python = std::env::var("AETHER_PYTHON").unwrap_or_else(|_| "python3".to_string());
    let key = Arc::new(KernelSigningKey::generate());

    let handler = StdioPluginHandler::new(
        "uq-basic",
        "aether.uq.method",
        &python,
        root.join("reference_plugins/uq-basic/python/stdio_plugin.py"),
        Arc::clone(&key),
    );
    let step_id = "uq-propagate";
    let result = handler
        .execute_with_method(step_id, problem_value, 1, override_method)
        .map_err(|e| anyhow!("UQ plugin error: {e}"))?;
    Ok(serde_json::to_value(&result)?)
}

/// `aether simulate compose <coupling.yaml>` — run a multiphysics
/// composition through `aether_compose::CompositionEngine`.
pub fn cmd_simulate_compose(coupling_file: PathBuf, mode: OutputMode) -> Result<()> {
    let raw = std::fs::read_to_string(&coupling_file)
        .with_context(|| format!("read {}", coupling_file.display()))?;
    let v: Value = serde_yaml::from_str(&raw)?;
    let compose = v
        .get("compose")
        .ok_or_else(|| anyhow!("AWL has no 'compose:' block"))?;
    let coupling = Coupling::from_json(compose)?;
    let mut engine = CompositionEngine::new();
    let result = engine
        .run_simulation(&coupling)
        .map_err(|e| anyhow!("composition engine: {e}"))?;
    // Hash the result bundle for CAS.
    let canonical = serde_json::to_string(&result)?;
    let mut h = Sha256::new();
    h.update(canonical.as_bytes());
    let result_hash = format!("{:x}", h.finalize());
    let summary = serde_json::json!({
        "coupling": coupling.name,
        "result_hash": result_hash,
        "converged": result.converged,
        "iterations": result.iterations,
        "convergence_residual": result.convergence_residual,
        "diverged": result.diverged,
        "conservation_residuals": result.conservation_residuals,
        "per_step": result.per_step_results,
        "note": "G4: composition engine's internal simulation converges. Production sites run real CFD + FEA participants via aether_compose::run_with_plugins.",
    });
    emit(mode, &summary)
}

/// `aether optimize <problem.yaml>` — run NSGA-III on the
/// `aether.optimizer` reference plugin and emit the result.
pub fn cmd_optimize(problem: PathBuf, mode: OutputMode) -> Result<()> {
    let raw = std::fs::read_to_string(&problem)
        .with_context(|| format!("read {}", problem.display()))?;
    let v: Value = serde_yaml::from_str(&raw)?;
    let v = strip_schema_id(v);
    let root = monorepo_root();
    let python = std::env::var("AETHER_PYTHON").unwrap_or_else(|_| "python3".to_string());
    let key = Arc::new(KernelSigningKey::generate());
    let handler = StdioPluginHandler::new(
        "optimizer",
        "aether.optimizer",
        &python,
        root.join("reference_plugins/optimizer-reference/python/stdio_plugin.py"),
        Arc::clone(&key),
    );
    let result = handler
        .execute("optimize", &v, 1)
        .map_err(|e| anyhow!("optimizer plugin error: {e}"))?;
    let result_json: Value = serde_json::to_value(&result)?;
    emit(mode, &result_json)
}

/// `aether twin estimate <problem.yaml>` — run the EKF/UKF
/// reference plugin on the `aether.twin.estimator` contract and
/// emit the state estimate.
pub fn cmd_twin_estimate(problem: PathBuf, mode: OutputMode) -> Result<()> {
    let raw = std::fs::read_to_string(&problem)
        .with_context(|| format!("read {}", problem.display()))?;
    let v: Value = serde_yaml::from_str(&raw)?;
    let v = strip_schema_id(v);
    let root = monorepo_root();
    let python = std::env::var("AETHER_PYTHON").unwrap_or_else(|_| "python3".to_string());
    let key = Arc::new(KernelSigningKey::generate());
    let handler = StdioPluginHandler::new(
        "twin-ekf",
        "aether.twin.estimator",
        &python,
        root.join("reference_plugins/twin-kf/python/stdio_plugin.py"),
        Arc::clone(&key),
    );
    let result = handler
        .execute("twin-estimate", &v, 1)
        .map_err(|e| anyhow!("twin plugin error: {e}"))?;
    let result_json: Value = serde_json::to_value(&result)?;
    emit(mode, &result_json)
}

/// `aether manufacture plan <problem.yaml>` — run the
/// `aether.manufacturing.process` reference plugin and emit the
/// program artifact.
pub fn cmd_manufacture_plan(problem: PathBuf, mode: OutputMode) -> Result<()> {
    let raw = std::fs::read_to_string(&problem)
        .with_context(|| format!("read {}", problem.display()))?;
    let v: Value = serde_yaml::from_str(&raw)?;
    let v = strip_schema_id(v);
    let result = dispatch_physics_contract(
        "aether.manufacturing.process",
        "plc-reference",
        "reference_plugins/plc-reference/python/stdio_plugin.py",
        "manufacture-plan",
        &v,
        mode,
    )?;
    emit(mode, &result)
}

/// `aether inspect acquire <problem.yaml>` — run the
/// `aether.inspection.device` reference plugin (CMM acquire).
pub fn cmd_inspect_acquire(problem: PathBuf, mode: OutputMode) -> Result<()> {
    let raw = std::fs::read_to_string(&problem)
        .with_context(|| format!("read {}", problem.display()))?;
    let v: Value = serde_yaml::from_str(&raw)?;
    let v = strip_schema_id(v);
    let result = dispatch_physics_contract(
        "aether.inspection.device",
        "plc-reference",
        "reference_plugins/plc-reference/python/stdio_plugin.py",
        "cmm-acquire",
        &v,
        mode,
    )?;
    emit(mode, &result)
}

/// `aether test run <problem.yaml>` — run the
/// `aether.test.rig` reference plugin (test run).
pub fn cmd_test_run(problem: PathBuf, mode: OutputMode) -> Result<()> {
    let raw = std::fs::read_to_string(&problem)
        .with_context(|| format!("read {}", problem.display()))?;
    let v: Value = serde_yaml::from_str(&raw)?;
    let v = strip_schema_id(v);
    let result = dispatch_physics_contract(
        "aether.test.rig",
        "plc-reference",
        "reference_plugins/plc-reference/python/stdio_plugin.py",
        "rig-run",
        &v,
        mode,
    )?;
    emit(mode, &result)
}

fn dispatch_physics_contract(
    contract: &str,
    handler_name: &str,
    script_rel: &str,
    step_id: &str,
    inputs: &Value,
    _mode: OutputMode,
) -> Result<Value> {
    let root = monorepo_root();
    let python = std::env::var("AETHER_PYTHON").unwrap_or_else(|_| "python3".to_string());
    let key = Arc::new(KernelSigningKey::generate());
    let handler = StdioPluginHandler::new(
        handler_name,
        contract,
        &python,
        root.join(script_rel),
        Arc::clone(&key),
    );
    let result = handler
        .execute(step_id, inputs, 1)
        .map_err(|e| anyhow!("{contract} plugin error: {e}"))?;
    Ok(serde_json::to_value(&result)?)
}

fn strip_schema_id(mut v: Value) -> Value {
    if let Some(obj) = v.as_object_mut() {
        obj.remove("_schema_id");
    }
    v
}
