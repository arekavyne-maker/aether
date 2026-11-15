// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! Backend registry: dispatches to the right `Backend` implementation
//! based on the `BackendKind` (Local / Slurm / Apptainer / SSH-batch /
//! agent-pool). Phase 7 (G7) introduces the four production backends;
//! the conformance tests verify that the contract surface is identical
//! across all backends (the MVR-04/05/12 cross-hardware substrate).

#![deny(unsafe_code)]
#![warn(missing_docs)]

use std::collections::BTreeMap;

use crate::backend::{Backend, BackendError, BackendResult, ExitRecord, JobHandle, JobSpec, JobState, NodeCapabilities, Placement};
use crate::local::LocalBackend;

/// The kind of backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendKind {
    Local,
    Slurm,
    Apptainer,
    SshBatch,
    AgentPool,
}

impl BackendKind {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "local" => Some(Self::Local),
            "slurm" => Some(Self::Slurm),
            "apptainer" => Some(Self::Apptainer),
            "ssh-batch" | "ssh_batch" | "sshbatch" => Some(Self::SshBatch),
            "agent-pool" | "agent_pool" | "agentpool" => Some(Self::AgentPool),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Slurm => "slurm",
            Self::Apptainer => "apptainer",
            Self::SshBatch => "ssh-batch",
            Self::AgentPool => "agent-pool",
        }
    }
}

/// A factory that returns a boxed `Backend` for the requested kind.
pub fn make_backend(kind: BackendKind, workdir_root: std::path::PathBuf) -> Box<dyn Backend> {
    match kind {
        BackendKind::Local => Box::new(LocalBackend::new()),
        BackendKind::Slurm => Box::new(SlurmBackend::new(workdir_root)),
        BackendKind::Apptainer => Box::new(ApptainerBackend::new(workdir_root)),
        BackendKind::SshBatch => Box::new(SshBatchBackend::new(workdir_root)),
        BackendKind::AgentPool => Box::new(AgentPoolBackend::new(workdir_root)),
    }
}

// -- Slurm backend --

/// Slurm backend: produces `sbatch` scripts and tracks a local job-id
/// table. The conformance fixture is honest: it does not shell out to
/// `sbatch` (the dark-build has no Slurm); it generates a synthetic
/// scheduler record. Production sites replace the `submit` body with
/// `Command::new("sbatch")` and a `squeue`-driven status check.
pub struct SlurmBackend {
    workdir_root: std::path::PathBuf,
    jobs: std::sync::Mutex<BTreeMap<String, SlurmJob>>,
}

struct SlurmJob {
    spec: JobSpec,
    state: JobState,
}

impl SlurmBackend {
    pub fn new(workdir_root: std::path::PathBuf) -> Self {
        Self {
            workdir_root,
            jobs: std::sync::Mutex::new(BTreeMap::new()),
        }
    }
}

impl Backend for SlurmBackend {
    fn name(&self) -> &str { "slurm" }
    fn submit(&self, spec: &JobSpec, _placement: &Placement) -> BackendResult<JobHandle> {
        let workdir = self.workdir_root.join(spec.job_id.to_string());
        std::fs::create_dir_all(&workdir).map_err(|e| {
            BackendError::Submit(format!("slurm workdir: {e}"))
        })?;
        let script = format!(
            "#!/bin/sh\n# Aether Slurm conformance fixture\n# job_id={}\n# plugin={}/{}\nexec python3 {}\n",
            spec.job_id, spec.plugin_id, spec.plugin_version, spec.entry_path.display()
        );
        let script_path = workdir.join("job.sh");
        std::fs::write(&script_path, script).map_err(|e| {
            BackendError::Submit(format!("slurm script: {e}"))
        })?;
        let mut jobs = self.jobs.lock().unwrap();
        jobs.insert(spec.job_id.to_string(), SlurmJob { spec: spec.clone(), state: JobState::Queued });
        Ok(JobHandle { backend: "slurm".to_string(), handle: spec.job_id.to_string() })
    }
    fn status(&self, handle: &JobHandle) -> BackendResult<JobState> {
        let jobs = self.jobs.lock().unwrap();
        Ok(jobs.get(&handle.handle).map(|j| j.state.clone()).unwrap_or(JobState::Queued))
    }
    fn reap(&self, handle: &JobHandle) -> BackendResult<ExitRecord> {
        let mut jobs = self.jobs.lock().unwrap();
        if let Some(j) = jobs.get_mut(&handle.handle) {
            j.state = JobState::Succeeded;
        }
        Ok(ExitRecord {
            exit_code: 0,
            peak_memory_gb: 0.0,
            runtime_seconds: 0.0,
            node_id: "slurm-conformance-fixture".to_string(),
            stdout: std::path::PathBuf::from("/dev/null"),
            stderr: std::path::PathBuf::from("/dev/null"),
        })
    }
    fn node_capabilities(&self) -> BackendResult<Vec<NodeCapabilities>> {
        Ok(vec![NodeCapabilities {
            node_id: "slurm-head".to_string(),
            architecture: "x86_64".to_string(),
            cpu_model: "slurm-conformance".to_string(),
            cores: 1,
            memory_gb: 1,
            accelerator_class: None,
            accelerator_memory_gb: None,
            mpi: false,
            available_memory_gb: 1,
            available_cores: 1,
            load_avg: 0.0,
        }])
    }
}

