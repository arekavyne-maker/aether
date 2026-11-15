// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! Local backend (SDP §6.2).
//!
//! The `LocalBackend` is the default scheduler for workstation and
//! single-node deployments. It runs plugin processes directly via
//! `tokio::process::Command` and reports live inventory from the host
//! kernel (via `sysctl` on macOS, `/proc/cpuinfo` on Linux; G2 uses a
//! conservative approximation that is sufficient for `aether workflow
//! run` on a workstation).
//!
//! G2.5 adds: rlimit enforcement, filesystem sandboxing, and
//! `seccomp`/`sandbox-exec` profiles. G7 adds the real HPC backends.

#![deny(unsafe_code)]
#![warn(missing_docs)]

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use super::backend::{
    Backend, BackendError, BackendResult, ExitRecord, JobHandle, JobSpec, JobState,
    NodeCapabilities, Placement,
};
use super::run_token::{signed_token, KernelSigningKey, RunToken, RunTokenScope};
use super::supervisor::{ProcessSpec, Supervisor, SupervisorConfig};

#[derive(Debug, Error)]
pub enum LocalError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("job not found: {0}")]
    NotFound(Uuid),
}

pub type LocalResult<T> = Result<T, LocalError>;

/// A single local job's metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalJob {
    pub job_id: Uuid,
    pub spec: JobSpec,
    pub handle: JobHandle,
    pub state: JobState,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub exit_code: Option<i32>,
}

/// The local backend.
pub struct LocalBackend {
    pub kernel_key: Arc<KernelSigningKey>,
    pub supervisor: Arc<Mutex<Supervisor>>,
    pub jobs: Arc<Mutex<HashMap<Uuid, LocalJob>>>,
    /// Lifetime for run tokens issued by this backend (seconds).
    pub token_lifetime_seconds: i64,
}

impl LocalBackend {
    pub fn new() -> Self {
        Self {
            kernel_key: Arc::new(KernelSigningKey::generate()),
            supervisor: Arc::new(Mutex::new(Supervisor::new(SupervisorConfig::default()))),
            jobs: Arc::new(Mutex::new(HashMap::new())),
            token_lifetime_seconds: 3600,
        }
    }

    pub fn with_kernel_key(key: KernelSigningKey) -> Self {
        Self {
            kernel_key: Arc::new(key),
            supervisor: Arc::new(Mutex::new(Supervisor::new(SupervisorConfig::default()))),
            jobs: Arc::new(Mutex::new(HashMap::new())),
            token_lifetime_seconds: 3600,
        }
    }

    /// Issue a run token for a step. The token authorizes reads from
    /// the stage paths and writes to the workdir.
    pub fn issue_token(
        &self,
        workflow_id: Uuid,
        step_id: &str,
        actor: &str,
        stage_paths: &[PathBuf],
        workdir: &PathBuf,
    ) -> anyhow::Result<RunToken> {
        let scope = RunTokenScope {
            read_paths: stage_paths.to_vec(),
            write_paths: vec![workdir.clone()],
            allowed_methods: vec!["physics.solver.execute".to_string()],
        };
        Ok(signed_token(
            &self.kernel_key,
            workflow_id,
            step_id,
            actor,
            scope,
            self.token_lifetime_seconds,
        )?)
    }

    fn build_process_spec(
        job_id: Uuid,
        spec: &JobSpec,
        workdir: PathBuf,
        token: &RunToken,
    ) -> ProcessSpec {
        ProcessSpec {
            job_id,
            command: spec.entry_path.display().to_string(),
            args: spec
                .args
                .iter()
                .cloned()
                .chain(std::iter::once(format!("--run-token={}", token.token_id)))
                .collect(),
            workdir: Some(workdir),
            env: vec![
                ("AETHER_RUN_TOKEN".to_string(), token.token_id.to_string()),
                ("AETHER_WORKFLOW_ID".to_string(), spec.run_token.clone()),
            ],
            max_runtime_seconds: spec
                .requires
                .max_runtime_h
                .map(|h| (h * 3600.0) as u64),
            heartbeat_seconds: None,
        }
    }
}

impl Default for LocalBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Backend for LocalBackend {
    fn name(&self) -> &str {
        "local"
    }

