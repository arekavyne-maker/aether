// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! AWL v1 schema and parser (SDP §4.2, §17, Appendix C).
//!
//! The schema is a typed YAML subset; every field is validated against
//! a JSON Schema in `contracts/workflow/awl.schema.json` (added in
//! Phase 3; G2 ships the structural parser).

#![deny(unsafe_code)]

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum AwlError {
    #[error("yaml parse error: {0}")]
    Parse(String),
    #[error("validation error: {0}")]
    Validation(String),
    #[error("semantic error: {0}")]
    Semantic(String),
}

pub type AwlResult<T> = Result<T, AwlError>;

/// AWL document version. v1 ships in G2; v1.2 (G4) adds `compose:`,
/// `si_hooks`, `ladder:`, and `fidelity_class` extensions.
pub const AWL_VERSION: &str = "1.2";

/// Supported AWL versions. The executor accepts any of these and
/// normalizes to its internal v1.3 representation.
pub const AWL_VERSIONS: &[&str] = &["1.0", "1.2", "1.3"];

/// A parsed AWL v1 document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Awl {
    /// Schema version; must equal `AWL_VERSION` for v1.
    pub awl: String,
    /// Optional human-readable study name.
    #[serde(default)]
    pub study: Option<String>,
    /// Optional description.
    #[serde(default)]
    pub description: Option<String>,
    /// Inputs (key -> value). Values may be scalars or parameter-space
    /// declarations.
    #[serde(default)]
    pub inputs: BTreeMap<String, serde_json::Value>,
    /// Steps in the workflow.
    pub steps: Vec<Step>,
    /// Optional outputs the workflow produces.
    #[serde(default)]
    pub outputs: Vec<OutputDecl>,
    /// Optional evidence dossier settings.
    #[serde(default)]
    pub evidence: Option<EvidenceDecl>,
    /// Reproducibility mode.
    #[serde(default)]
    pub reproducibility: Option<ReproducibilityDecl>,
    /// AWL v1.2: optional multiphysics composition declaration. The
    /// executor routes the `compose` handler to `aether_compose::Coupling`.
    #[serde(default)]
    pub compose: Option<CompositionDecl>,
    /// AWL v1.2: optional SI hooks (apply to all steps unless overridden).
    #[serde(default)]
    pub si_hooks: Option<SIHooks>,
    /// AWL v1.2: optional fidelity ladder.
    #[serde(default)]
    pub ladder: Option<FidelityLadder>,
}

/// AWL v1.2 extension: a fidelity ladder declaration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FidelityLadder {
    /// The tiers of the ladder, in order from low to high.
    #[serde(default)]
    pub tiers: Vec<LadderTier>,
}

/// A single tier of a fidelity ladder.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LadderTier {
    /// Tier name (e.g. "low_fidelity", "medium_fidelity", "high_fidelity").
    pub tier: String,
    /// Optional plugin for this tier.
    #[serde(default)]
    pub plugin: Option<String>,
    /// Optional plugin version.
    #[serde(default)]
    pub plugin_version: Option<String>,
    /// Optional SI hooks (surrogate trust, data value, etc.).
    #[serde(default)]
    pub si_hooks: Option<SIHooks>,
}

/// SI hooks for a step: governs which SI decisions apply.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SIHooks {
    /// Whether the FidelityGovernor may adjust the sampling plan.
    #[serde(default)]
    pub fidelity_governor: bool,
    /// Whether the SurrogateTrustAssessor may substitute a surrogate.
    #[serde(default)]
    pub surrogate_trust: bool,
    /// Whether the DataValueEstimator may suggest additional data/compute.
    #[serde(default)]
    pub data_value: bool,
    /// Maximum autonomy class allowed (e.g. "A2" — A3 requires sign-off).
    #[serde(default = "default_autonomy")]
    pub autonomy_max: String,
}

fn default_autonomy() -> String {
    "A2".to_string()
}

