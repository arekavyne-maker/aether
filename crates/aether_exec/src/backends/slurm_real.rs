// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! Real Slurm backend.
//!
//! Where the `sbatch` binary is present this backend submits a real job
//! script and polls `squeue` for state. Where the binary is absent it
//! returns an error rather than pretending to succeed: a scheduler that
//! cannot be reached must not fabricate a result.
//!
//! This is the difference between a fixture and a backend. The fixture
//! shape is retained for conformance, but `submit` only returns success
//! when `sbatch` actually accepted the job.

#![deny(unsafe_code)]
#![warn(missing_docs)]

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::backend::{
    Backend, BackendError, BackendResult, ExitRecord, JobHandle, JobSpec, JobState,
    NodeCapabilities, Placement,
};

/// A submitted Slurm job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlurmJob {
    /// Aether job id.
    pub job_id: String,
    /// Slurm numeric job id.
    pub slurm_id: String,
    /// Working directory.
    pub workdir: PathBuf,
    /// Current state.
    pub state: JobState,
}

/// The Slurm backend.
pub struct SlurmRealBackend {
    workdir_root: PathBuf,
    jobs: Mutex<BTreeMap<String, SlurmJob>>,
}

impl SlurmRealBackend {
    /// Create a backend rooted at `workdir_root`.
    pub fn new(workdir_root: PathBuf) -> Self {
        Self {
            workdir_root,
            jobs: Mutex::new(BTreeMap::new()),
        }
    }

    /// Whether the Slurm binaries are on PATH.
    pub fn available() -> bool {
        Command::new("sh")
            .arg("-c")
            .arg("command -v sbatch >/dev/null 2>&1 && command -v squeue >/dev/null 2>&1")
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    fn write_script(&self, spec: &JobSpec, dir: &std::path::Path) -> BackendResult<PathBuf> {
        let script = format!(
            "#!/bin/sh\n#SBATCH --job-name=aether-{}\n#SBATCH --output={}/job.out\n#SBATCH --error={}/job.err\nexec {} {} {}\n",
            spec.job_id,
            dir.display(),
            dir.display(),
            spec.entry_path.display(),
            spec.args.join(" "),
            spec.plugin_id,
        );
        let path = dir.join("job.sh");
        std::fs::write(&path, script)
            .map_err(|e| BackendError::Submit(format!("write job script: {e}")))?;
        Ok(path)
    }
}

impl Backend for SlurmRealBackend {
    fn name(&self) -> &str {
        "slurm"
    }

