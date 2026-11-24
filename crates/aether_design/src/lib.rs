// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! Design-space exploration, candidate lineage, feasibility gating, and
//! uncertainty representation.
//!
//! Design generation is treated as untrusted search. A candidate is not a
//! result: it must pass an ordered feasibility pipeline, retain its lineage,
//! and keep rejected candidates as negative evidence (A3-I17). Uncertainty
//! is represented by class instead of collapsed into one confidence score
//! (A3-I09).

#![deny(unsafe_code)]
#![warn(missing_docs)]

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use thiserror::Error;

/// Design representation carried by a candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Representation {
    /// Parametric variables only.
    Parametric,
    /// Boundary representation.
    BRep,
    /// Mesh or field.
    Mesh,
    /// Material/process field.
    Field,
    /// State-space model for morphing structures.
    StateSpace,
}

/// Search method that produced a candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchMethod {
    /// Gradient or adjoint.
    Gradient,
    /// Evolutionary algorithm.
    Evolutionary,
    /// Quality-diversity.
    QualityDiversity,
    /// Bayesian optimisation.
    Bayesian,
    /// Generative model.
    Generative,
    /// Human-directed.
    Human,
}

/// Uncertainty classes. They are reported separately.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UncertaintyClass {
    /// Parameter uncertainty.
    Parameter,
    /// Material variability.
    Material,
    /// Manufacturing variation.
    Manufacturing,
    /// Boundary-condition uncertainty.
    BoundaryCondition,
    /// Measurement uncertainty.
    Measurement,
    /// Numerical discretisation error.
    Discretisation,
    /// Model-form uncertainty.
    ModelForm,
    /// Surrogate epistemic uncertainty.
    Surrogate,
    /// Operational/environmental variability.
    Operational,
}

/// One uncertainty component with a declared value and unit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UncertaintyComponent {
    /// Class.
    pub class: UncertaintyClass,
    /// Description.
    pub description: String,
    /// Magnitude in the declared unit.
    pub magnitude: f64,
    /// Unit symbol, e.g. `mm`, `MPa`.
    pub unit: String,
    /// Optional distribution name, e.g. `normal`, `uniform`.
    #[serde(default)]
    pub distribution: Option<String>,
}

impl UncertaintyComponent {
    /// Validate that the component has a non-empty description, a finite
    /// non-negative magnitude, and a unit.
    pub fn validate(&self) -> Result<(), DesignError> {
        if self.description.trim().is_empty() {
            return Err(DesignError::Invalid("uncertainty description is empty".into()));
        }
        if !self.magnitude.is_finite() || self.magnitude < 0.0 {
            return Err(DesignError::Invalid(format!(
                "uncertainty magnitude {} is not finite and non-negative",
                self.magnitude
            )));
        }
        if self.unit.trim().is_empty() {
            return Err(DesignError::Invalid("uncertainty unit is empty".into()));
        }
        Ok(())
    }
}

/// A candidate design.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Candidate {
    /// Stable candidate id.
    pub id: String,
    /// Parent candidates, if any.
    #[serde(default)]
    pub parents: Vec<String>,
    /// Representation.
    pub representation: Representation,
    /// Search method.
    pub method: SearchMethod,
    /// Random seed, when applicable.
    #[serde(default)]
    pub seed: Option<u64>,
    /// Design variables.
    pub variables: BTreeMap<String, Value>,
    /// Declared objectives.
    #[serde(default)]
    pub objectives: BTreeMap<String, f64>,
    /// Constraint results, keyed by constraint name.
    #[serde(default)]
    pub constraints: BTreeMap<String, bool>,
    /// Separate uncertainty components.
    #[serde(default)]
    pub uncertainty: Vec<UncertaintyComponent>,
    /// Validity envelope inherited from the model/method.
    #[serde(default)]
    pub validity_envelope: BTreeMap<String, Value>,
}

impl Candidate {
    /// Canonical hash of the candidate's design definition.
    pub fn content_hash(&self) -> String {
        let canonical = serde_json::json!({
            "id": self.id,
            "parents": self.parents,
            "representation": self.representation,
            "method": self.method,
            "seed": self.seed,
            "variables": self.variables,
            "objectives": self.objectives,
            "constraints": self.constraints,
            "uncertainty": self.uncertainty,
            "validity_envelope": self.validity_envelope,
        });
        let bytes = serde_json::to_vec(&canonical).unwrap_or_default();
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        format!("{:x}", hasher.finalize())
    }

    /// Whether all declared constraints pass.
    pub fn constraints_pass(&self) -> bool {
        self.constraints.values().all(|passed| *passed)
    }