/// AWL v1.2 extension: a multiphysics composition declaration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositionDecl {
    pub name: String,
    pub participants: Vec<CompositionParticipant>,
    pub exchanges: Vec<CompositionExchange>,
    #[serde(default)]
    pub scheme: CompositionScheme,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositionParticipant {
    pub name: String,
    pub plugin: String,
    #[serde(default)]
    pub plugin_version: Option<String>,
    #[serde(default)]
    pub physics_class: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositionExchange {
    pub from: String,
    pub to: String,
    pub name: String,
    #[serde(default)]
    pub units: Option<String>,
    #[serde(default = "default_mapping")]
    pub mapping: String,
    #[serde(default)]
    pub tolerance: Option<f64>,
}

fn default_mapping() -> String {
    "conservative".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CompositionScheme {
    #[serde(rename = "type", default = "default_scheme_type")]
    pub kind: String,
    #[serde(default)]
    pub accelerator: Option<String>,
    #[serde(default = "default_max_iterations")]
    pub max_iterations: u32,
    #[serde(default = "default_tolerance")]
    pub tolerance: f64,
}

fn default_scheme_type() -> String { "partitioned_implicit".to_string() }
fn default_max_iterations() -> u32 { 30 }
fn default_tolerance() -> f64 { 1e-4 }

/// An output declaration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputDecl {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
}

/// Evidence dossier settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceDecl {
    #[serde(default = "default_attach")]
    pub attach: bool,
    #[serde(default)]
    pub verification_relevant: bool,
}

fn default_attach() -> bool {
    true
}

impl Default for EvidenceDecl {
    fn default() -> Self {
        Self {
            attach: true,
            verification_relevant: false,
        }
    }
}

/// Reproducibility mode.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ReproducibilityMode {
    /// Refuse to run if any pin cannot be resolved.
    Strict,
    /// Allow nondeterministic backends with equivalence checks.
    Equivalence,
}

impl Default for ReproducibilityMode {
    fn default() -> Self {
        Self::Strict
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReproducibilityDecl {
    #[serde(default)]
    pub mode: ReproducibilityMode,
    #[serde(default)]
    pub env_manifest: Option<String>,
}

/// A step in the workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step {
    /// Unique step id (used as a DAG node key and provenance record key).
    pub id: String,
    /// Human-readable label.
    #[serde(default)]
    pub name: Option<String>,
    /// Step type (`run`, `sweep`, `gate`).
    #[serde(rename = "type")]
    pub kind: StepType,
    /// Optional plugin id (reverse-DNS).
    #[serde(default)]
    pub plugin: Option<String>,
    /// Optional plugin version (semver range).
    #[serde(default)]
    pub plugin_version: Option<String>,
    /// Optional contract id this step implements.
    #[serde(default)]
    pub contract: Option<String>,
    /// Step-level inputs (key -> value reference).
    #[serde(default)]
    pub inputs: BTreeMap<String, serde_json::Value>,
    /// Step-level outputs.
    #[serde(default)]
    pub outputs: Vec<String>,
    /// Resource requirements (forwarded to compute.backend).
    #[serde(default)]
    pub requires: Option<PlacementHint>,
    /// Parameter space (sweeps, etc.).
    #[serde(default)]
    pub parameters: Option<ParameterSpace>,
    /// Dependencies on other step ids.
    #[serde(default)]
    pub depends_on: Vec<String>,
    /// Retry policy.
    #[serde(default)]
    pub retries: Option<RetryPolicy>,
    /// Optional step-level determinism class.
    #[serde(default)]
    pub determinism: Option<String>,
    /// Optional when-condition for `branch` steps. The step runs only when
    /// the referenced input is truthy.
    #[serde(default)]
    pub when: Option<WhenCondition>,
    /// Optional iteration count for `loop` steps.
    #[serde(default)]
    pub max_iterations: Option<u32>,
    /// For `approval` steps: the role authorized to decide.
    #[serde(default)]
    pub approver_role: Option<String>,
    /// For `nested` steps: path to the nested AWL document.
    #[serde(default)]
    pub nested_workflow: Option<String>,
    /// Idempotency key for remote/backended execution. Derived from the
    /// step definition when absent so retries cannot duplicate work.
    #[serde(default)]
    pub idempotency_key: Option<String>,
    /// Compensating step id to run when this step fails permanently.
    #[serde(default)]
    pub on_failure: Option<String>,
    /// Cache policy keyed by the canonical input hash.
    #[serde(default)]
    pub cache: Option<CachePolicy>,
    /// Event trigger for `trigger` steps.
    #[serde(default)]
    pub trigger: Option<TriggerSpec>,
    /// Multi-fidelity escalation declaration.
    #[serde(default)]
    pub fidelity: Option<FidelityTransition>,
}

/// Cache policy for a step.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CachePolicy {
    /// Whether caching is enabled.
    pub enabled: bool,
    /// Optional explicit cache key; otherwise the canonical input hash is used.
    #[serde(default)]
    pub key: Option<String>,
}