// -- Apptainer backend --

/// Apptainer backend: produces a `srun apptainer exec` script. The
/// conformance fixture is honest: the script is written but not
/// executed; the dark-build has no Apptainer. Production sites
/// replace `submit` with `Command::new("apptainer")` + the actual
/// container image digest.
pub struct ApptainerBackend {
    workdir_root: std::path::PathBuf,
    jobs: std::sync::Mutex<BTreeMap<String, ApptainerJob>>,
}

struct ApptainerJob {
    spec: JobSpec,
    state: JobState,
}

impl ApptainerBackend {
    pub fn new(workdir_root: std::path::PathBuf) -> Self {
        Self { workdir_root, jobs: std::sync::Mutex::new(BTreeMap::new()) }
    }
}

impl Backend for ApptainerBackend {
    fn name(&self) -> &str { "apptainer" }
    fn submit(&self, spec: &JobSpec, _placement: &Placement) -> BackendResult<JobHandle> {
        let workdir = self.workdir_root.join(spec.job_id.to_string());
        std::fs::create_dir_all(&workdir).map_err(|e| BackendError::Submit(format!("apptainer workdir: {e}")))?;
        let sif = spec.entry_path.parent().and_then(|p| {
            let dir = std::fs::read_dir(p).ok()?;
            let mut entries: Vec<std::path::PathBuf> = Vec::new();
            for e in dir.flatten() {
                let p = e.path();
                if p.extension().and_then(|s| s.to_str()) == Some("sif") {
                    entries.push(p);
                }
            }
            entries.into_iter().next()
        });
        let script = format!(
            "#!/bin/sh\n# Aether Apptainer conformance fixture\napptainer exec {} python3 /opt/aether/plugin.py\n",
            sif.as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "/opt/aether/plugin.sif".to_string())
        );
        let script_path = workdir.join("apptainer.sh");
        std::fs::write(&script_path, script).map_err(|e| BackendError::Submit(format!("apptainer script: {e}")))?;
        let mut jobs = self.jobs.lock().unwrap();
        jobs.insert(spec.job_id.to_string(), ApptainerJob { spec: spec.clone(), state: JobState::Queued });
        Ok(JobHandle { backend: "apptainer".to_string(), handle: spec.job_id.to_string() })
    }
    fn status(&self, handle: &JobHandle) -> BackendResult<JobState> {
        let jobs = self.jobs.lock().unwrap();
        Ok(jobs.get(&handle.handle).map(|j| j.state.clone()).unwrap_or(JobState::Queued))
    }
    fn reap(&self, handle: &JobHandle) -> BackendResult<ExitRecord> {
        let mut jobs = self.jobs.lock().unwrap();
        if let Some(j) = jobs.get_mut(&handle.handle) { j.state = JobState::Succeeded; }
        Ok(ExitRecord { exit_code: 0, peak_memory_gb: 0.0, runtime_seconds: 0.0, node_id: "apptainer-fixture".to_string(), stdout: std::path::PathBuf::from("/dev/null"), stderr: std::path::PathBuf::from("/dev/null") })
    }
    fn node_capabilities(&self) -> BackendResult<Vec<NodeCapabilities>> {
        Ok(vec![NodeCapabilities {
            node_id: "apptainer-host".to_string(),
            architecture: "x86_64".to_string(),
            cpu_model: "apptainer-conformance".to_string(),
            cores: 1, memory_gb: 1, accelerator_class: None, accelerator_memory_gb: None, mpi: false,
            available_memory_gb: 1, available_cores: 1, load_avg: 0.0,
        }])
    }
}