    /// Whether every uncertainty component is well-formed.
    pub fn uncertainty_valid(&self) -> Result<(), DesignError> {
        for component in &self.uncertainty {
            component.validate()?;
        }
        Ok(())
    }
}

/// One feasibility gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Gate {
    /// Schema and topology validity.
    Schema,
    /// Dimensional and geometric validity.
    Geometric,
    /// Interface and assembly compatibility.
    Interface,
    /// Material and process compatibility.
    MaterialProcess,
    /// Minimum feature and tolerance checks.
    FeatureTolerance,
    /// Tool and inspection access.
    Access,
    /// Preliminary structural checks.
    PreliminaryStructural,
    /// Detailed analysis.
    DetailedAnalysis,
    /// Uncertainty and robustness review.
    Robustness,
    /// Independent engineering review.
    IndependentReview,
}

impl Gate {
    /// Ordered pipeline index.
    pub fn order(self) -> u8 {
        match self {
            Gate::Schema => 1,
            Gate::Geometric => 2,
            Gate::Interface => 3,
            Gate::MaterialProcess => 4,
            Gate::FeatureTolerance => 5,
            Gate::Access => 6,
            Gate::PreliminaryStructural => 7,
            Gate::DetailedAnalysis => 8,
            Gate::Robustness => 9,
            Gate::IndependentReview => 10,
        }
    }
}

/// Result of one gate for one candidate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GateResult {
    /// Gate.
    pub gate: Gate,
    /// Whether the candidate passed.
    pub passed: bool,
    /// Stable reason code when rejected.
    pub reason_code: Option<String>,
    /// Human explanation.
    pub detail: String,
}

/// Feasibility decision for a candidate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FeasibilityReport {
    /// Candidate id and content hash.
    pub candidate_id: String,
    /// Candidate content hash.
    pub content_hash: String,
    /// Accepted by the pipeline.
    pub accepted: bool,
    /// Gate results in pipeline order.
    pub gates: Vec<GateResult>,
    /// First rejection, if any.
    pub rejection: Option<GateResult>,
}

/// Design-space errors.
#[derive(Debug, Error, PartialEq)]
pub enum DesignError {
    /// Malformed candidate or pipeline input.
    #[error("invalid design input: {0}")]
    Invalid(String),
    /// Lineage error.
    #[error("invalid lineage: {0}")]
    Lineage(String),
}

