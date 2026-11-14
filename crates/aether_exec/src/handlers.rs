// SPDX-License-Identifier: Aether-VPSL-1.0
// SPDX-FileCopyrightText: 2026 The Aether Consortium
//! `aether.physics.solver` reference handlers (G2).
//!
//! Two independent implementations of the `aether.physics.solver`
//! contract are registered as `StepHandler`s, so a workflow can swap
//! one for the other without changing the AWL (MVR-01/02 conformance).
//!
//! - `OpenFOAMAdapterHandler` — wraps the OpenFOAM stdio plugin
//!   (`reference_plugins/openfoam-adapter/python/stdio_plugin.py`).
//! - `CalculiXAdapterHandler` — wraps the CalculiX stdio plugin
//!   (`reference_plugins/calculix-adapter/python/stdio_plugin.py`).
//!
//! Both delegate the actual work to the Python plugins over stdio
//! JSON-RPC; the handlers themselves are responsible for spawning the
//! subprocess, exchanging messages, and returning a `StepResult`.

#![deny(unsafe_code)]
#![warn(missing_docs)]

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use uuid::Uuid;

use aether_workflow::checkpoint::StepResult;
use aether_workflow::execute::StepHandler;

use crate::run_token::{signed_token, KernelSigningKey, RunToken, RunTokenScope};

#[derive(Debug, Error)]
pub enum HandlerError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("plugin error: {0}")]
    Plugin(String),
    #[error("spawn failed: {0}")]
    Spawn(String),
}

pub type HandlerResult<T> = Result<T, HandlerError>;

/// Find the path to a reference plugin's stdio entry point.
pub fn find_plugin_script(monorepo_root: &Path, plugin_rel: &str) -> PathBuf {
    monorepo_root.join(plugin_rel)
}

/// Locate the monorepo root (helper for handlers that don't have one).
pub fn monorepo_root() -> PathBuf {
    // AETHER_MONOREPO override: lets a standalone (out-of-monorepo)
    // project resolve the platform catalog and plugins against an
    // explicitly installed Aether platform.
    if let Ok(root) = std::env::var("AETHER_MONOREPO") {
        let p = std::path::PathBuf::from(root);
        if p.join("contracts").is_dir() {
            return p;
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        let mut ancestor = exe;
        for _ in 0..8 {
            if ancestor.join("contracts").is_dir() && ancestor.join("Cargo.toml").is_file() {
                return ancestor;
            }
            if !ancestor.pop() {
                break;
            }
        }
    }
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let mut here = cwd.clone();
    loop {
        if here.join("contracts").is_dir() && here.join("Cargo.toml").is_file() {
            return here;
        }
        if !here.pop() {
            return cwd;
        }
    }
}

/// A JSON-RPC 2.0 request (newline-delimited over stdio).
#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<Value>,
    id: String,
}

/// A JSON-RPC 2.0 response.
#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
    id: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JsonRpcError {
    code: i64,
    message: String,
}