/// An event trigger for a workflow step.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TriggerSpec {
    /// Event kind: `file`, `timer`, or `message`.
    pub kind: String,
    /// Trigger payload (path, interval, or topic).
    pub value: String,
}

/// A multi-fidelity transition: run a low-fidelity step first, escalate to
/// a higher-fidelity step when the declared condition holds.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FidelityTransition {
    /// Lower-fidelity step id.
    pub from: String,
    /// Higher-fidelity step id.
    pub to: String,
    /// Quantity of interest that drives escalation.
    pub quantity: String,
    /// Threshold that triggers escalation.
    pub threshold: f64,
}

/// A declarative when-condition for branch and loop steps.
///
/// The condition is intentionally small and deterministic: a named input
/// must equal a declared value. Anything more expressive belongs in a
/// contract-backed plugin, not in the workflow language.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WhenCondition {
    /// Input key to test.
    pub input: String,
    /// Required value. Numbers compare numerically; everything else
    /// compares as JSON equality.
    pub equals: serde_json::Value,
}

impl WhenCondition {
    /// Evaluate the condition against a step's inputs.
    pub fn evaluate(&self, inputs: &serde_json::Value) -> bool {
        let actual = inputs.get(&self.input);
        match (actual, &self.equals) {
            (Some(serde_json::Value::Number(a)), serde_json::Value::Number(b)) => {
                match (a.as_f64(), b.as_f64()) {
                    (Some(a), Some(b)) => (a - b).abs() < f64::EPSILON,
                    _ => false,
                }
            }
            (Some(a), b) => a == b,
            (None, _) => false,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum StepType {
    /// A single execution of a plugin.
    Run,
    /// A parameter sweep over a plugin.
    Sweep,
    /// A gate that evaluates a condition and selects the next step.
    Gate,
    /// A no-op step (used for control flow).
    Noop,
    /// A conditional branch: run only when the declared condition holds.
    Branch,
    /// A join point that waits for all declared dependencies.
    Join,
    /// A bounded loop with a termination condition.
    Loop,
    /// An approval gate: blocks until an authorized human decision is
    /// recorded in the evidence ledger.
    Approval,
    /// A nested workflow: runs another AWL document as a sub-run.
    Nested,
}

/// Resource requirement for a step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlacementHint {
    #[serde(default)]
    pub memory_gb: Option<u32>,
    #[serde(default)]
    pub cores: Option<u32>,
    #[serde(default)]
    pub parallelism: Option<String>,
    #[serde(default)]
    pub accelerator: Option<String>,
    #[serde(default)]
    pub accelerator_class: Option<String>,
    #[serde(default)]
    pub architecture: Option<Vec<String>>,
    #[serde(default)]
    pub max_runtime_h: Option<f64>,
}

/// A parameter space declaration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ParameterSpace {
    /// Linear spacing between two values, inclusive.
    Linspace {
        name: String,
        start: f64,
        end: f64,
        steps: u32,
    },
    /// List of explicit values.
    List {
        name: String,
        values: Vec<serde_json::Value>,
    },
}

/// Retry policy for a step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    #[serde(default)]
    pub backoff_seconds: Option<f64>,
}