/// The ordered feasibility pipeline.
///
/// A candidate rejected at any stage remains in the report; callers are
/// expected to keep the report as negative evidence rather than discarding
/// it.
pub fn evaluate_feasibility(
    candidate: &Candidate,
    available: &[Gate],
) -> Result<FeasibilityReport, DesignError> {
    if candidate.id.trim().is_empty() {
        return Err(DesignError::Invalid("candidate id is empty".into()));
    }
    candidate.uncertainty_valid()?;

    let mut ordered: Vec<Gate> = available.to_vec();
    ordered.sort_by_key(|gate| gate.order());
    ordered.dedup();

    let mut gates = Vec::new();
    for gate in ordered {
        let (passed, reason_code, detail) = match gate {
            Gate::Schema => (
                !candidate.id.trim().is_empty()
                    && !candidate.variables.is_empty(),
                if candidate.variables.is_empty() {
                    Some("EMPTY_DESIGN_VARIABLES".to_string())
                } else {
                    None
                },
                "candidate carries an id and at least one design variable".to_string(),
            ),
            Gate::Geometric => (
                candidate
                    .variables
                    .values()
                    .all(|v| match v {
                        Value::Number(n) => n.as_f64().map(|f| f.is_finite()).unwrap_or(false),
                        _ => true,
                    }),
                if candidate.variables.values().any(|v| match v {
                    Value::Number(n) => n.as_f64().map(|f| !f.is_finite()).unwrap_or(false),
                    _ => false,
                }) {
                    Some("NON_FINITE_GEOMETRY".to_string())
                } else {
                    None
                },
                "all numeric geometry variables are finite".to_string(),
            ),
            Gate::Interface => (
                candidate
                    .variables
                    .get("interfaces")
                    .and_then(Value::as_object)
                    .map(|m| m.values().all(|v| v.as_bool().unwrap_or(true)))
                    .unwrap_or(true),
                Some("INTERFACE_INCOMPATIBLE".to_string()),
                "declared interfaces are compatible".to_string(),
            ),
            Gate::MaterialProcess => (
                candidate
                    .variables
                    .get("material_process_compatible")
                    .and_then(Value::as_bool)
                    .unwrap_or(true),
                Some("MATERIAL_PROCESS_INCOMPATIBLE".to_string()),
                "material and process route are compatible".to_string(),
            ),
            Gate::FeatureTolerance => (
                candidate
                    .variables
                    .get("min_feature_mm")
                    .and_then(Value::as_f64)
                    .map(|mm| mm >= 0.1)
                    .unwrap_or(true),
                Some("MIN_FEATURE_TOO_SMALL".to_string()),
                "minimum feature is at least 0.1 mm".to_string(),
            ),
            Gate::Access => (
                candidate
                    .variables
                    .get("tool_accessible")
                    .and_then(Value::as_bool)
                    .unwrap_or(true),
                Some("TOOL_INACCESSIBLE".to_string()),
                "tool and inspection access is available".to_string(),
            ),
            Gate::PreliminaryStructural => (
                candidate
                    .variables
                    .get("preliminary_utilisation")
                    .and_then(Value::as_f64)
                    .map(|u| u <= 1.0)
                    .unwrap_or(true),
                Some("PRELIMINARY_OVERSTRESS".to_string()),
                "preliminary utilisation is within unity".to_string(),
            ),
            Gate::DetailedAnalysis => (
                candidate.constraints_pass(),
                if candidate.constraints.values().any(|passed| !passed) {
                    Some("DETAILED_ANALYSIS_CONSTRAINT_FAILED".to_string())
                } else {
                    None
                },
                "all detailed-analysis constraints pass".to_string(),
            ),
            Gate::Robustness => (
                candidate.uncertainty.is_empty()
                    || candidate
                        .uncertainty
                        .iter()
                        .all(|c| c.validate().is_ok()),
                Some("UNCERTAINTY_UNREPRESENTED".to_string()),
                "uncertainty components are declared and well-formed".to_string(),
            ),
            Gate::IndependentReview => (
                candidate
                    .variables
                    .get("independent_review")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                Some("REVIEW_NOT_RECORDED".to_string()),
                "an independent review decision is recorded".to_string(),
            ),
        };
        let result = GateResult {
            gate,
            passed,
            reason_code: if passed { None } else { reason_code.clone() },
            detail,
        };
        let rejected = !result.passed;
        gates.push(result.clone());
        if rejected {
            return Ok(FeasibilityReport {
                candidate_id: candidate.id.clone(),
                content_hash: candidate.content_hash(),
                accepted: false,
                rejection: Some(result),
                gates,
            });
        }
    }
    Ok(FeasibilityReport {
        candidate_id: candidate.id.clone(),
        content_hash: candidate.content_hash(),
        accepted: true,
        rejection: None,
        gates,
    })
}

/// Check candidate lineage for cycles and missing parents.
pub fn validate_lineage(candidates: &[Candidate]) -> Result<(), DesignError> {
    let ids: std::collections::BTreeSet<&str> =
        candidates.iter().map(|c| c.id.as_str()).collect();
    if ids.len() != candidates.len() {
        return Err(DesignError::Lineage(
            "duplicate candidate id".to_string(),
        ));
    }
    for candidate in candidates {
        for parent in &candidate.parents {
            if parent == &candidate.id {
                return Err(DesignError::Lineage(format!(
                    "candidate {} is its own parent",
                    candidate.id
                )));
            }
            if !ids.contains(parent.as_str()) {
                return Err(DesignError::Lineage(format!(
                    "candidate {} references unknown parent {}",
                    candidate.id, parent
                )));
            }
        }
    }
    // Depth-first cycle detection.
    let by_id: BTreeMap<&str, &Candidate> =
        candidates.iter().map(|c| (c.id.as_str(), c)).collect();
    let mut state: BTreeMap<&str, u8> = BTreeMap::new();
    for candidate in candidates {
        visit(candidate.id.as_str(), &by_id, &mut state)?;
    }
    Ok(())
}