/// Drive a stdio JSON-RPC plugin and execute one method call.
/// This is the **same** protocol as Phase 0's walking skeleton (a),
/// extended to carry a typed result bundle.
pub fn run_stdio_plugin(
    python_bin: &str,
    script: &Path,
    run_token: &RunToken,
    step_id: &str,
    inputs: &Value,
    contract: &str,
    override_method: Option<&str>,
) -> HandlerResult<StepResult> {
    let result = drive_stdio_plugin(
        python_bin, script, run_token, step_id, inputs, contract, override_method,
    )?;
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    if result.get("error").is_some() {
        return Ok(StepResult {
            step_id: step_id.to_string(),
            status: "failed".to_string(),
            attempt: 1,
            started_at: now.clone(),
            ended_at: now,
            output_hashes: Default::default(),
            plugin_id: None,
            plugin_version: None,
            determinism_class: None,
            diagnostics: Some(serde_json::json!({ "error": result.get("error") })),
        });
    }
    let output_hashes = extract_output_hashes(&result);
    Ok(StepResult {
        step_id: step_id.to_string(),
        status: "succeeded".to_string(),
        attempt: 1,
        started_at: now.clone(),
        ended_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        output_hashes,
        plugin_id: Some(result
            .get("plugin_id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string()),
        plugin_version: Some(result
            .get("plugin_version")
            .and_then(|v| v.as_str())
            .unwrap_or("0.0.0")
            .to_string()),
        determinism_class: result
            .get("determinism_class")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        diagnostics: Some(
            result
                .get("diagnostics")
                .cloned()
                .or_else(|| result.get("verification").cloned())
                // SI-governor steps return a decision record (question /
                // selection / options) as the result root; carry it so
                // the CLI can record it as a graph Decision node.
                .or_else(|| {
                    if result.get("selection").is_some() {
                        Some(result.clone())
                    } else {
                        None
                    }
                })
                .unwrap_or(Value::Null),
        ),
    })
}

/// A full JSON-RPC plugin call result: the raw result document, kept
/// intact (outputs, verification, diagnostics) so callers that need the
/// engineering payload (the CLI spine: doctor, benchmark, evidence) can
/// read actual values instead of only hashes.
#[derive(Debug, Clone)]
pub struct PluginCallResult {
    /// The full JSON-RPC `result` document from the plugin.
    pub result: Value,
}

/// Drive one JSON-RPC method call against a stdio plugin and return
/// the complete result document. The single-plugin substrate used by
/// `run_stdio_plugin` (step execution) and `execute_plugin` (direct
/// CLI calls). Returns the raw result; JSON-RPC *protocol* errors are
/// surfaced as `Err`, while contract-level failures (validation fail,
/// verification fail) are returned in `result.verification` per I-12.
pub fn execute_plugin(
    python_bin: &str,
    script: &Path,
    kernel_key: &Arc<KernelSigningKey>,
    step_id: &str,
    inputs: &Value,
    contract: &str,
    method: &str,
) -> HandlerResult<PluginCallResult> {
    let workflow_id = Uuid::now_v7();
    let scope = RunTokenScope {
        read_paths: vec![script.to_path_buf()],
        write_paths: vec![script.parent().unwrap_or(Path::new(".")).to_path_buf()],
        allowed_methods: vec![method.to_string(), "init".to_string()],
    };
    let token = signed_token(kernel_key, workflow_id, step_id, "stdio-plugin-cli", scope, 3600)
        .map_err(|e| HandlerError::Spawn(format!("token: {e}")))?;
    let result = drive_stdio_plugin(python_bin, script, &token, step_id, inputs, contract, Some(method))?;
    Ok(PluginCallResult { result })
}

/// The shared stdio JSON-RPC protocol driver: spawn, init handshake,
/// one method call, parse the response into the raw result Value.
fn drive_stdio_plugin(
    python_bin: &str,
    script: &Path,
    run_token: &RunToken,
    step_id: &str,
    inputs: &Value,
    contract: &str,
    override_method: Option<&str>,
) -> HandlerResult<Value> {
    let mut child = Command::new(python_bin)
        .arg(script)
        .arg("--stdio")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("AETHER_RUN_TOKEN", run_token.token_id.to_string())
        .spawn()
        .map_err(|e| HandlerError::Spawn(format!("{}: {e}", script.display())))?;

    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| HandlerError::Spawn("plugin stdin".to_string()))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| HandlerError::Spawn("plugin stdout".to_string()))?;
    let mut stdout = BufReader::new(stdout);
    let mut stdin = stdin;

    // JSON-RPC: first message is the run token (as a method call so
    // the plugin can verify it before processing other requests).
    let init = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "init".to_string(),
        params: Some(serde_json::to_value(run_token)?),
        id: Uuid::now_v7().to_string(),
    };
    let init_str = serde_json::to_string(&init)?;
    stdin.write_all(init_str.as_bytes())?;
    stdin.write_all(b"\n")?;
    stdin.flush()?;

    // Read the init response.
    let mut line = String::new();
    stdout.read_line(&mut line)?;
    let _init_resp: JsonRpcResponse = serde_json::from_str(line.trim())?;

    // Send the JSON-RPC method. Most contracts use the contract id's
    // last segment (geometry.kernel -> "import", physics.solver ->
    // "execute"); we look it up from the contract passed in. An
    // `override_method` lets callers select a different method on the
    // same contract (e.g. `sobol_indices` on aether.uq.method).
    let method = override_method
        .map(|s| s.to_string())
        .unwrap_or_else(|| method_for_contract(contract).to_string());
    let req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        method: method.to_string(),
        params: Some(serde_json::json!({
            "step_id": step_id,
            "inputs": inputs,
        })),
        id: Uuid::now_v7().to_string(),
    };
    let req_str = serde_json::to_string(&req)?;
    stdin.write_all(req_str.as_bytes())?;
    stdin.write_all(b"\n")?;
    stdin.flush()?;

    // Read the execute response.
    line.clear();
    stdout.read_line(&mut line)?;
    let resp: JsonRpcResponse = serde_json::from_str(line.trim())?;
    if let Some(err) = resp.error {
        // JSON-RPC protocol-level error (method not found, plugin
        // internal error). Return it as an error object in the result
        // position; the single-plugin callers decide pass/fail.
        return Ok(serde_json::json!({ "error": err }));
    }
    Ok(resp.result.unwrap_or(Value::Null))
}