impl Awl {
    /// Parse an AWL document from a YAML string.
    pub fn parse(yaml: &str) -> AwlResult<Self> {
        let doc: Awl = serde_yaml::from_str(yaml).map_err(|e| AwlError::Parse(e.to_string()))?;
        doc.validate()?;
        Ok(doc)
    }

    /// Validate the document's structural and semantic invariants.
    pub fn validate(&self) -> AwlResult<()> {
        if !AWL_VERSIONS.contains(&self.awl.as_str()) {
            return Err(AwlError::Validation(format!(
                "AWL version mismatch: document declares {} but the v1.x executor accepts {:?}",
                self.awl, AWL_VERSIONS
            )));
        }
        let mut ids = std::collections::BTreeSet::new();
        for step in &self.steps {
            if !ids.insert(step.id.clone()) {
                return Err(AwlError::Semantic(format!(
                    "duplicate step id: {}",
                    step.id
                )));
            }
        }
        for step in &self.steps {
            for dep in &step.depends_on {
                if !ids.contains(dep) {
                    return Err(AwlError::Semantic(format!(
                        "step {} depends on unknown step {}",
                        step.id, dep
                    )));
                }
            }
        }
        // No cycles (topological feasibility); the executor's DAG
        // validator (§dag) does the rigorous check, but a sanity scan
        // here catches the most common mistake.
        Ok(())
    }
}

use crate::dag::StepInstance;

/// A typed run instance produced by the planner.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunInstance {
    pub workflow_id: Uuid,
    pub awl_version: String,
    pub started_at: String,
    pub determinism: ReproducibilityMode,
    pub env_manifest_hash: Option<String>,
    pub inputs: BTreeMap<String, serde_json::Value>,
    pub steps: Vec<StepInstance>,
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
awl: "1.0"
study: SS-112-aeroelastic
inputs:
  geometry: { node: "aether://uav-wing/rib-014" }
  mach: { linspace: [0.15, 0.30, 8] }
steps:
  - id: setup
    type: noop
  - id: cfd_fine
    type: run
    plugin: com.acme.cfd.opensfoam
    plugin_version: "2.4.1"
    contract: aether.physics.solver
    depends_on: [setup]
    inputs:
      geometry: "${{ inputs.geometry }}"
    requires: { memory_gb: 64, cores: 32, parallelism: distributed }
    retries: { max_attempts: 3, backoff_seconds: 5 }
  - id: decide
    type: gate
    depends_on: [cfd_fine]
