// SPDX-License-Identifier: Aether-VPSL-1.0
// SPDX-FileCopyrightText: 2026 The Aether Consortium
//! Handler registry for the CLI spine (Phase 2: doctor / demo / run).
//!
//! `workflow run` historically registered exactly one physics-solver
//! handler (OpenFOAM or CalculiX). The golden path needs the *reference
//! beam* solver (the offline structural default) and the SI governor so
//! demo workflows execute end-to-end without optional binaries.
//! Handlers are keyed by contract id (one handler per contract per run).

use std::path::Path;
use std::sync::Arc;

use aether_exec::handlers::{find_plugin_script, StdioPluginHandler};
use aether_exec::run_token::KernelSigningKey;

/// Contract ids used by the Phase 2 spine.
pub const CONTRACT_PHYSICS_SOLVER: &str = "aether.physics.solver";
pub const CONTRACT_SI_GOVERNOR: &str = "aether.si.governor";
pub const CONTRACT_UQ_METHOD: &str = "aether.uq.method";
pub const CONTRACT_OPTIMIZER: &str = "aether.optimizer";
pub const CONTRACT_TWIN_ESTIMATOR: &str = "aether.twin.estimator";
pub const CONTRACT_SURROGATE: &str = "aether.surrogate.builder";

/// Plugin ids (reverse-DNS) of the bundled reference plugins, used as
/// the plugin-qualified executor routing keys (`contract::plugin`).
pub const PLUGIN_BEAM: &str = "org.aether.reference.fe.beam";
pub const PLUGIN_SIZING: &str = "org.aether.reference.structures.sizing";
pub const PLUGIN_SI: &str = "org.aether.reference.si.essentials";
pub const PLUGIN_UQ: &str = "org.aether.reference.uq.basic";
pub const PLUGIN_OPTIMIZER: &str = "org.aether.reference.optimizer.nsga_iii";
pub const PLUGIN_SURROGATE: &str = "org.aether.reference.surrogate.core";
pub const PLUGIN_TWIN: &str = "org.aether.reference.twin.kf";

/// Register the default (offline-capable) handlers for the golden
/// path: reference beam structural solver + SI governor. Returns the
/// plugin script roots for diagnostics.
pub fn register_default_handlers(
    ex: &mut aether_workflow::execute::Executor,
    monorepo_root: &Path,
    python_bin: &str,
    key: Arc<KernelSigningKey>,
) {
    let beam_script = find_plugin_script(
        monorepo_root,
        "reference_plugins/fea-reference-beam/python/stdio_plugin.py",
    );
    ex.register_handler(Arc::new(StdioPluginHandler::new(
        "fea-reference-beam",
        CONTRACT_PHYSICS_SOLVER,
        python_bin,
        beam_script,
        Arc::clone(&key),
    )));
    // Plugin-qualified routing: a workflow step may pin the sizing
    // implementation via `plugin:` on the same physics-solver contract.
    let sizing_script = find_plugin_script(
        monorepo_root,
        "reference_plugins/structures-sizing/python/stdio_plugin.py",
    );
    ex.register_handler(Arc::new(StdioPluginHandler::new(
        "structures-sizing",
        format!("{CONTRACT_PHYSICS_SOLVER}::{PLUGIN_SIZING}"),
        python_bin,
        sizing_script,
        Arc::clone(&key),
    )));

    let si_script = find_plugin_script(
        monorepo_root,
        "reference_plugins/si-essentials/python/stdio_plugin.py",
    );
    ex.register_handler(Arc::new(StdioPluginHandler::new(
        "si-essentials",
        CONTRACT_SI_GOVERNOR,
        python_bin,
        si_script,
        Arc::clone(&key),
    )));

    // Surrogate builder (gp/rbf training) for the Phase 5
    // DOE -> surrogate -> optimize loop.
    let surrogate_script = find_plugin_script(
        monorepo_root,
        "reference_plugins/surrogates-core/python/stdio_plugin.py",
    );
    ex.register_handler(Arc::new(StdioPluginHandler::new(
        "surrogates-core",
        CONTRACT_SURROGATE,
        python_bin,
        surrogate_script,
        Arc::clone(&key),
    )));

    let uq_script =
        find_plugin_script(monorepo_root, "reference_plugins/uq-basic/python/stdio_plugin.py");
    ex.register_handler(Arc::new(StdioPluginHandler::new(
        "uq-basic",
        CONTRACT_UQ_METHOD,
        python_bin,
        uq_script,
        Arc::clone(&key),
    )));

    let opt_script = find_plugin_script(
        monorepo_root,
        "reference_plugins/optimizer-reference/python/stdio_plugin.py",
    );
    ex.register_handler(Arc::new(StdioPluginHandler::new(
        "optimizer-reference",
        CONTRACT_OPTIMIZER,
        python_bin,
        opt_script,
        Arc::clone(&key),
    )));

    let twin_script = find_plugin_script(
        monorepo_root,
        "reference_plugins/twin-kf/python/stdio_plugin.py",
    );
    ex.register_handler(Arc::new(StdioPluginHandler::new(
        "twin-kf",
        CONTRACT_TWIN_ESTIMATOR,
        python_bin,
        twin_script,
        key,
    )));
}