fn extract_output_hashes(result: &Value) -> std::collections::BTreeMap<String, String> {
    let mut out = std::collections::BTreeMap::new();
    if let Some(outputs) = result.get("outputs").and_then(|v| v.as_object()) {
        for (k, v) in outputs {
            if let Some(h) = v.get("cas_hash").and_then(|v| v.as_str()) {
                out.insert(k.clone(), h.to_string());
            }
        }
    }
    out
}

/// Build the JSON-RPC method name from the contract id. Most
/// contracts in Aether use the contract id's last segment as the
/// method name (e.g. `aether.physics.solver` -> `execute`,
/// `aether.geometry.kernel` -> `import`,
/// `aether.mesh.engine` -> `generate`,
/// `aether.uq.method` -> `propagate`,
/// `aether.si.governor` -> `recommend`,
/// `aether.twin.estimator` -> `estimate`,
/// `aether.optimizer` -> `optimize`,
/// `aether.surrogate.builder` -> `train`,
/// `aether.manufacturing.process` -> `plan`,
/// `aether.inspection.device` -> `acquire`,
/// `aether.test.rig` -> `run`).
fn method_for_contract(contract: &str) -> &'static str {
    if contract.starts_with("aether.geometry") {
        "import"
    } else if contract.starts_with("aether.mesh") {
        "generate"
    } else if contract.starts_with("aether.uq") {
        "propagate"
    } else if contract.starts_with("aether.si") {
        "recommend"
    } else if contract.starts_with("aether.twin") {
        "estimate"
    } else if contract.starts_with("aether.optimizer") {
        "optimize"
    } else if contract.starts_with("aether.surrogate") {
        "train"
    } else if contract.starts_with("aether.manufacturing") {
        "plan"
    } else if contract.starts_with("aether.inspection") {
        "acquire"
    } else if contract.starts_with("aether.test") {
        "run"
    } else {
        "execute"
    }
}

/// Strip the harness-only `_schema_id` field from an input document.
/// Some contracts use this convention to declare which schema a
/// document validates against.
#[allow(dead_code)]
fn strip_schema_id(v: &mut Value) {
    if let Some(obj) = v.as_object_mut() {
        obj.remove("_schema_id");
    }
}

/// A handler that wraps a Python stdio plugin (the native binary
/// adapter pattern, SDP §5.6). The handler spawns the plugin for each
/// `execute` call and returns the result bundle.
pub struct StdioPluginHandler {
    pub name: String,
    pub contract: String,
    pub python_bin: String,
    pub script_path: PathBuf,
    pub kernel_key: Arc<KernelSigningKey>,
    pub token_lifetime_seconds: i64,
}

impl StdioPluginHandler {
    pub fn new(
        name: impl Into<String>,
        contract: impl Into<String>,
        python_bin: impl Into<String>,
        script_path: PathBuf,
        kernel_key: Arc<KernelSigningKey>,
    ) -> Self {
        Self {
            name: name.into(),
            contract: contract.into(),
            python_bin: python_bin.into(),
            script_path,
            kernel_key,
            token_lifetime_seconds: 3600,
        }
    }

    /// Build the JSON-RPC method name from the contract id. Most
    /// contracts in Aether use the contract id's last segment as the
    /// method name (e.g. `aether.physics.solver` -> `execute`,
    /// `aether.geometry.kernel` -> `import`).
    fn method_name(&self) -> &'static str {
        if self.contract.starts_with("aether.geometry") {
            "import"
        } else {
            "execute"
        }
    }
}

impl StepHandler for StdioPluginHandler {
    fn name(&self) -> &str {
        &self.name
    }
    fn contract_id(&self) -> &str {
        &self.contract
    }
    fn execute(
        &self,
        step_id: &str,
        inputs: &serde_json::Value,
        attempt: u32,
    ) -> Result<StepResult, String> {
        self.execute_with_method(step_id, inputs, attempt, None)
    }
}

impl StdioPluginHandler {
    /// Like `execute` but with an explicit JSON-RPC method override
    /// (used by `cmd_uq_sobol` to dispatch to `sobol_indices` instead
    /// of the default `propagate`).
    pub fn execute_with_method(
        &self,
        step_id: &str,
        inputs: &serde_json::Value,
        attempt: u32,
        override_method: Option<&str>,
    ) -> Result<StepResult, String> {
        let workflow_id = Uuid::now_v7();
        let scope = RunTokenScope {
            read_paths: vec![self.script_path.clone()],
            write_paths: vec![self.script_path.parent().unwrap_or(Path::new(".")).to_path_buf()],
            allowed_methods: vec![self.method_name().to_string(), "init".to_string()],
        };
        let token = signed_token(
            &self.kernel_key,
            workflow_id,
            step_id,
            "stdio-handler",
            scope,
            self.token_lifetime_seconds,
        )
        .map_err(|e| format!("token: {e}"))?;
        match run_stdio_plugin(
            &self.python_bin,
            &self.script_path,
            &token,
            step_id,
            inputs,
            &self.contract,
            override_method,
        ) {
            Ok(mut r) => {
                r.attempt = attempt;
                Ok(r)
            }
            Err(e) => Err(format!("plugin error: {e}")),
        }
    }
}

