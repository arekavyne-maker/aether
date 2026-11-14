// SPDX-License-Identifier: Aether-VPSL-1.0
// SPDX-FileCopyrightText: 2026 The Aether Consortium
//! Aether kernel — Phase 0 walking skeleton (a).
//!
//! The kernel spawns a plugin process over stdio, exchanges newline-delimited
//! JSON-RPC 2.0 messages, and validates the IPC substrate that all plugins
//! will use (SDP §4.2, §4.4, ADR-0015).
//!
//! The kernel keeps a tiny static LOC footprint on purpose — the real
//! microkernel lands in Phase 1 (G1). This binary's job is to prove the
//! transport, not to be the platform.

#![deny(unsafe_code)]
#![warn(missing_docs)]

use std::process::Stdio;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tracing::{info, warn};
use uuid::Uuid;

/// JSON-RPC 2.0 request envelope (newline-delimited, sent over stdio).
#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcRequest {
    /// Protocol version; always "2.0".
    jsonrpc: String,
    /// Method name.
    method: String,
    /// Optional params object/array.
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<Value>,
    /// Correlation id; kernel-generated UUIDv7.
    id: String,
}

/// JSON-RPC 2.0 response envelope.
#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    /// Present on success.
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    /// Present on error.
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
    /// Echoes the request id on success; null on protocol errors.
    id: Value,
}

/// JSON-RPC 2.0 error object.
#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcError {
    code: i64,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

/// Plugin process supervision: spawn, exchange, reap.
struct PluginProcess {
    child: tokio::process::Child,
    stdin: tokio::process::ChildStdin,
    stdout: BufReader<tokio::process::ChildStdout>,
}

impl PluginProcess {
    async fn spawn(command: &str, args: &[&str]) -> Result<Self> {
        info!(%command, ?args, "spawning plugin process");
        let mut child = Command::new(command)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .kill_on_drop(true)
            .spawn()
            .with_context(|| format!("failed to spawn plugin: {command}"))?;

        let stdin = child.stdin.take().context("plugin stdin unavailable")?;
        let stdout = child.stdout.take().context("plugin stdout unavailable")?;
        Ok(Self {
            child,
            stdin,
            stdout: BufReader::new(stdout),
        })
    }

    async fn request(&mut self, method: &str, params: Option<Value>) -> Result<JsonRpcResponse> {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params,
            id: Uuid::now_v7().to_string(),
        };
        let mut line = serde_json::to_string(&req).context("encode request")?;
        line.push('\n');
        self.stdin.write_all(line.as_bytes()).await.context("write request")?;
        self.stdin.flush().await.context("flush request")?;

        let mut buf = String::new();
        let n = self.stdout.read_line(&mut buf).await.context("read response")?;
        if n == 0 {
            bail!("plugin closed stdout before responding");
        }
        let resp: JsonRpcResponse = serde_json::from_str(buf.trim()).context("decode response")?;
        if resp.jsonrpc != "2.0" {
            warn!(jsonrpc = %resp.jsonrpc, "non-conforming jsonrpc version");
        }
        Ok(resp)
    }

    async fn shutdown(mut self) -> Result<()> {
        // Closing stdin is the polite shutdown signal.
        self.stdin.shutdown().await.ok();
        // Bound the wait so the kernel does not hang on a misbehaving plugin.
        let status = match tokio::time::timeout(
            std::time::Duration::from_secs(5),
            self.child.wait(),
        )
        .await
        {
            Ok(Ok(status)) => status,
            Ok(Err(e)) => return Err(e).context("wait plugin"),
            Err(_) => {
                warn!("plugin did not exit within 5s after stdin close; killing");
                let _ = self.child.start_kill();
                self.child.wait().await.context("wait plugin after kill")?
            }
        };
        info!(?status, "plugin exited");
        Ok(())
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(false)
        .init();

    // Resolve the reference plugin. Phase 0: Python stdio plugin.
    let plugin_arg = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "python3".to_string());
    let script_arg = std::env::args().nth(2).unwrap_or_else(|| {
        "../reference_plugins/cfd-reference-minimal/python/stdio_plugin.py".to_string()
    });

    let mut plugin = PluginProcess::spawn(&plugin_arg, &[&script_arg]).await?;

    // 1. discover
    let discover = plugin.request("discover", None).await?;
    if let Some(err) = discover.error {
        bail!("plugin returned error for discover: {}", err.message);
    }
    let descriptor = discover.result.context("plugin returned no result for discover")?;
    info!(plugin = %descriptor, "discover ok");

    // 2. capabilities
    let caps = plugin.request("capabilities", None).await?;
    if let Some(err) = caps.error {
        bail!("plugin returned error for capabilities: {}", err.message);
    }
    info!(capabilities = %caps.result.unwrap(), "capabilities ok");

    // 3. validate
    let problem = serde_json::json!({
        "problem_id": "01900000-0000-0000-0000-000000000000",
        "physics_class": "fluid",
        "geometry": {
            "cas_hash": "0".repeat(64),
            "format": "native"
        },
        "boundary_conditions": [],
        "solver_settings": {},
        "fidelity_target": "low_fidelity"
    });
    let valid = plugin.request("validate", Some(problem)).await?;
    if let Some(err) = valid.error {
        bail!("plugin returned error for validate: {}", err.message);
    }
    info!(validation = %valid.result.unwrap(), "validate ok");

    plugin.shutdown().await?;
    println!("walking-skeleton-(a): stdio JSON-RPC substrate OK");
    // Dropping the runtime should make main return.
    Ok(())
}
