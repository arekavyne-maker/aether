// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! Scheduler SPI (SDP §6.2).
//!
//! Every compute backend implements the `Backend` trait. The kernel
//! talks to backends only through this trait; plugins never see the
//! scheduler directly.

#![deny(unsafe_code)]

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

/// A handle returned by a backend when a job is submitted. Backends
/// use whatever internal id they like; the kernel only persists
/// `JobHandle` and resolves it back via `status()` / `reap()`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobHandle {
    pub backend: String,
    pub handle: String,
}

/// Lifecycle of a scheduled job.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", tag = "status")]
pub enum JobState {
    /// Queued by the backend, not yet running.
    Queued,
    /// Currently executing.
    Running,
    /// Completed successfully.
    Succeeded,
    /// Completed with a non-zero exit.
    Failed { reason: String },
    /// Preempted by the scheduler; the executor may retry.
    Preempted,
    /// Cancelled by the executor (e.g. on workflow cancel).
    Cancelled,
}

/// The job spec: a workflow step + resource requirements + run token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobSpec {
    pub job_id: Uuid,
    pub plugin_id: String,
    pub plugin_version: String,
    pub run_token: String,
    /// Path to the plugin entry point.
    pub entry_path: PathBuf,
    /// Arguments to the plugin entry point.
    pub args: Vec<String>,
    /// Working directory for the plugin process.
    pub workdir: Option<PathBuf>,
    /// Paths to stage into the node-local scratch directory.
    pub stage_paths: Vec<PathBuf>,
    /// Resource requirements.
    pub requires: aether_workflow::awl::PlacementHint,
}

/// Placement hint: which node / class to use.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Placement {
    /// Backend-specific class hint (e.g. "cpu-x86_64-epyc-rome").
    pub node_class: Option<String>,
    /// Optional explicit node id.
    pub node_id: Option<String>,
}

impl Default for Placement {
    fn default() -> Self {
        Self { node_class: None, node_id: None }
    }
}

/// Live inventory + capabilities of a node, reported by the backend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeCapabilities {
    pub node_id: String,
    pub architecture: String,
    pub cpu_model: String,
    pub cores: u32,
    pub memory_gb: u32,
    pub accelerator_class: Option<String>,
    pub accelerator_memory_gb: Option<f64>,
    pub mpi: bool,
    pub available_memory_gb: u32,
    pub available_cores: u32,
    pub load_avg: f64,
}

#[derive(Debug, Error)]
pub enum BackendError {
    #[error("submit failed: {0}")]
    Submit(String),
    #[error("status query failed: {0}")]
    Status(String),
    #[error("reap failed: {0}")]
    Reap(String),
    #[error("not implemented: {0}")]
    NotImplemented(String),
}

pub type BackendResult<T> = Result<T, BackendError>;

/// The scheduler SPI.
pub trait Backend: Send + Sync {
    /// A short identifier (`local`, `slurm`, `apptainer`, `agent-pool`).
    fn name(&self) -> &str;
    /// Submit a job. Returns a `JobHandle`.
    fn submit(&self, spec: &JobSpec, placement: &Placement) -> BackendResult<JobHandle>;
    /// Query a job's state.
    fn status(&self, handle: &JobHandle) -> BackendResult<JobState>;
    /// Reap a completed job; return its exit record.
    fn reap(&self, handle: &JobHandle) -> BackendResult<ExitRecord>;
    /// Inventory the available nodes.
    fn node_capabilities(&self) -> BackendResult<Vec<NodeCapabilities>>;
}

/// A job's exit record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExitRecord {
    pub exit_code: i32,
    pub peak_memory_gb: f64,
    pub runtime_seconds: f64,
    pub node_id: String,
    pub stdout: PathBuf,
    pub stderr: PathBuf,
}