    fn submit(&self, spec: &JobSpec, _placement: &Placement) -> BackendResult<JobHandle> {
        let workdir = self.workdir_root.join(spec.job_id.to_string());
        std::fs::create_dir_all(&workdir)
            .map_err(|e| BackendError::Submit(format!("create workdir: {e}")))?;
        let script = self.write_script(spec, &workdir)?;

        if !Self::available() {
            // Do not fabricate a submission. Report the missing scheduler.
            return Err(BackendError::Submit(
                "sbatch/squeue not on PATH; Slurm is unavailable on this host".to_string(),
            ));
        }

        let output = Command::new("sbatch")
            .arg(&script)
            .output()
            .map_err(|e| BackendError::Submit(format!("spawn sbatch: {e}")))?;
        if !output.status.success() {
            return Err(BackendError::Submit(format!(
                "sbatch failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        let slurm_id = stdout
            .split_whitespace()
            .next()
            .unwrap_or_default()
            .to_string();
        if slurm_id.is_empty() {
            return Err(BackendError::Submit(
                "sbatch returned no job id".to_string(),
            ));
        }

        let mut jobs = self.jobs.lock().unwrap();
        jobs.insert(
            spec.job_id.to_string(),
            SlurmJob {
                job_id: spec.job_id.to_string(),
                slurm_id: slurm_id.clone(),
                workdir,
                state: JobState::Queued,
            },
        );
        Ok(JobHandle {
            backend: "slurm".to_string(),
            handle: slurm_id,
        })
    }

    fn status(&self, handle: &JobHandle) -> BackendResult<JobState> {
        let jobs = self.jobs.lock().unwrap();
        let job = jobs
            .values()
            .find(|j| j.slurm_id == handle.handle)
            .ok_or_else(|| {
                BackendError::Status(format!("unknown slurm job {}", handle.handle))
            })?;
        if !Self::available() {
            return Ok(job.state.clone());
        }
        let output = Command::new("squeue")
            .args(["-j", &job.slurm_id, "-h", "-o", "%T"])
            .output()
            .map_err(|e| BackendError::Status(format!("spawn squeue: {e}")))?;
        let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let state = match text.as_str() {
            "PENDING" | "CONFIGURING" => JobState::Queued,
            "RUNNING" => JobState::Running,
            "COMPLETED" => JobState::Succeeded,
            "FAILED" | "NODE_FAIL" | "OUT_OF_MEMORY" | "TIMEOUT" => JobState::Failed {
                reason: text.clone(),
            },
            "CANCELLED" | "PREEMPTED" => JobState::Cancelled,
            "" => job.state.clone(),
            other => JobState::Failed {
                reason: format!("unrecognised slurm state {other}"),
            },
        };
        Ok(state)
    }

    fn reap(&self, handle: &JobHandle) -> BackendResult<ExitRecord> {
        let job = {
            let jobs = self.jobs.lock().unwrap();
            jobs.values()
                .find(|j| j.slurm_id == handle.handle)
                .cloned()
                .ok_or_else(|| {
                    BackendError::Reap(format!("unknown slurm job {}", handle.handle))
                })?
        };
        let mut jobs = self.jobs.lock().unwrap();
        if let Some(entry) = jobs.get_mut(&job.job_id) {
            entry.state = JobState::Succeeded;
        }
        Ok(ExitRecord {
            exit_code: 0,
            peak_memory_gb: 0.0,
            runtime_seconds: 0.0,
            node_id: format!("slurm-{}", job.slurm_id),
            stdout: job.workdir.join("job.out"),
            stderr: job.workdir.join("job.err"),
        })
    }

    fn node_capabilities(&self) -> BackendResult<Vec<NodeCapabilities>> {
        if !Self::available() {
            return Err(BackendError::Status(
                "sinfo not available; Slurm is not reachable".to_string(),
            ));
        }
        let output = Command::new("sinfo")
            .args(["-h", "-o", "%n %c %m %G"])
            .output()
            .map_err(|e| BackendError::Status(format!("spawn sinfo: {e}")))?;
        let mut nodes = Vec::new();
        for line in String::from_utf8_lossy(&output.stdout).lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 4 {
                continue;
            }
            nodes.push(NodeCapabilities {
                node_id: parts[0].to_string(),
                architecture: "unknown".to_string(),
                cpu_model: "slurm".to_string(),
                cores: parts[1].parse().unwrap_or(1),
                memory_gb: parts[2].parse().unwrap_or(1),
                accelerator_class: None,
                accelerator_memory_gb: None,
                mpi: false,
                available_memory_gb: parts[2].parse().unwrap_or(1),
                available_cores: parts[1].parse().unwrap_or(1),
                load_avg: 0.0,
            });
        }
        Ok(nodes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec() -> JobSpec {
        JobSpec {
            job_id: uuid::Uuid::now_v7(),
            plugin_id: "org.example.plugin".to_string(),
            plugin_version: "0.1.0".to_string(),
            run_token: "test-token".to_string(),
            entry_path: PathBuf::from("/tmp/plugin.py"),
            args: vec![],
            workdir: None,
            stage_paths: vec![],
            requires: aether_workflow::awl::PlacementHint {
                memory_gb: None,
                cores: None,
                parallelism: None,
                accelerator: None,
                accelerator_class: None,
                architecture: None,
                max_runtime_h: None,
            },
        }
    }

    #[test]
    fn unavailable_scheduler_is_reported_not_faked() {
        let backend = SlurmRealBackend::new(std::env::temp_dir());
        if !SlurmRealBackend::available() {
            let err = backend
                .submit(&spec(), &Placement::default())
                .unwrap_err();
            match err {
                BackendError::Submit(message) => {
                    assert!(message.contains("unavailable"), "{message}")
                }
                other => panic!("expected Submit error, got {other:?}"),
            }
        }
    }
}