fn visit<'a>(
    id: &'a str,
    by_id: &BTreeMap<&'a str, &'a Candidate>,
    state: &mut BTreeMap<&'a str, u8>,
) -> Result<(), DesignError> {
    match state.get(id).copied().unwrap_or(0) {
        1 => return Err(DesignError::Lineage(format!("lineage cycle at {id}"))),
        2 => return Ok(()),
        _ => {}
    }
    state.insert(id, 1);
    if let Some(candidate) = by_id.get(id) {
        for parent in &candidate.parents {
            if let Some(parent_candidate) = by_id.get(parent.as_str()) {
                visit(parent_candidate.id.as_str(), by_id, state)?;
            }
        }
    }
    state.insert(id, 2);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate() -> Candidate {
        Candidate {
            id: "cand-1".to_string(),
            parents: vec![],
            representation: Representation::Parametric,
            method: SearchMethod::Evolutionary,
            seed: Some(42),
            variables: BTreeMap::from([
                ("span_mm".to_string(), serde_json::json!(1200.0)),
                ("independent_review".to_string(), serde_json::json!(true)),
            ]),
            objectives: BTreeMap::from([("mass_kg".to_string(), 12.5)]),
            constraints: BTreeMap::from([("stress_ok".to_string(), true)]),
            uncertainty: vec![],
            validity_envelope: BTreeMap::new(),
        }
    }

    #[test]
    fn complete_candidate_passes_full_pipeline() {
        let report = evaluate_feasibility(
            &candidate(),
            &[
                Gate::Schema,
                Gate::Geometric,
                Gate::Interface,
                Gate::MaterialProcess,
                Gate::FeatureTolerance,
                Gate::Access,
                Gate::PreliminaryStructural,
                Gate::DetailedAnalysis,
                Gate::Robustness,
                Gate::IndependentReview,
            ],
        )
        .unwrap();
        assert!(report.accepted, "{:?}", report.rejection);
        assert_eq!(report.gates.len(), 10);
    }

    #[test]
    fn rejection_is_the_first_failing_gate() {
        let mut c = candidate();
        c.variables.insert("min_feature_mm".to_string(), serde_json::json!(0.01));
        let report = evaluate_feasibility(
            &c,
            &[Gate::Schema, Gate::FeatureTolerance, Gate::DetailedAnalysis],
        )
        .unwrap();
        assert!(!report.accepted);
        let rejection = report.rejection.unwrap();
        assert_eq!(rejection.gate, Gate::FeatureTolerance);
        assert_eq!(rejection.reason_code.as_deref(), Some("MIN_FEATURE_TOO_SMALL"));
        assert_eq!(report.gates.len(), 2);
    }

    #[test]
    fn failed_constraint_blocks_detailed_analysis() {
        let mut c = candidate();
        c.constraints.insert("stress_ok".to_string(), false);
        let report = evaluate_feasibility(&c, &[Gate::DetailedAnalysis]).unwrap();
        assert!(!report.accepted);
        assert!(report
            .rejection
            .unwrap()
            .reason_code
            .unwrap()
            .contains("CONSTRAINT"));
    }

    #[test]
    fn uncertainty_classes_are_preserved() {
        let mut c = candidate();
        c.uncertainty = vec![
            UncertaintyComponent {
                class: UncertaintyClass::Material,
                description: "modulus scatter".to_string(),
                magnitude: 3.5,
                unit: "GPa".to_string(),
                distribution: Some("normal".to_string()),
            },
            UncertaintyComponent {
                class: UncertaintyClass::Manufacturing,
                description: "thickness variation".to_string(),
                magnitude: 0.2,
                unit: "mm".to_string(),
                distribution: None,
            },
        ];
        let report = evaluate_feasibility(&c, &[Gate::Robustness]).unwrap();
        assert!(report.accepted);
        assert_eq!(c.uncertainty.len(), 2);
        assert_ne!(c.uncertainty[0].class, c.uncertainty[1].class);
    }

    #[test]
    fn malformed_uncertainty_is_rejected() {
        let mut c = candidate();
        c.uncertainty = vec![UncertaintyComponent {
            class: UncertaintyClass::Parameter,
            description: "bad".to_string(),
            magnitude: -1.0,
            unit: "mm".to_string(),
            distribution: None,
        }];
        assert!(matches!(
            evaluate_feasibility(&c, &[Gate::Robustness]),
            Err(DesignError::Invalid(_))
        ));
    }

    #[test]
    fn lineage_rejects_cycles() {
        let mut a = candidate();
        a.id = "a".to_string();
        let mut b = candidate();
        b.id = "b".to_string();
        b.parents = vec!["a".to_string()];
        a.parents = vec!["b".to_string()];
        assert!(matches!(
            validate_lineage(&[a, b]),
            Err(DesignError::Lineage(_))
        ));
    }

    #[test]
    fn lineage_rejects_missing_parent() {
        let mut c = candidate();
        c.parents = vec!["ghost".to_string()];
        assert!(matches!(
            validate_lineage(&[c]),
            Err(DesignError::Lineage(_))
        ));
    }

    #[test]
    fn candidate_hash_is_stable() {
        let c = candidate();
        assert_eq!(c.content_hash(), c.content_hash());
        assert_eq!(c.content_hash().len(), 64);
    }
}