/// Direct single-call plugin invocation for the CLI spine (doctor,
/// benchmark). Returns the full plugin result document.
pub(crate) fn call_plugin(
    monorepo_root: &Path,
    plugin_rel: &str,
    contract: &str,
    method: &str,
    inputs: &serde_json::Value,
    step_id: &str,
) -> anyhow::Result<serde_json::Value> {
    use aether_exec::handlers::execute_plugin;
    use std::sync::Arc as StdArc;

    let script = find_plugin_script(monorepo_root, plugin_rel);
    if !script.is_file() {
        anyhow::bail!(
            "plugin script not found: {} (is the platform tree intact?)",
            script.display()
        );
    }
    let key = StdArc::new(KernelSigningKey::generate());
    let python_bin = std::env::var("AETHER_PYTHON").unwrap_or_else(|_| "python3".to_string());
    let res = execute_plugin(
        &python_bin,
        &script,
        &key,
        step_id,
        inputs,
        contract,
        method,
    )?;
    Ok(res.result)
}

/// Run the reference beam solver on a cantilever point-load case and
/// return (v_numerical, v_analytical, rel_err) from the plugin's own
/// verification diagnostics. Real execution — the numbers come from
/// the plugin, not from a table.
pub fn probe_beam_solver(monorepo_root: &Path) -> anyhow::Result<(f64, f64, f64)> {
    let result = call_plugin(
        monorepo_root,
        "reference_plugins/fea-reference-beam/python/stdio_plugin.py",
        CONTRACT_PHYSICS_SOLVER,
        "execute",
        &serde_json::json!({
            "problem_id": "doctor-beam-probe",
            "physics_class": "structural",
            "beam_length_m": 1.0,
            "E_pa": 200e9,
            "I_m4": 1e-6,
            "case": "cantilever_point",
            "P_n": 1000.0,
            "n_elements": 8
        }),
        "doctor-beam",
    )?;
    let overall = result
        .get("verification")
        .and_then(|v| v.get("overall_status"))
        .and_then(|v| v.as_str())
        .unwrap_or("fail");
    if overall != "pass" {
        anyhow::bail!("beam solver verification failed: {overall}");
    }
    let diag = result
        .get("diagnostics")
        .ok_or_else(|| anyhow::anyhow!("beam solver returned no diagnostics"))?;
    let v_num = diag
        .get("v_numerical")
        .and_then(|v| v.as_f64())
        .ok_or_else(|| anyhow::anyhow!("no v_numerical in diagnostics"))?;
    let v_ana = diag
        .get("v_analytical")
        .and_then(|v| v.as_f64())
        .ok_or_else(|| anyhow::anyhow!("no v_analytical in diagnostics"))?;
    let rel = diag
        .get("rel_err")
        .and_then(|v| v.as_f64())
        .unwrap_or(f64::INFINITY);
    Ok((v_num, v_ana, rel))
}

/// Probe the SI governor with a FidelityGovernor recommend call.
pub fn probe_si_governor(monorepo_root: &Path) -> anyhow::Result<String> {
    let result = call_plugin(
        monorepo_root,
        "reference_plugins/si-essentials/python/stdio_plugin.py",
        CONTRACT_SI_GOVERNOR,
        "recommend",
        &serde_json::json!({ "governor": "FidelityGovernor", "target_tolerance": 1e-3 }),
        "doctor-si",
    )?;
    result
        .get("selection")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("si governor returned no selection"))
}

/// Probe the UQ method plugin (Monte Carlo on the analytic Ishigami
/// function; deterministic seed).
pub fn probe_uq(monorepo_root: &Path) -> anyhow::Result<(f64, f64)> {
    let result = call_plugin(
        monorepo_root,
        "reference_plugins/uq-basic/python/stdio_plugin.py",
        CONTRACT_UQ_METHOD,
        "propagate",
        &serde_json::json!({
            "method": "monte_carlo",
            "samples": 200,
            "seed": 42,
            "inputs": [
                { "name": "x1", "distribution": "uniform", "params": { "min": -3.14159, "max": 3.14159 } },
                { "name": "x2", "distribution": "uniform", "params": { "min": -3.14159, "max": 3.14159 } },
                { "name": "x3", "distribution": "uniform", "params": { "min": -3.14159, "max": 3.14159 } }
            ],
            "model": { "plugin": "aether.test.ishigami" }
        }),
        "doctor-uq",
    )?;
    let stats = result
        .get("result")
        .and_then(|r| r.get("outputs"))
        .and_then(|o| o.as_array())
        .and_then(|arr| arr.first())
        .and_then(|o| o.get("statistics"))
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("uq plugin returned no statistics"))?;
    let mean = stats.get("mean").and_then(|v| v.as_f64()).unwrap_or(f64::NAN);
    let std = stats.get("std").and_then(|v| v.as_f64()).unwrap_or(f64::NAN);
    Ok((mean, std))
}