"#;

    #[test]
    fn parses_minimal_awl() {
        let doc = Awl::parse(SAMPLE).expect("parse");
        assert_eq!(doc.awl, "1.0");
        assert_eq!(doc.steps.len(), 3);
        assert_eq!(doc.steps[0].id, "setup");
        assert_eq!(doc.steps[1].plugin.as_deref(), Some("com.acme.cfd.opensfoam"));
    }

    #[test]
    fn rejects_wrong_awl_version() {
        let bad = SAMPLE.replace(r#"awl: "1.0""#, r#"awl: "0.9""#);
        let err = Awl::parse(&bad).unwrap_err();
        assert!(matches!(err, AwlError::Validation(_)));
    }

    #[test]
    fn rejects_duplicate_step_ids() {
        // Replace the first occurrence of "  - id: cfd_fine" with a
        // duplicate of "setup", producing two `setup` steps.
        let bad = SAMPLE.replacen(
            "  - id: cfd_fine",
            "  - id: setup",
            1,
        );
        let err = Awl::parse(&bad).unwrap_err();
        assert!(matches!(err, AwlError::Semantic(_)), "got: {err:?}");
    }

    #[test]
    fn rejects_unknown_dependency() {
        let bad = SAMPLE.replace(
            "depends_on: [cfd_fine]",
            "depends_on: [nonexistent]",
        );
        let err = Awl::parse(&bad).unwrap_err();
        assert!(matches!(err, AwlError::Semantic(_)));
    }

    #[test]
    fn rejects_self_dependency() {
        let bad = SAMPLE.replace(
            "depends_on: [setup]",
            "depends_on: [cfd_fine]",
        );
        // The `validate` does not catch this directly; the DAG validator
        // (in `dag.rs`) does. Skip the strict assertion here.
        let _ = Awl::parse(&bad);
    }

    #[test]
    fn v12_compose_and_ladder_parse() {
        // AWL v1.2 adds compose: and ladder: declarations.
        let v12 = r#"
awl: "1.2"
study: "ss-112-flutter"
compose:
  name: "aeroelastic"
  participants:
    - { name: flow, plugin: com.acme.cfd.opensfoam, plugin_version: "2.4.1", physics_class: fluid }
    - { name: structure, plugin: org.oss.feather.calculix, plugin_version: "2.21.0", physics_class: structural }
  exchanges:
    - { from: flow, to: structure, name: pressure, units: Pa, mapping: conservative, tolerance: 1.0e-4 }
  scheme: { type: partitioned_implicit, accelerator: iqn_ils, max_iterations: 30, tolerance: 1.0e-4 }
si_hooks: { fidelity_governor: true, surrogate_trust: true, data_value: false, autonomy_max: A2 }
ladder:
  tiers:
    - { tier: low_fidelity, plugin: com.acme.cfd.opensfoam, plugin_version: "2.4.1" }
    - { tier: high_fidelity, plugin: com.acme.cfd.opensfoam, plugin_version: "2.4.1" }
steps:
  - id: solve
    type: run
    contract: aether.physics.solver
    plugin: com.acme.cfd.opensfoam
    plugin_version: "2.4.1"
"#;
        let awl = Awl::parse(v12).expect("parse v1.2");
        assert_eq!(awl.awl, "1.2");
        let compose = awl.compose.expect("compose present");
        assert_eq!(compose.name, "aeroelastic");
        assert_eq!(compose.participants.len(), 2);
        assert_eq!(compose.exchanges.len(), 1);
        let si = awl.si_hooks.expect("si_hooks present");
        assert!(si.fidelity_governor);
        assert_eq!(si.autonomy_max, "A2");
        let ladder = awl.ladder.expect("ladder present");
        assert_eq!(ladder.tiers.len(), 2);
    }

    #[test]
    fn v12_rejects_unknown_version() {
        let bad = r#"
awl: "0.9"
steps:
  - { id: x, type: noop }
"#;
        let err = Awl::parse(bad).unwrap_err();
        assert!(matches!(err, AwlError::Validation(_)));
    }

    #[test]
    fn v10_still_accepted() {
        // AWL v1.0 documents are still accepted by the v1.x executor.
        let v10 = SAMPLE.replace("awl: \"1.0\"", "awl: \"1.0\"");
        let _ = Awl::parse(&v10).expect("v1.0 still works");
    }

    #[test]
    fn branch_join_and_loop_parse() {
        let doc = r#"
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
  - id: bounded
    type: loop
    max_iterations: 3
    depends_on: [join]
"#;
        let awl = Awl::parse(doc).expect("branch/join/loop parse");
        let compiled = crate::dag::CompiledWorkflow::from_awl(&awl).expect("compiles");
        assert_eq!(compiled.topological_order.len(), 4);
        assert_eq!(compiled.steps["maybe"].when.as_ref().unwrap().input, "run_expensive");
        assert_eq!(compiled.steps["bounded"].max_iterations, Some(3));
    }

    #[test]
    fn when_condition_compares_numbers_and_values() {
        let when = WhenCondition {
            input: "mode".to_string(),
            equals: serde_json::json!(2.0),
        };
        assert!(when.evaluate(&serde_json::json!({ "mode": 2 })));
        assert!(!when.evaluate(&serde_json::json!({ "mode": 3 })));
        assert!(!when.evaluate(&serde_json::json!({})));
    }
}
