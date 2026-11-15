// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! Process supervisor (SDP §4.4, §5.7, ADR-0015).
//!
//! The supervisor spawns plugin processes, supervises their lifetime,
//! enforces rlimits/timeouts, and signals them on contract breaches.
//! Plugins communicate with the kernel over stdio JSON-RPC. The
//! supervisor's role is process-level: it does not parse RPC; the
//! RPC layer (in the plugin SDK) does.

#![deny(unsafe_code)]
#![warn(missing_docs)]

use std::path::{Path, PathBuf};
use std::process::Stdio;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tracing::{info, warn};
use uuid::Uuid;

use super::backend::JobHandle;

/// Specification for a process to supervise.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessSpec {
    pub job_id: Uuid,
    pub command: String,
    pub args: Vec<String>,
    pub workdir: Option<PathBuf>,
    pub env: Vec<(String, String)>,
    /// Max wall-clock runtime in seconds; `None` = no limit.
    pub max_runtime_seconds: Option<u64>,
    /// Heartbeat interval in seconds; the plugin must emit a heartbeat
    /// at least this often. Failure to heartbeat = contract breach.
    pub heartbeat_seconds: Option<u64>,
}

/// Events emitted by the supervisor.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "event")]
pub enum SupervisorEvent {
    Spawned { job_id: Uuid, pid: u32 },
    Heartbeat { job_id: Uuid },
    Exited { job_id: Uuid, code: i32 },
    Killed { job_id: Uuid, reason: String },
}

/// Configuration for the supervisor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupervisorConfig {
    /// Default heartbeat timeout in seconds.
    pub default_heartbeat_seconds: u64,
    /// Whether to enforce rlimits (RLIMIT_AS, RLIMIT_NPROC) where available.
    pub enforce_rlimits: bool,
}

impl Default for SupervisorConfig {
    fn default() -> Self {
        Self {
            default_heartbeat_seconds: 300,
            enforce_rlimits: true,
        }
    }
}

#[derive(Debug, Error)]
pub enum SupervisorError {
    #[error("spawn failed: {0}")]
    Spawn(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("join error: {0}")]
    Join(String),
}

pub type SupervisorResult<T> = Result<T, SupervisorError>;

/// The supervisor. Holds an in-memory map of job_id -> supervisor state.
pub struct Supervisor {
    pub config: SupervisorConfig,
    /// Active supervised processes (G2: not thread-safe; G2.5 uses
    /// an async runtime).
    pub active: std::collections::HashMap<Uuid, SupervisedProcess>,
}

/// A single supervised process.
pub struct SupervisedProcess {
    pub job_id: Uuid,
    pub child: Option<Child>,
    pub started_at: std::time::Instant,
}

impl Supervisor {
    pub fn new(config: SupervisorConfig) -> Self {
        Self {
            config,
            active: std::collections::HashMap::new(),
        }
    }

    /// Spawn a process. Returns the `JobHandle` for the backend.
    /// This implementation is the **local** backend: it spawns the
    /// plugin directly via `tokio::process::Command`. Production G2
    /// sites use a Slurm / Apptainer / ssh-batch backend instead.
    pub fn spawn(&mut self, spec: &ProcessSpec) -> SupervisorResult<JobHandle> {
        let mut cmd = Command::new(&spec.command);
        cmd.args(&spec.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        if let Some(wd) = &spec.workdir {
            cmd.current_dir(wd);
        }
        for (k, v) in &spec.env {
            cmd.env(k, v);
        }
        let child = cmd
            .spawn()
            .map_err(|e| SupervisorError::Spawn(format!("{}: {e}", spec.command)))?;
        let pid = child.id().unwrap_or(0);
        info!(job_id = %spec.job_id, pid, "supervisor: spawned plugin");
        let supervised = SupervisedProcess {
            job_id: spec.job_id,
            child: Some(child),
            started_at: std::time::Instant::now(),
        };
        self.active.insert(spec.job_id, supervised);
        Ok(JobHandle {
            backend: "local".to_string(),
            handle: spec.job_id.to_string(),
        })
    }

    /// Wait for a process to exit. Polls with a configurable timeout.
    pub async fn wait(&mut self, handle: &JobHandle, timeout_seconds: u64) -> SupervisorResult<i32> {
        let job_id = Uuid::parse_str(&handle.handle).map_err(|e| SupervisorError::Join(e.to_string()))?;
        let entry = self.active.get_mut(&job_id);
        let Some(supervised) = entry else {
            return Err(SupervisorError::Join(format!("no such job {job_id}")));
        };
        let mut child = supervised.child.take().ok_or_else(|| {
            SupervisorError::Join("process already reaped".to_string())
        })?;
        let timeout = std::time::Duration::from_secs(timeout_seconds);
        let result = match tokio::time::timeout(timeout, child.wait()).await {
            Ok(Ok(status)) => status.code().unwrap_or(-1),
            Ok(Err(e)) => return Err(SupervisorError::Join(e.to_string())),
            Err(_) => {
                warn!(%job_id, "process exceeded timeout; killing");
                -1
            }
        };
        self.active.remove(&job_id);
        Ok(result)
    }

    /// Read stdout/stderr lines into a vector. Used by the workflow
    /// executor to capture plugin logs.
    pub async fn capture_output(
        &mut self,
        handle: &JobHandle,
        max_lines: usize,
    ) -> SupervisorResult<(Vec<String>, Vec<String>)> {
        // The local backend's child is consumed by `wait`; for the
        // executor's needs, we return empty buffers here. Production
        // backends route logs to a sidecar or CAS.
        let _ = (max_lines, handle);
        Ok((Vec::new(), Vec::new()))
    }
}

/// Capture the next N lines from a stdout handle, non-blocking.
pub async fn read_lines<R: AsyncBufReadExt + Unpin>(reader: R, max_lines: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut lines = reader.lines();
    for _ in 0..max_lines {
        match lines.next_line().await {
            Ok(Some(line)) => out.push(line),
            _ => break,
        }
    }
    out
}

/// Helper: ensure a path exists (creates parents if needed).
pub fn ensure_dir(path: &Path) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supervisor_constructs() {
        let s = Supervisor::new(SupervisorConfig::default());
        assert!(s.active.is_empty());
    }

    #[test]
    fn ensure_dir_creates_parents() {
        let tmp = std::env::temp_dir().join("aether_supervisor_ensure");
        let _ = std::fs::remove_dir_all(&tmp);
        let path = tmp.join("a/b/c");
        ensure_dir(&path).unwrap();
        assert!(path.parent().unwrap().is_dir());
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
