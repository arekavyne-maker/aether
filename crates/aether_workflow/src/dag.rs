// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! DAG compilation and topological execution ordering (G2).
//!
//! The DAG is the **typed** view of the workflow: nodes are step
//! instances, edges are `depends_on` references. The compiler
//! (1) validates that the graph is acyclic, (2) produces a
//! topological order, and (3) returns a `CompiledWorkflow` ready
//! for the executor to run.

#![deny(unsafe_code)]
#![warn(missing_docs)]

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::awl::{Awl, AwlError, AwlResult, RunInstance, Step, StepType};

#[derive(Debug, Error)]
pub enum DagError {
    #[error("cycle detected in workflow: {0:?}")]
    Cycle(Vec<String>),
    #[error("step {0} not found")]
    MissingStep(String),
}

pub type DagResult<T> = Result<T, DagError>;

/// A step instance ready for execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepInstance {
    /// The step id (matches the AWL `Step.id`).
    pub id: String,
    /// The step kind.
    pub kind: StepType,
    /// The plugin id (if any).
    pub plugin: Option<String>,
    /// The plugin version range (if any).
    pub plugin_version: Option<String>,
    /// The contract id (if any).
    pub contract: Option<String>,
    /// Inputs the step will receive at execution time.
    pub inputs: serde_json::Value,
    /// Outputs the step will produce.
    pub outputs: Vec<String>,
    /// Step-level resource requirements.
    pub requires: Option<crate::awl::PlacementHint>,
    /// Retry policy.
    pub retries: Option<crate::awl::RetryPolicy>,
    /// Dependencies (step ids) that must complete before this step runs.
    pub depends_on: Vec<String>,
    /// Parameter space (sweeps).
    pub parameters: Option<crate::awl::ParameterSpace>,
    /// Optional determinism class declaration.
    pub determinism: Option<String>,
    /// Optional when-condition for branch steps.
    pub when: Option<crate::awl::WhenCondition>,
    /// Optional iteration count for loop steps.
    pub max_iterations: Option<u32>,
    /// For approval steps: the role authorized to decide.
    pub approver_role: Option<String>,
    /// For nested steps: path to the nested AWL document.
    pub nested_workflow: Option<String>,
}

impl From<&Step> for StepInstance {
    fn from(s: &Step) -> Self {
        Self {
            id: s.id.clone(),
            kind: s.kind,
            plugin: s.plugin.clone(),
            plugin_version: s.plugin_version.clone(),
            contract: s.contract.clone(),
            inputs: serde_json::to_value(&s.inputs).unwrap_or(serde_json::Value::Null),
            outputs: s.outputs.clone(),
            requires: s.requires.clone(),
            retries: s.retries.clone(),
            depends_on: s.depends_on.clone(),
            parameters: s.parameters.clone(),
            determinism: s.determinism.clone(),
            when: s.when.clone(),
            max_iterations: s.max_iterations,
            approver_role: s.approver_role.clone(),
            nested_workflow: s.nested_workflow.clone(),
        }
    }
}

/// The status of a step during execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "status")]
pub enum StepStatus {
    /// Awaiting dependencies.
    Pending,
    /// Ready to run; no remaining dependencies.
    Ready,
    /// Currently executing.
    Running { attempt: u32 },
    /// Succeeded.
    Succeeded,
    /// Failed permanently.
    Failed { reason: String },
    /// Skipped because a gate evaluated false.
    Skipped { reason: String },
    /// Awaiting retry after a transient failure.
    AwaitingRetry { attempt: u32, next_attempt_at: String },
}

/// A compiled workflow: a DAG plus the topological order in which
/// steps should be considered for execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledWorkflow {
    pub workflow_id: Uuid,
    pub awl_version: String,
    pub steps: BTreeMap<String, StepInstance>,
    pub topological_order: Vec<String>,
    /// Reverse map: step id -> steps that depend on it.
    pub downstream: BTreeMap<String, Vec<String>>,
    pub inputs: serde_json::Value,
    pub reproducibility: crate::awl::ReproducibilityMode,
    pub env_manifest_hash: Option<String>,
}

impl CompiledWorkflow {
    /// Compile an AWL document into an executable workflow.
    pub fn from_awl(awl: &Awl) -> AwlResult<Self> {
        let steps: BTreeMap<String, StepInstance> = awl
            .steps
            .iter()
            .map(|s| (s.id.clone(), StepInstance::from(s)))
            .collect();

        // Build adjacency
        let mut downstream: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for step in awl.steps.iter() {
            downstream.entry(step.id.clone()).or_default();
            for dep in &step.depends_on {
                downstream.entry(dep.clone()).or_default().push(step.id.clone());
            }
        }

        let topological_order = topo_sort(&steps)?;
        let reproducibility = awl
            .reproducibility
            .as_ref()
            .map(|r| r.mode.clone())
            .unwrap_or_default();
        let env_manifest_hash = awl
            .reproducibility
            .as_ref()
            .and_then(|r| r.env_manifest.clone());

        Ok(Self {
            workflow_id: Uuid::now_v7(),
            awl_version: awl.awl.clone(),
            steps,
            topological_order,
            downstream,
            inputs: serde_json::to_value(&awl.inputs).unwrap_or(serde_json::Value::Null),
            reproducibility,
            env_manifest_hash,
        })
    }