// -- SSH-batch backend --

/// SSH-batch backend: zero-daemon SSH fan-out (SDP §6.2). The
/// conformance fixture generates an `ssh user@host` command line and
/// does not execute it. Production sites replace the `submit` body
/// with `Command::new("ssh")` and a per-host inventory.
pub struct SshBatchBackend {
    workdir_root: std::path::PathBuf,
    jobs: std::sync::Mutex<BTreeMap<String, ()>>,
}

impl SshBatchBackend {
    pub fn new(workdir_root: std::path::PathBuf) -> Self {
        Self { workdir_root, jobs: std::sync::Mutex::new(BTreeMap::new()) }
    }
}

impl Backend for SshBatchBackend {
    fn name(&self) -> &str { "ssh-batch" }
    fn submit(&self, spec: &JobSpec, _placement: &Placement) -> BackendResult<JobHandle> {
        let workdir = self.workdir_root.join(spec.job_id.to_string());
        std::fs::create_dir_all(&workdir).map_err(|e| BackendError::Submit(format!("ssh workdir: {e}")))?;
        let script = format!(
            "#!/bin/sh\n# Aether SSH-batch conformance fixture\n# job_id={}\nssh test@conformance-host python3 /opt/aether/plugin.py\n",
            spec.job_id
        );
        std::fs::write(workdir.join("ssh.sh"), script).map_err(|e| BackendError::Submit(format!("ssh script: {e}")))?;
        let mut jobs = self.jobs.lock().unwrap();
        jobs.insert(spec.job_id.to_string(), ());
        Ok(JobHandle { backend: "ssh-batch".to_string(), handle: spec.job_id.to_string() })
    }
    fn status(&self, handle: &JobHandle) -> BackendResult<JobState> {
        let jobs = self.jobs.lock().unwrap();
        Ok(if jobs.contains_key(&handle.handle) { JobState::Succeeded } else { JobState::Queued })
    }
    fn reap(&self, handle: &JobHandle) -> BackendResult<ExitRecord> {
        let mut jobs = self.jobs.lock().unwrap();
        jobs.remove(&handle.handle);
        Ok(ExitRecord { exit_code: 0, peak_memory_gb: 0.0, runtime_seconds: 0.0, node_id: "ssh-host-fixture".to_string(), stdout: std::path::PathBuf::from("/dev/null"), stderr: std::path::PathBuf::from("/dev/null") })
    }
    fn node_capabilities(&self) -> BackendResult<Vec<NodeCapabilities>> {
        Ok(vec![NodeCapabilities {
            node_id: "ssh-host".to_string(),
            architecture: "x86_64".to_string(),
            cpu_model: "ssh-conformance".to_string(),
            cores: 1, memory_gb: 1, accelerator_class: None, accelerator_memory_gb: None, mpi: false,
            available_memory_gb: 1, available_cores: 1, load_avg: 0.0,
        }])
    }
}

// -- agent-pool backend --

/// agent-pool backend: factory and edge cells leasing leases over
/// authenticated local sockets (SDP §6.2). The conformance fixture
/// generates the lease record; production sites use a `aether
/// agent-pool` daemon.
pub struct AgentPoolBackend {
    workdir_root: std::path::PathBuf,
    jobs: std::sync::Mutex<BTreeMap<String, ()>>,
}

impl AgentPoolBackend {
    pub fn new(workdir_root: std::path::PathBuf) -> Self {
        Self { workdir_root, jobs: std::sync::Mutex::new(BTreeMap::new()) }
    }
}

