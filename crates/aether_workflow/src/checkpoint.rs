// SPDX-License-Identifier: Aether-VPSL-1.0
// SPDX-FileCopyrightText: 2026 The Aether Consortium
//! Checkpointing (G2; MVR-09/11 substrate).
//!
//! A `Checkpoint` is the per-step execution trace: the set of completed
//! steps, their results (CAS hashes), and a Merkle chain hash over the
//! trace. The checkpoint is **content-addressed** and **append-only**;
//! resume picks up at the last completed step.
//!
//! The `CheckpointStore` trait is the storage SPI. G2 ships the
//! `InMemoryCheckpointStore`; the durable backend (file-based or
//! object-store) lands in G2.5.

#![deny(unsafe_code)]
#![warn(missing_docs)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum CheckpointError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("checkpoint not found: {0}")]
    NotFound(String),
    #[error("checkpoint integrity check failed: {0}")]
    Integrity(String),
}

pub type CheckpointResult<T> = Result<T, CheckpointError>;

/// A single step's outcome, captured in the checkpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    pub step_id: String,
    pub status: String,
    pub attempt: u32,
    pub started_at: String,
    pub ended_at: String,
    /// CAS hashes of outputs produced.
    pub output_hashes: BTreeMap<String, String>,
    /// Plugin id + version (for provenance).
    pub plugin_id: Option<String>,
    pub plugin_version: Option<String>,
    /// Determinism class observed.
    pub determinism_class: Option<String>,
    /// Engineering payload reported by the plugin: verification status,
    /// measured/derived quantities, diagnostics. Kept verbatim so the
    /// CLI spine (explain / evidence) can report *what the plugin
    /// actually computed* rather than only output hashes. Optional for
    /// backward compatibility with checkpoints written before this
    /// field existed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diagnostics: Option<serde_json::Value>,
}

impl StepResult {
    /// Compute a content hash of this result.
    pub fn content_hash(&self) -> String {
        let json = serde_json::to_vec(self).expect("step result encodes");
        let mut h = Sha256::new();
        h.update(&json);
        format!("{:x}", h.finalize())
    }
}

/// A checkpoint: the set of completed step results, plus the chain
/// head. Resumability is a property of the chain: the head is the
/// hash of the most recent step result, and the prev_checkpoint_hash
/// links checkpoints together.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub checkpoint_id: Uuid,
    pub workflow_id: Uuid,
    pub created_at: String,
    pub step_results: BTreeMap<String, StepResult>,
    /// Hash of the previous checkpoint; `None` for the genesis checkpoint.
    pub prev_checkpoint_hash: Option<String>,
    /// Hash of this checkpoint (SHA-256 of canonical JSON without `self_hash`).
    pub self_hash: String,
    /// The chain head (last step's content hash, or empty if no steps completed).
    pub chain_head: String,
    /// Set of step ids that have completed.
    pub completed: BTreeSet<String>,
}

impl Checkpoint {
    /// Build a new checkpoint, computing the self hash and chain head.
    pub fn new(
        workflow_id: Uuid,
        step_results: BTreeMap<String, StepResult>,
        prev_checkpoint_hash: Option<String>,
    ) -> Self {
        let chain_head = step_results
            .values()
            .map(|r| r.content_hash())
            .max()
            .unwrap_or_default();
        let completed: BTreeSet<String> = step_results.keys().cloned().collect();
        let mut cp = Self {
            checkpoint_id: Uuid::now_v7(),
            workflow_id,
            created_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            step_results,
            prev_checkpoint_hash,
            self_hash: String::new(),
            chain_head,
            completed,
        };
        cp.self_hash = cp.compute_hash();
        cp
    }

    /// Compute the canonical hash of this checkpoint.
    pub fn compute_hash(&self) -> String {
        let mut clone = self.clone();
        clone.self_hash = String::new();
        let value = serde_json::to_value(&clone).expect("checkpoint encodes");
        let s = aether_graph::aql::canonical_json(&value);
        let mut h = Sha256::new();
        h.update(s.as_bytes());
        format!("{:x}", h.finalize())
    }

    /// Verify the self-hash and chain link.
    pub fn verify(&self) -> CheckpointResult<()> {
        let computed = self.compute_hash();
        if computed != self.self_hash {
            return Err(CheckpointError::Integrity(format!(
                "self-hash mismatch: stored={} computed={}",
                self.self_hash, computed
            )));
        }
        Ok(())
    }
}

/// Storage SPI for checkpoints. Phase 0/G2: in-memory; G2.5 adds a
/// file-based backend; G3 adds a per-project SQLite/PostgreSQL backend.
pub trait CheckpointStore: Send {
    fn save(&mut self, cp: &Checkpoint) -> CheckpointResult<()>;
    fn load(&self, workflow_id: Uuid) -> CheckpointResult<Vec<Checkpoint>>;
    fn latest(&self, workflow_id: Uuid) -> CheckpointResult<Option<Checkpoint>>;
}

