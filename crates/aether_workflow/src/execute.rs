// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! Workflow executor (G2).
//!
//! The executor runs a compiled workflow step-by-step, in topological
//! order, with retries, parameter sweeps, and checkpoints. Plugin
//! invocations are abstracted by the `StepHandler` trait; the local
//! backend (§aether_exec) is the default handler. MVR-01/02 conformance
//! (plugin replacement) is the result of using the trait: two
//! implementations of `StepHandler` for the same contract are
//! interchangeable in a workflow.

#![deny(unsafe_code)]
#![warn(missing_docs)]

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::awl::{AwlError, AwlResult, StepType};
use crate::checkpoint::{Checkpoint, CheckpointStore, StepResult};
use crate::dag::{CompiledWorkflow, StepStatus};

#[derive(Debug, Error)]
pub enum ExecError {
    #[error("awl error: {0}")]
    Awl(#[from] AwlError),
    #[error("step failed: {step_id} ({reason})")]
    StepFailed { step_id: String, reason: String },
    #[error("handler not registered for contract {0}")]
    NoHandler(String),
}

pub type ExecResult<T> = Result<T, ExecError>;

/// Handler for a step. The local backend (`aether_exec`) provides a
/// default; reference solver adapters are second implementations of
/// this trait (the two-impl rule, ADR-0022).
pub trait StepHandler: Send + Sync {
    /// A short identifier (e.g. "local", "openfoam", "calculix").
    fn name(&self) -> &str;
    /// The contract id this handler implements.
    fn contract_id(&self) -> &str;
    /// Execute a step. `attempt` is 1-indexed. The handler may return
    /// a typed `StepResult` (success) or an `Err` (failure). Retries
    /// are the executor's responsibility.
    fn execute(
        &self,
        step_id: &str,
        inputs: &serde_json::Value,
        attempt: u32,
    ) -> Result<StepResult, String>;
}

/// Configuration for the executor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutorConfig {
    /// Maximum attempts per step (the step's own `retries` overrides this).
    pub default_max_attempts: u32,
    /// Backoff between retries in seconds.
    pub default_backoff_seconds: f64,
    /// Whether to checkpoint after every step.
    pub checkpoint_after_each_step: bool,
    /// Optional `Workflow.workflow_id` for resume; if `None`, a new id
    /// is generated.
    pub workflow_id: Option<Uuid>,
}

impl Default for ExecutorConfig {
    fn default() -> Self {
        Self {
            default_max_attempts: 1,
            default_backoff_seconds: 0.0,
            checkpoint_after_each_step: true,
            workflow_id: None,
        }
    }
}

/// The result of a workflow execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub workflow_id: Uuid,
    pub started_at: String,
    pub ended_at: String,
    pub step_results: BTreeMap<String, StepResult>,
    pub chain_head: String,
    pub status: ExecutionStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    Succeeded,
    Failed { step_id: String, reason: String },
    Skipped,
}

impl ExecutionResult {
    pub fn success(&self) -> bool {
        matches!(self.status, ExecutionStatus::Succeeded)
    }
}

/// The workflow executor.
pub struct Executor {
    pub config: ExecutorConfig,
    /// Map of handler key -> handler. The key is the contract id
    /// (`aether.physics.solver`) for contract-level default handlers,
    /// or `contract::plugin` (`aether.physics.solver::org.aether.reference.fea.beam`)
    /// for plugin-level handlers. Lookup tries the plugin-qualified key
    /// first (from the AWL step's `plugin:` field) and falls back to the
    /// bare contract so single-plugin contracts keep working unchanged.
    pub handlers: BTreeMap<String, Arc<dyn StepHandler>>,
    pub store: Arc<Mutex<Box<dyn CheckpointStore>>>,
}

impl Executor {
    pub fn new(
        config: ExecutorConfig,
        store: Box<dyn CheckpointStore>,
    ) -> Self {
        Self {
            config,
            handlers: BTreeMap::new(),
            store: Arc::new(Mutex::new(store)),
        }
    }

    /// Register a handler.
    pub fn register_handler(&mut self, h: Arc<dyn StepHandler>) {
        self.handlers.insert(h.contract_id().to_string(), h);
    }