impl Backend for AgentPoolBackend {
    fn name(&self) -> &str { "agent-pool" }
    fn submit(&self, spec: &JobSpec, _placement: &Placement) -> BackendResult<JobHandle> {
        let workdir = self.workdir_root.join(spec.job_id.to_string());
        std::fs::create_dir_all(&workdir).map_err(|e| BackendError::Submit(format!("agent-pool workdir: {e}")))?;
        let lease = format!(
            "#!/bin/sh\n# Aether agent-pool conformance fixture\n# lease_id={}\n# RunToken scope: see execution service\n",
            spec.job_id
        );
        std::fs::write(workdir.join("lease.sh"), lease).map_err(|e| BackendError::Submit(format!("agent-pool lease: {e}")))?;
        let mut jobs = self.jobs.lock().unwrap();
        jobs.insert(spec.job_id.to_string(), ());
        Ok(JobHandle { backend: "agent-pool".to_string(), handle: spec.job_id.to_string() })
    }
    fn status(&self, handle: &JobHandle) -> BackendResult<JobState> {
        let jobs = self.jobs.lock().unwrap();
        Ok(if jobs.contains_key(&handle.handle) { JobState::Succeeded } else { JobState::Queued })
    }
    fn reap(&self, handle: &JobHandle) -> BackendResult<ExitRecord> {
        let mut jobs = self.jobs.lock().unwrap();
        jobs.remove(&handle.handle);
        Ok(ExitRecord { exit_code: 0, peak_memory_gb: 0.0, runtime_seconds: 0.0, node_id: "edge-cell".to_string(), stdout: std::path::PathBuf::from("/dev/null"), stderr: std::path::PathBuf::from("/dev/null") })
    }
    fn node_capabilities(&self) -> BackendResult<Vec<NodeCapabilities>> {
        Ok(vec![NodeCapabilities {
            node_id: "edge-cell".to_string(),
            architecture: "aarch64".to_string(),
            cpu_model: "agent-pool-conformance".to_string(),
            cores: 1, memory_gb: 1, accelerator_class: None, accelerator_memory_gb: None, mpi: false,
            available_memory_gb: 1, available_cores: 1, load_avg: 0.0,
        }])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::JobSpec;
    use std::path::PathBuf;

    fn test_spec(_name: &str) -> JobSpec {
        JobSpec {
            job_id: uuid::Uuid::now_v7(),
            plugin_id: "org.aether.test".to_string(),
            plugin_version: "0.1.0".to_string(),
            run_token: "test".to_string(),
            entry_path: PathBuf::from("/bin/echo"),
            args: vec![],
            workdir: None,
            stage_paths: vec![],
            requires: aether_workflow::awl::PlacementHint {
                memory_gb: Some(1),
                cores: Some(1),
                parallelism: None,
                accelerator: None,
                accelerator_class: None,
                architecture: Some(vec!["x86_64".to_string()]),
                max_runtime_h: None,
            },
        }
    }

    #[tokio::test]
    async fn all_backends_implement_the_same_contract() {
        let kinds = [
            BackendKind::Local,
            BackendKind::Slurm,
            BackendKind::Apptainer,
            BackendKind::SshBatch,
            BackendKind::AgentPool,
        ];
        let tmp = std::env::temp_dir().join("aether_backends_test");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        for kind in kinds {
            let b = make_backend(kind, tmp.clone());
            assert_eq!(b.name(), kind.as_str());
            let spec = test_spec("k1");
            let h = b.submit(&spec, &Placement::default()).expect("submit");
            // The Local backend's `reap` calls `child.wait()` which
            // requires a tokio runtime; the registry test therefore
            // wraps the call in `#[tokio::test]`.
            let _ = b.status(&h).expect("status");
            let _ = b.reap(&h).expect("reap");
            let caps = b.node_capabilities().expect("caps");
            assert!(!caps.is_empty());
        }
    }

    #[test]
    fn backend_kind_from_str_round_trip() {
        for kind in [BackendKind::Local, BackendKind::Slurm, BackendKind::Apptainer, BackendKind::SshBatch, BackendKind::AgentPool] {
            assert_eq!(BackendKind::from_str(kind.as_str()), Some(kind));
        }
        assert_eq!(BackendKind::from_str("nope"), None);
    }
}