    /// Return the steps that are immediately runnable given the
    /// completed set. A step is ready if all its dependencies are in
    /// `completed` and it has not yet been started.
    pub fn ready_steps(&self, completed: &BTreeSet<String>) -> Vec<String> {
        self.topological_order
            .iter()
            .filter(|id| !completed.contains(*id))
            .filter(|id| {
                self.steps
                    .get(*id)
                    .map(|s| s.depends_on.iter().all(|d| completed.contains(d)))
                    .unwrap_or(false)
            })
            .cloned()
            .collect()
    }

    /// Materialize the compiled workflow as a `RunInstance` for the
    /// executor.
    pub fn to_run_instance(&self) -> RunInstance {
        RunInstance {
            workflow_id: self.workflow_id,
            awl_version: self.awl_version.clone(),
            started_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            determinism: self.reproducibility.clone(),
            env_manifest_hash: self.env_manifest_hash.clone(),
            inputs: self
                .inputs
                .as_object()
                .map(|m| {
                    m.iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect()
                })
                .unwrap_or_default(),
            steps: self.topological_order.iter()
                .filter_map(|id| self.steps.get(id).cloned())
                .collect(),
        }
    }
}

fn topo_sort(steps: &BTreeMap<String, StepInstance>) -> AwlResult<Vec<String>> {
    // Compute in-degree per node
    let mut in_degree: HashMap<String, usize> = HashMap::new();
    for id in steps.keys() {
        in_degree.insert(id.clone(), 0);
    }
    for step in steps.values() {
        for _dep in &step.depends_on {
            *in_degree.entry(step.id.clone()).or_insert(0) += 1;
        }
    }
    let mut queue: VecDeque<String> = in_degree
        .iter()
        .filter(|(_, d)| **d == 0)
        .map(|(id, _)| id.clone())
        .collect();
    let mut order = Vec::new();
    let mut visited: HashSet<String> = HashSet::new();
    while let Some(id) = queue.pop_front() {
        if !visited.insert(id.clone()) {
            continue;
        }
        order.push(id.clone());
        // For each step that depends on `id`, decrement its in-degree.
        for step in steps.values() {
            if step.depends_on.iter().any(|d| d == &id) {
                let entry = in_degree.entry(step.id.clone()).or_insert(0);
                if *entry > 0 {
                    *entry -= 1;
                }
                if *entry == 0 {
                    queue.push_back(step.id.clone());
                }
            }
        }
    }
    if order.len() != steps.len() {
        // Cycle: collect remaining nodes
        let remaining: Vec<String> = steps
            .keys()
            .filter(|id| !visited.contains(*id))
            .cloned()
            .collect();
        return Err(AwlError::Semantic(format!(
            "cycle detected involving: {remaining:?}"
        )));
    }
    Ok(order)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::awl::Awl;

    fn sample() -> Awl {
        Awl::parse(
            r#"
awl: "1.0"
steps:
  - id: a
    type: noop
  - id: b
    type: noop
    depends_on: [a]
  - id: c
    type: noop
    depends_on: [a]
  - id: d
    type: noop
    depends_on: [b, c]
"#,
        )
        .unwrap()
    }

    #[test]
    fn topo_sort_respects_dependencies() {
        let c = CompiledWorkflow::from_awl(&sample()).unwrap();
        assert_eq!(c.topological_order[0], "a");
        let pos = |id: &str| c.topological_order.iter().position(|x| x == id).unwrap();
        assert!(pos("a") < pos("b"));
        assert!(pos("a") < pos("c"));
        assert!(pos("b") < pos("d"));
        assert!(pos("c") < pos("d"));
    }

    #[test]
    fn ready_steps_only_with_completed_deps() {
        let c = CompiledWorkflow::from_awl(&sample()).unwrap();
        let empty: BTreeSet<String> = BTreeSet::new();
        let mut just_a = BTreeSet::new();
        just_a.insert("a".to_string());
        let mut done = BTreeSet::new();
        done.insert("a".to_string());
        done.insert("b".to_string());
        done.insert("c".to_string());
        assert_eq!(c.ready_steps(&empty), vec!["a".to_string()]);
        assert_eq!(c.ready_steps(&just_a), vec!["b".to_string(), "c".to_string()]);
        assert_eq!(c.ready_steps(&done), vec!["d".to_string()]);
    }

    #[test]
    fn cycle_detected() {
        let bad = Awl::parse(
            r#"
awl: "1.0"
steps:
  - id: a
    type: noop
    depends_on: [b]
  - id: b
    type: noop
    depends_on: [a]
"#,
        )
        .unwrap();
        // The simple `validate` doesn't catch this, but the DAG
        // compiler must.
        let err = CompiledWorkflow::from_awl(&bad).unwrap_err();
        let s = format!("{err:?}");
        assert!(s.contains("cycle") || s.contains("Semantic"), "got: {s}");
    }
}