/// OpenFOAM adapter handler. Wraps the CFD reference stdio plugin
/// (`reference_plugins/cfd-reference-minimal/python/stdio_plugin.py`).
pub struct OpenFOAMAdapterHandler {
    inner: StdioPluginHandler,
}

impl OpenFOAMAdapterHandler {
    pub fn new(monorepo_root: &Path, python_bin: &str, kernel_key: Arc<KernelSigningKey>) -> Self {
        let script = find_plugin_script(
            monorepo_root,
            "reference_plugins/cfd-reference-minimal/python/stdio_plugin.py",
        );
        Self {
            inner: StdioPluginHandler::new(
                "openfoam-adapter",
                "aether.physics.solver",
                python_bin,
                script,
                kernel_key,
            ),
        }
    }
}

impl StepHandler for OpenFOAMAdapterHandler {
    fn name(&self) -> &str { self.inner.name() }
    fn contract_id(&self) -> &str { self.inner.contract_id() }
    fn execute(&self, step_id: &str, inputs: &serde_json::Value, attempt: u32) -> Result<StepResult, String> {
        self.inner.execute(step_id, inputs, attempt)
    }
}

/// CalculiX adapter handler. Wraps
/// `reference_plugins/calculix-adapter/python/stdio_plugin.py`.
pub struct CalculiXAdapterHandler {
    inner: StdioPluginHandler,
}

impl CalculiXAdapterHandler {
    pub fn new(monorepo_root: &Path, python_bin: &str, kernel_key: Arc<KernelSigningKey>) -> Self {
        let script = find_plugin_script(
            monorepo_root,
            "reference_plugins/calculix-adapter/python/stdio_plugin.py",
        );
        Self {
            inner: StdioPluginHandler::new(
                "calculix-adapter",
                "aether.physics.solver",
                python_bin,
                script,
                kernel_key,
            ),
        }
    }
}

impl StepHandler for CalculiXAdapterHandler {
    fn name(&self) -> &str { self.inner.name() }
    fn contract_id(&self) -> &str { self.inner.contract_id() }
    fn execute(&self, step_id: &str, inputs: &serde_json::Value, attempt: u32) -> Result<StepResult, String> {
        self.inner.execute(step_id, inputs, attempt)
    }
}

/// Mesh baseline handler. Wraps
/// `reference_plugins/mesh-baseline/python/stdio_plugin.py`.
pub struct MeshBaselineHandler {
    inner: StdioPluginHandler,
}

impl MeshBaselineHandler {
    pub fn new(monorepo_root: &Path, python_bin: &str, kernel_key: Arc<KernelSigningKey>) -> Self {
        let script = find_plugin_script(
            monorepo_root,
            "reference_plugins/mesh-baseline/python/stdio_plugin.py",
        );
        Self {
            inner: StdioPluginHandler::new(
                "mesh-baseline",
                "aether.mesh.engine",
                python_bin,
                script,
                kernel_key,
            ),
        }
    }
}

impl StepHandler for MeshBaselineHandler {
    fn name(&self) -> &str { self.inner.name() }
    fn contract_id(&self) -> &str { self.inner.contract_id() }
    fn execute(&self, step_id: &str, inputs: &serde_json::Value, attempt: u32) -> Result<StepResult, String> {
        self.inner.execute(step_id, inputs, attempt)
    }
}

/// State shared between handlers and the lifecycle: the kernel signing
/// key, so a single key signs run tokens for all handlers.
pub struct SharedState {
    pub kernel_key: Arc<KernelSigningKey>,
    pub python_bin: String,
}

impl SharedState {
    pub fn new() -> Self {
        Self {
            kernel_key: Arc::new(KernelSigningKey::generate()),
            python_bin: std::env::var("AETHER_PYTHON").unwrap_or_else(|_| "python3".to_string()),
        }
    }
}

impl Default for SharedState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_plugin_script_path() {
        let root = monorepo_root();
        let p = find_plugin_script(&root, "reference_plugins/openfoam-adapter/python/stdio_plugin.py");
        // Either the file exists (we're in the monorepo) or we get a
        // path that would be the expected location.
        assert!(p.ends_with("stdio_plugin.py"));
    }

    #[test]
    fn shared_state_constructs() {
        let s = SharedState::new();
        assert!(!s.python_bin.is_empty());
    }
}