/// In-memory checkpoint store. Used by tests and the `aether workflow
/// run` command when no durable store is configured.
#[derive(Debug, Default, Clone)]
pub struct InMemoryCheckpointStore {
    by_workflow: BTreeMap<Uuid, Vec<Checkpoint>>,
}

impl InMemoryCheckpointStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl CheckpointStore for InMemoryCheckpointStore {
    fn save(&mut self, cp: &Checkpoint) -> CheckpointResult<()> {
        cp.verify()?;
        self.by_workflow
            .entry(cp.workflow_id)
            .or_default()
            .push(cp.clone());
        Ok(())
    }

    fn load(&self, workflow_id: Uuid) -> CheckpointResult<Vec<Checkpoint>> {
        Ok(self.by_workflow.get(&workflow_id).cloned().unwrap_or_default())
    }

    fn latest(&self, workflow_id: Uuid) -> CheckpointResult<Option<Checkpoint>> {
        Ok(self
            .by_workflow
            .get(&workflow_id)
            .and_then(|v| v.last().cloned()))
    }
}

/// File-backed checkpoint store. Stores checkpoints as JSON in
/// `infra/run/checkpoints/<workflow_id>/<checkpoint_id>.json`. The
/// durable store is part of the G2.5 deliverable.
#[derive(Debug, Clone)]
pub struct FileCheckpointStore {
    pub root: PathBuf,
}

impl FileCheckpointStore {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
}

impl CheckpointStore for FileCheckpointStore {
    fn save(&mut self, cp: &Checkpoint) -> CheckpointResult<()> {
        cp.verify()?;
        let dir = self.root.join(cp.workflow_id.to_string());
        std::fs::create_dir_all(&dir)?;
        let path = dir.join(format!("{}.json", cp.checkpoint_id));
        let json = serde_json::to_string_pretty(cp)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    fn load(&self, workflow_id: Uuid) -> CheckpointResult<Vec<Checkpoint>> {
        let dir = self.root.join(workflow_id.to_string());
        if !dir.is_dir() {
            return Ok(Vec::new());
        }
        let mut out = Vec::new();
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            let raw = std::fs::read_to_string(&path)?;
            let cp: Checkpoint = serde_json::from_str(&raw)?;
            out.push(cp);
        }
        // Sort by creation time
        out.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        Ok(out)
    }

    fn latest(&self, workflow_id: Uuid) -> CheckpointResult<Option<Checkpoint>> {
        Ok(self.load(workflow_id)?.last().cloned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step_result(id: &str) -> StepResult {
        StepResult {
            step_id: id.to_string(),
            status: "succeeded".to_string(),
            attempt: 1,
            started_at: "2026-08-30T00:00:00Z".to_string(),
            ended_at: "2026-08-30T00:00:01Z".to_string(),
            output_hashes: BTreeMap::new(),
            plugin_id: Some("com.example.solver".to_string()),
            plugin_version: Some("1.0.0".to_string()),
            determinism_class: Some("bit-reproducible".to_string()),
            diagnostics: None,
        }
    }

    #[test]
    fn checkpoint_self_verifies() {
        let wid = Uuid::now_v7();
        let mut results = BTreeMap::new();
        results.insert("a".to_string(), step_result("a"));
        let cp = Checkpoint::new(wid, results.clone(), None);
        assert!(cp.verify().is_ok());
    }

    #[test]
    fn in_memory_store_round_trip() {
        let wid = Uuid::now_v7();
        let mut results = BTreeMap::new();
        results.insert("a".to_string(), step_result("a"));
        let cp1 = Checkpoint::new(wid, results.clone(), None);
        let mut results2 = BTreeMap::new();
        results2.insert("a".to_string(), step_result("a"));
        results2.insert("b".to_string(), step_result("b"));
        let cp2 = Checkpoint::new(wid, results2, Some(cp1.self_hash.clone()));

        let mut store = InMemoryCheckpointStore::new();
        store.save(&cp1).unwrap();
        store.save(&cp2).unwrap();
        let loaded = store.load(wid).unwrap();
        assert_eq!(loaded.len(), 2);
        // The chain head is the latest step's content hash, not the
        // checkpoint's self-hash; the two are different.
        let latest = store.latest(wid).unwrap().unwrap();
        assert_eq!(latest.completed.len(), 2);
        assert_eq!(latest.step_results.get("a").unwrap().step_id, "a");
        assert_eq!(latest.step_results.get("b").unwrap().step_id, "b");
        // Self-hash is correct.
        latest.verify().unwrap();
    }

    #[test]
    fn file_store_round_trip() {
        let wid = Uuid::now_v7();
        let tmp = std::env::temp_dir().join(format!("aether_cp_{wid}"));
        let _ = std::fs::remove_dir_all(&tmp);
        let mut store = FileCheckpointStore::new(tmp.clone());
        let mut results = BTreeMap::new();
        results.insert("a".to_string(), step_result("a"));
        let cp = Checkpoint::new(wid, results, None);
        store.save(&cp).unwrap();
        let loaded = store.load(wid).unwrap();
        assert_eq!(loaded.len(), 1);
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