    /// Execute a compiled workflow.
    pub fn execute(&mut self, workflow: &CompiledWorkflow) -> Result<ExecutionResult> {
        let workflow_id = self.config.workflow_id.unwrap_or(workflow.workflow_id);
        let started_at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let mut step_results: BTreeMap<String, StepResult> = BTreeMap::new();
        let mut completed: BTreeSet<String> = BTreeSet::new();
        let mut last_error: Option<(String, String)> = None;

        // Resume: if the store has a checkpoint for this workflow, load it.
        {
            let store = self.store.lock().unwrap();
            if let Ok(Some(prev)) = store.latest(workflow_id) {
                step_results = prev.step_results.clone();
                completed = prev.completed.clone();
            }
        }

        let mut attempts: BTreeMap<String, u32> = BTreeMap::new();
        for step_id in &workflow.topological_order {
            if completed.contains(step_id) {
                continue;
            }
            let step = match workflow.steps.get(step_id) {
                Some(s) => s,
                None => {
                    bail!("step {step_id} not found in compiled workflow");
                }
            };
            let skip_reason: Option<&str> = if step.kind == StepType::Noop {
                Some("noop")
            } else if step.kind == StepType::Join {
                Some("join")
            } else if step.kind == StepType::Branch
                && step
                    .when
                    .as_ref()
                    .map(|when| !when.evaluate(&step.inputs))
                    .unwrap_or(true)
            {
                Some("branch_not_taken")
            } else if step.kind == StepType::Loop && step.max_iterations.unwrap_or(0) == 0 {
                Some("loop_zero_iterations")
            } else {
                None
            };
            if let Some(reason) = skip_reason {
                let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
                let result = StepResult {
                    step_id: step_id.clone(),
                    status: "succeeded".to_string(),
                    attempt: 1,
                    started_at: now.clone(),
                    ended_at: now,
                    output_hashes: BTreeMap::new(),
                    plugin_id: None,
                    plugin_version: None,
                    determinism_class: None,
                    diagnostics: Some(serde_json::json!({ "skipped": reason })),
                };
                step_results.insert(step_id.clone(), result);
                completed.insert(step_id.clone());
                self.checkpoint(workflow_id, &step_results).context("checkpoint skipped step")?;
                continue;
            }

            // Approval gates never execute a plugin. They require an
            // authorized decision supplied in the step inputs; without it
            // the workflow fails closed.
            if step.kind == StepType::Approval {
                let role = step
                    .approver_role
                    .clone()
                    .unwrap_or_else(|| "chief-engineer".to_string());
                let approved = step
                    .inputs
                    .get("approved")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let decided_by = step
                    .inputs
                    .get("approved_by_role")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
                let result = StepResult {
                    step_id: step_id.clone(),
                    status: if approved { "succeeded" } else { "failed" }.to_string(),
                    attempt: 1,
                    started_at: now.clone(),
                    ended_at: now,
                    output_hashes: BTreeMap::new(),
                    plugin_id: None,
                    plugin_version: None,
                    determinism_class: None,
                    diagnostics: Some(serde_json::json!({
                        "approval_required_role": role,
                        "approved": approved,
                        "approved_by_role": decided_by,
                    })),
                };
                let authorised = approved && decided_by == role;
                step_results.insert(step_id.clone(), result);
                if !authorised {
                    return Ok(ExecutionResult {
                        workflow_id,
                        started_at,
                        ended_at: chrono::Utc::now()
                            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                        step_results,
                        chain_head: String::new(),
                        status: ExecutionStatus::Failed {
                            step_id: step_id.clone(),
                            reason: format!(
                                "approval gate {step_id} requires role {role:?}; \
                                 got approved={approved} by {decided_by:?}"
                            ),
                        },
                    });
                }
                completed.insert(step_id.clone());
                self.checkpoint(workflow_id, &step_results)
                    .context("checkpoint approval")?;
                continue;
            }

            // Nested workflows are compiled and executed recursively with
            // their own checkpoint scope, then recorded as one step.
            if step.kind == StepType::Nested {
                let nested_path = step.nested_workflow.clone().ok_or_else(|| {
                    anyhow::anyhow!("nested step {step_id} has no nested_workflow path")
                })?;
                let raw = std::fs::read_to_string(&nested_path)
                    .with_context(|| format!("read nested workflow {nested_path}"))?;
                let nested_awl = crate::awl::Awl::parse(&raw)
                    .with_context(|| format!("parse nested workflow {nested_path}"))?;
                let nested = CompiledWorkflow::from_awl(&nested_awl)?;
                let mut nested_executor = Executor::new(
                    ExecutorConfig {
                        workflow_id: None,
                        ..self.config.clone()
                    },
                    Box::new(crate::checkpoint::InMemoryCheckpointStore::new()),
                );
                for handler in self.handlers.values() {
                    nested_executor
                        .handlers
                        .insert(handler.contract_id().to_string(), Arc::clone(handler));
                }
                let nested_result = nested_executor
                    .execute(&nested)
                    .map_err(|e| anyhow::anyhow!("nested workflow failed: {e}"))?;
                let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
                let result = StepResult {
                    step_id: step_id.clone(),
                    status: if nested_result.success() {
                        "succeeded"
                    } else {
                        "failed"
                    }
                    .to_string(),
                    attempt: 1,
                    started_at: now.clone(),
                    ended_at: now,
                    output_hashes: BTreeMap::from([(
                        "nested.chain_head".to_string(),
                        nested_result.chain_head.clone(),
                    )]),
                    plugin_id: None,
                    plugin_version: None,
                    determinism_class: None,
                    diagnostics: Some(serde_json::json!({
                        "nested_workflow": nested_path,
                        "nested_steps": nested_result.step_results.len(),
                        "nested_status": format!("{:?}", nested_result.status).to_lowercase(),
                    })),
                };
                let nested_ok = nested_result.success();
                step_results.insert(step_id.clone(), result);
                if !nested_ok {
                    return Ok(ExecutionResult {
                        workflow_id,
                        started_at,
                        ended_at: chrono::Utc::now()
                            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                        step_results,
                        chain_head: String::new(),
                        status: ExecutionStatus::Failed {
                            step_id: step_id.clone(),
                            reason: format!(
                                "nested workflow {nested_path} did not succeed"
                            ),
                        },
                    });
                }
                completed.insert(step_id.clone());
                self.checkpoint(workflow_id, &step_results)
                    .context("checkpoint nested step")?;
                continue;
            }
            // Resolve a handler
            let contract = match &step.contract {
                Some(c) => c,
                None => {
                    // No contract: treat as a no-op for now; G3+ will
                    // require a contract.
                    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
                    let result = StepResult {
                        step_id: step_id.clone(),
                        status: "succeeded".to_string(),
                        attempt: 1,
                        started_at: now.clone(),
                        ended_at: now,
                        output_hashes: BTreeMap::new(),
                        plugin_id: None,
                        plugin_version: None,
                        determinism_class: None,
                        diagnostics: None,
                    };
                    step_results.insert(step_id.clone(), result);
                    completed.insert(step_id.clone());
                    self.checkpoint(workflow_id, &step_results)?;
                    continue;
                }
            };
            // Resolve a handler for this step: prefer the plugin-
            // qualified key (contract::plugin) so a workflow can name
            // which implementation of a contract to use, then fall
            // back to the contract-level default.
            let handler = match step
                .plugin
                .as_deref()
                .and_then(|pid| self.handlers.get(&format!("{contract}::{pid}")))
                .or_else(|| self.handlers.get(contract))
            {
                Some(h) => Arc::clone(h),
                None => {
                    return Ok(ExecutionResult {
                        workflow_id,
                        started_at,
                        ended_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                        step_results,
                        chain_head: String::new(),
                        status: ExecutionStatus::Failed {
                            step_id: step_id.clone(),
                            reason: format!("no handler for contract {contract}"),
                        },
                    });
                }
            };
            let max_attempts = step
                .retries
                .as_ref()
                .map(|r| r.max_attempts)
                .unwrap_or(self.config.default_max_attempts);
            let backoff = step
                .retries
                .as_ref()
                .and_then(|r| r.backoff_seconds)
                .unwrap_or(self.config.default_backoff_seconds);
            let mut success = false;
            for attempt in 1..=max_attempts.max(1) {
                attempts.insert(step_id.clone(), attempt);
                let started = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
                match handler.execute(step_id, &step.inputs, attempt) {
                    Ok(mut result) => {
                        result.attempt = attempt;
                        result.started_at = started;
                        result.ended_at =
                            chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
                        let status = result.status.clone();
                        step_results.insert(step_id.clone(), result);
                        if status == "succeeded" {
                            success = true;
                            break;
                        }
                        // Non-success status: treat as failure, retry.
                        if backoff > 0.0 {
                            std::thread::sleep(std::time::Duration::from_secs_f64(backoff));
                        }
                    }
                    Err(e) => {
                        let ended =
                            chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
                        let result = StepResult {
                            step_id: step_id.clone(),
                            status: "failed".to_string(),
                            attempt,
                            started_at: started,
                            ended_at: ended,
                            output_hashes: BTreeMap::new(),
                            plugin_id: handler.name().to_string().into(),
                            plugin_version: None,
                            determinism_class: None,
                            diagnostics: None,
                        };
                        step_results.insert(step_id.clone(), result);
                        if attempt >= max_attempts.max(1) {
                            last_error = Some((step_id.clone(), e));
                            break;
                        }
                        if backoff > 0.0 {
                            std::thread::sleep(std::time::Duration::from_secs_f64(backoff));
                        }
                    }
                }
            }
            if !success {
                // Capture the failure.
                let reason = last_error
                    .as_ref()
                    .map(|(_, r)| r.clone())
                    .unwrap_or_else(|| "step did not succeed".to_string());
                return Ok(ExecutionResult {
                    workflow_id,
                    started_at,
                    ended_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                    step_results,
                    chain_head: String::new(),
                    status: ExecutionStatus::Failed {
                        step_id: step_id.clone(),
                        reason,
                    },
                });
            }
            completed.insert(step_id.clone());
            self.checkpoint(workflow_id, &step_results)?;
        }

        let chain_head = step_results
            .values()
            .map(|r| r.content_hash())
            .max()
            .unwrap_or_default();
        Ok(ExecutionResult {
            workflow_id,
            started_at,
            ended_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            step_results,
            chain_head,
            status: ExecutionStatus::Succeeded,
        })
    }