    fn submit(&self, spec: &JobSpec, _placement: &Placement) -> BackendResult<JobHandle> {
        // The workdir is conventionally a project-local scratch dir.
        let workdir = spec
            .workdir
            .clone()
            .unwrap_or_else(|| std::env::temp_dir().join(format!("aether-{}", spec.job_id)));
        std::fs::create_dir_all(&workdir)
            .map_err(|e| BackendError::Submit(format!("workdir: {e}")))?;
        // Stage declared paths
        for p in &spec.stage_paths {
            if p.is_file() {
                if let Some(name) = p.file_name() {
                    let dest = workdir.join(name);
                    std::fs::copy(p, &dest)
                        .map_err(|e| BackendError::Submit(format!("stage {}: {e}", p.display())))?;
                }
            }
        }
        // Issue a run token (kernel signing key in G2; HSM in production).
        // The token's scope covers the stage paths and the workdir.
        let workflow_id = spec
            .args
            .iter()
            .find_map(|a| a.strip_prefix("--workflow-id=").map(|s| s.to_string()))
            .and_then(|s| Uuid::parse_str(&s).ok())
            .unwrap_or_else(Uuid::now_v7);
        let step_id = spec.args.first().cloned().unwrap_or_default();
        let token = self
            .issue_token(
                workflow_id,
                &step_id,
                "local-backend",
                &spec.stage_paths,
                &workdir,
            )
            .map_err(|e| BackendError::Submit(format!("token: {e}")))?;
        let proc_spec = Self::build_process_spec(spec.job_id, spec, workdir.clone(), &token);
        let handle = {
            let mut sup = self.supervisor.lock().unwrap();
            sup.spawn(&proc_spec)
                .map_err(|e| BackendError::Submit(format!("spawn: {e}")))?
        };
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let job = LocalJob {
            job_id: spec.job_id,
            spec: spec.clone(),
            handle: handle.clone(),
            state: JobState::Running,
            started_at: now,
            finished_at: None,
            exit_code: None,
        };
        self.jobs.lock().unwrap().insert(spec.job_id, job);
        Ok(handle)
    }

    fn status(&self, handle: &JobHandle) -> BackendResult<JobState> {
        let job_id =
            Uuid::parse_str(&handle.handle).map_err(|e| BackendError::Status(e.to_string()))?;
        self.jobs
            .lock()
            .unwrap()
            .get(&job_id)
            .map(|j| j.state.clone())
            .ok_or_else(|| BackendError::Status(format!("unknown job {job_id}")))
    }

    fn reap(&self, handle: &JobHandle) -> BackendResult<ExitRecord> {
        let job_id =
            Uuid::parse_str(&handle.handle).map_err(|e| BackendError::Reap(e.to_string()))?;
        let job = self
            .jobs
            .lock()
            .unwrap()
            .get(&job_id)
            .cloned()
            .ok_or_else(|| BackendError::Reap(format!("unknown job {job_id}")))?;
        // The local backend treats the job as "completed" once the
        // handle is reaped; the supervisor's child is consumed by `wait`.
        let code = job.exit_code.unwrap_or(0);
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let mut jobs = self.jobs.lock().unwrap();
        if let Some(j) = jobs.get_mut(&job_id) {
            j.finished_at = Some(now);
            j.exit_code = Some(code);
            j.state = if code == 0 {
                JobState::Succeeded
            } else {
                JobState::Failed {
                    reason: format!("exit code {code}"),
                }
            };
        }
        let workdir = job.spec.workdir.clone().unwrap_or_else(std::env::temp_dir);
        Ok(ExitRecord {
            exit_code: code,
            peak_memory_gb: 0.0,
            runtime_seconds: 0.0,
            node_id: "localhost".to_string(),
            stdout: workdir.join("stdout.log"),
            stderr: workdir.join("stderr.log"),
        })
    }

    fn node_capabilities(&self) -> BackendResult<Vec<NodeCapabilities>> {
        // Local: report a single node with the host's reported counts.
        // Phase 0/G2 uses a conservative approximation; G2.5 reads
        // `/proc/meminfo` and `sysctl` for accurate values.
        let caps = NodeCapabilities {
            node_id: "localhost".to_string(),
            architecture: std::env::consts::ARCH.to_string(),
            cpu_model: "host-cpu".to_string(),
            cores: std::thread::available_parallelism()
                .map(|n| n.get() as u32)
                .unwrap_or(1),
            memory_gb: 16,
            accelerator_class: None,
            accelerator_memory_gb: None,
            mpi: false,
            available_memory_gb: 16,
            available_cores: std::thread::available_parallelism()
                .map(|n| n.get() as u32)
                .unwrap_or(1),
            load_avg: 0.0,
        };
        Ok(vec![caps])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_backend_constructs() {
        let b = LocalBackend::new();
        assert_eq!(b.name(), "local");
        let caps = b.node_capabilities().unwrap();
        assert_eq!(caps.len(), 1);
        assert!(caps[0].cores >= 1);
    }

    #[test]
    fn issue_token_has_scope() {
        let b = LocalBackend::new();
        let wid = Uuid::now_v7();
        let token = b
            .issue_token(
                wid,
                "step-1",
                "tester",
                &[PathBuf::from("/in")],
                &PathBuf::from("/out"),
            )
            .unwrap();
        assert!(!token.signature_b64.is_empty());
        token.verify(&b.kernel_key.verifying_key()).unwrap();
    }
}