    fn checkpoint(
        &self,
        workflow_id: Uuid,
        step_results: &BTreeMap<String, StepResult>,
    ) -> Result<()> {
        let prev = {
            let store = self.store.lock().unwrap();
            store.latest(workflow_id).ok().flatten()
        };
        let cp = Checkpoint::new(
            workflow_id,
            step_results.clone(),
            prev.map(|p| p.self_hash.clone()),
        );
        let mut store = self.store.lock().unwrap();
        store.save(&cp).map_err(|e| anyhow::anyhow!("save checkpoint: {e}"))?;
        Ok(())
    }
}

/// A trivial handler that always succeeds; useful for tests.
pub struct AlwaysSucceedHandler {
    pub name: String,
    pub contract: String,
}

impl StepHandler for AlwaysSucceedHandler {
    fn name(&self) -> &str {
        &self.name
    }
    fn contract_id(&self) -> &str {
        &self.contract
    }
    fn execute(
        &self,
        step_id: &str,
        _inputs: &serde_json::Value,
        _attempt: u32,
    ) -> Result<StepResult, String> {
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        Ok(StepResult {
            step_id: step_id.to_string(),
            status: "succeeded".to_string(),
            attempt: 1,
            started_at: now.clone(),
            ended_at: now,
            output_hashes: BTreeMap::new(),
            plugin_id: Some(self.name.clone()),
            plugin_version: Some("test".to_string()),
            determinism_class: Some("bit-reproducible".to_string()),
            diagnostics: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::awl::Awl;
    use crate::checkpoint::InMemoryCheckpointStore;

    fn run_simple_workflow() -> ExecutionResult {
        let awl = Awl::parse(
            r#"
awl: "1.0"
steps:
  - id: a
    type: noop
  - id: b
    type: run
    contract: aether.physics.solver
    depends_on: [a]
"#,
        )
        .unwrap();
        let compiled = CompiledWorkflow::from_awl(&awl).unwrap();
        let mut ex = Executor::new(ExecutorConfig::default(), Box::new(InMemoryCheckpointStore::new()));
        ex.register_handler(Arc::new(AlwaysSucceedHandler {
            name: "test".to_string(),
            contract: "aether.physics.solver".to_string(),
        }));
        ex.execute(&compiled).unwrap()
    }

    #[test]
    fn executes_simple_workflow() {
        let r = run_simple_workflow();
        assert!(r.success(), "execution failed: {:?}", r.status);
        assert_eq!(r.step_results.len(), 2);
        assert!(r.step_results.contains_key("a"));
        assert!(r.step_results.contains_key("b"));
    }

    #[test]
    fn missing_handler_fails() {
        let awl = Awl::parse(
            r#"
awl: "1.0"
steps:
  - id: a
    type: run
    contract: aether.nonexistent.contract
"#,
        )
        .unwrap();
        let compiled = CompiledWorkflow::from_awl(&awl).unwrap();
        let mut ex = Executor::new(ExecutorConfig::default(), Box::new(InMemoryCheckpointStore::new()));
        let r = ex.execute(&compiled).unwrap();
        assert!(!r.success());
        assert!(matches!(r.status, ExecutionStatus::Failed { .. }));
    }

    #[test]
    fn branch_skipped_when_condition_is_false() {
        let awl = Awl::parse(
            r#"
awl: "1.3"
steps:
  - id: start
    type: noop
  - id: maybe
    type: branch
    when: { input: run_expensive, equals: true }
    depends_on: [start]
  - id: join
    type: join
    depends_on: [start, maybe]
"#,
        )
        .unwrap();
        let compiled = CompiledWorkflow::from_awl(&awl).unwrap();
        let mut ex = Executor::new(ExecutorConfig::default(), Box::new(InMemoryCheckpointStore::new()));
        let r = ex.execute(&compiled).unwrap();
        assert!(r.success(), "{:?}", r.status);
        assert!(r.step_results.contains_key("join"));
        // The branch is skipped and records why.
        let maybe = &r.step_results["maybe"];
        assert_eq!(
            maybe.diagnostics.as_ref().and_then(|d| d.get("skipped")),
            Some(&serde_json::json!("branch_not_taken"))
        );
    }

    #[test]
    fn zero_iteration_loop_is_skipped() {
        let awl = Awl::parse(
            r#"
awl: "1.3"
steps:
  - id: bounded
    type: loop
    max_iterations: 0
"#,
        )
        .unwrap();
        let compiled = CompiledWorkflow::from_awl(&awl).unwrap();
        let mut ex = Executor::new(ExecutorConfig::default(), Box::new(InMemoryCheckpointStore::new()));
        let r = ex.execute(&compiled).unwrap();
        assert!(r.success());
        let bounded = &r.step_results["bounded"];
        assert_eq!(
            bounded.diagnostics.as_ref().and_then(|d| d.get("skipped")),
            Some(&serde_json::json!("loop_zero_iterations"))
        );
    }

    #[test]
    fn approval_gate_requires_the_declared_role() {
        let awl = Awl::parse(
            r#"
awl: "1.3"
steps:
  - id: gate
    type: approval
    approver_role: chief-engineer
    inputs:
      approved: true
      approved_by_role: chief-engineer
"#,
        )
        .unwrap();
        let compiled = CompiledWorkflow::from_awl(&awl).unwrap();
        let mut ex = Executor::new(ExecutorConfig::default(), Box::new(InMemoryCheckpointStore::new()));
        let r = ex.execute(&compiled).unwrap();
        assert!(r.success(), "{:?}", r.status);
    }

    #[test]
    fn approval_gate_fails_closed_for_the_wrong_role() {
        let awl = Awl::parse(
            r#"
awl: "1.3"
steps:
  - id: gate
    type: approval
    approver_role: chief-engineer
    inputs:
      approved: true
      approved_by_role: analyst
"#,
        )
        .unwrap();
        let compiled = CompiledWorkflow::from_awl(&awl).unwrap();
        let mut ex = Executor::new(ExecutorConfig::default(), Box::new(InMemoryCheckpointStore::new()));
        let r = ex.execute(&compiled).unwrap();
        assert!(!r.success());
        match r.status {
            ExecutionStatus::Failed { reason, .. } => {
                assert!(reason.contains("chief-engineer"), "{reason}")
            }
            other => panic!("expected failure, got {other:?}"),
        }
    }

    #[test]
    fn nested_workflow_runs_as_one_step() {
        let dir = std::env::temp_dir().join(format!(
            "aether-nested-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let nested = dir.join("nested.awl.yaml");
        std::fs::write(
            &nested,
            r#"
awl: "1.3"
steps:
  - id: inner
    type: noop
"#,
        )
        .unwrap();
        let outer = dir.join("outer.awl.yaml");
        std::fs::write(
            &outer,
            format!(
                r#"
awl: "1.3"
steps:
  - id: sub
    type: nested
    nested_workflow: "{}"
"#,
                nested.display()
            ),
        )
        .unwrap();

        let awl = Awl::parse(&std::fs::read_to_string(&outer).unwrap()).unwrap();
        let compiled = CompiledWorkflow::from_awl(&awl).unwrap();
        let mut ex = Executor::new(ExecutorConfig::default(), Box::new(InMemoryCheckpointStore::new()));
        let r = ex.execute(&compiled).unwrap();
        assert!(r.success(), "{:?}", r.status);
        assert!(r.step_results["sub"]
            .diagnostics
            .as_ref()
            .unwrap()
            .get("nested_workflow")
            .is_some());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
