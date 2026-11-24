// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! Deterministic multi-fidelity selection.
//!
//! The planner does not decide what is physically true. It makes the
//! fidelity choice explicit and reproducible: given a declared error
//! target, a compute budget, and a set of available fidelity levels with
//! declared cost and expected error, it selects the cheapest level that
//! meets the target, or explains why it must escalate.
//!
//! The declared errors are claims, not measurements. A level that claims a
//! tighter error must carry validation evidence in its model card; the
//! planner records that requirement rather than silently trusting it.

#![deny(unsafe_code)]
#![warn(missing_docs)]

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Physics domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Domain {
    /// Structural mechanics.
    Structures,
    /// Fluid mechanics.
    Fluids,
    /// Heat transfer.
    Thermal,
    /// Coupled multiphysics.
    Multiphysics,
}

/// One available fidelity level.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FidelityLevel {
    /// Stable name, e.g. `beam_fem`.
    pub name: String,
    /// Estimated compute cost in core-hours.
    pub cost_core_hours: f64,
    /// Declared relative error (0, 1].
    pub expected_error: f64,
    /// Whether a model card with validation evidence is required.
    #[serde(default)]
    pub requires_validation: bool,
}

/// A fidelity request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FidelityRequest {
    /// Domain.
    pub domain: Domain,
    /// Maximum acceptable relative error.
    pub target_error: f64,
    /// Compute budget in core-hours.
    pub budget_core_hours: f64,
    /// Available levels.
    pub available: Vec<FidelityLevel>,
    /// Previously used level, if this is a re-run.
    #[serde(default)]
    pub previous: Option<String>,
}

/// The planning result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FidelityPlan {
    /// Selected level, or the best escalation target if none meets target.
    pub selected: Option<String>,
    /// Ordered escalation path from cheapest to most accurate.
    pub escalation_path: Vec<String>,
    /// Whether the selected level meets the error target.
    pub meets_target: bool,
    /// Whether the selected level exceeds the compute budget.
    pub budget_exceeded: bool,
    /// Estimated cost of the selected level.
    pub estimated_cost_core_hours: f64,
    /// Declared error of the selected level.
    pub expected_error: f64,
    /// Whether validation evidence is required for the selected level.
    pub validation_required: bool,
    /// Plain-language explanation.
    pub reasons: Vec<String>,
}

/// Planner errors.
#[derive(Debug, Error, PartialEq)]
pub enum FidelityError {
    /// The request is malformed.
    #[error("invalid request: {0}")]
    Invalid(String),
}

/// Plan a fidelity selection.
pub fn plan(request: &FidelityRequest) -> Result<FidelityPlan, FidelityError> {
    if request.available.is_empty() {
        return Err(FidelityError::Invalid(
            "no fidelity levels are available".to_string(),
        ));
    }
    if !(0.0 < request.target_error && request.target_error <= 1.0) {
        return Err(FidelityError::Invalid(
            "target_error must be in (0, 1]".to_string(),
        ));
    }
    if request.budget_core_hours <= 0.0 {
        return Err(FidelityError::Invalid(
            "budget_core_hours must be positive".to_string(),
        ));
    }
    let mut names = std::collections::BTreeSet::new();
    for level in &request.available {
        if level.name.trim().is_empty() {
            return Err(FidelityError::Invalid("level name is empty".to_string()));
        }
        if !names.insert(level.name.as_str()) {
            return Err(FidelityError::Invalid(format!(
                "duplicate fidelity level {:?}",
                level.name
            )));
        }
        if !(0.0 < level.expected_error && level.expected_error <= 1.0) {
            return Err(FidelityError::Invalid(format!(
                "level {:?} expected_error must be in (0, 1]",
                level.name
            )));
        }
        if level.cost_core_hours < 0.0 {
            return Err(FidelityError::Invalid(format!(
                "level {:?} cost must be non-negative",
                level.name
            )));
        }
    }

    let mut ordered = request.available.clone();
    ordered.sort_by(|a, b| {
        a.cost_core_hours
            .partial_cmp(&b.cost_core_hours)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.name.cmp(&b.name))
    });
    let escalation_path = ordered.iter().map(|l| l.name.clone()).collect();

    let mut affordable: Vec<&FidelityLevel> = ordered
        .iter()
        .filter(|l| l.expected_error <= request.target_error && l.cost_core_hours <= request.budget_core_hours)
        .collect();
    affordable.sort_by(|a, b| {
        a.cost_core_hours
            .partial_cmp(&b.cost_core_hours)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.expected_error.partial_cmp(&b.expected_error).unwrap_or(std::cmp::Ordering::Equal))
    });

    let mut reasons = Vec::new();
    if let Some(level) = affordable.first() {
        reasons.push(format!(
            "{:?}: {} is the cheapest level whose declared error {:.3} meets the target {:.3} within the {:.1} core-hour budget",
            request.domain, level.name, level.expected_error, request.target_error, request.budget_core_hours
        ));
        if let Some(previous) = &request.previous {
            if previous != &level.name {
                reasons.push(format!(
                    "fidelity changed from {previous:?} to {:?}; the change must be recorded in the run provenance",
                    level.name
                ));
            }
        }
        if level.requires_validation {
            reasons.push(format!(
                "{} requires a current model card with independent validation",
                level.name
            ));
        }
        return Ok(FidelityPlan {
            selected: Some(level.name.clone()),
            escalation_path,
            meets_target: true,
            budget_exceeded: false,
            estimated_cost_core_hours: level.cost_core_hours,
            expected_error: level.expected_error,
            validation_required: level.requires_validation,
            reasons,
        });
    }

    // No affordable level meets the target. Name the most accurate level
    // as the escalation target and say exactly which constraint binds.
    let best = ordered
        .iter()
        .min_by(|a, b| {
            a.expected_error
                .partial_cmp(&b.expected_error)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| {
                    a.cost_core_hours
                        .partial_cmp(&b.cost_core_hours)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
        })
        .expect("non-empty checked above");
    let budget_exceeded = best.cost_core_hours > request.budget_core_hours;
    let meets_target = best.expected_error <= request.target_error;
    if !meets_target {
        reasons.push(format!(
            "no available level meets the declared error target {:.3}; the best level {:?} declares {:.3}",
            request.target_error, best.name, best.expected_error
        ));
    }
    if budget_exceeded {
        reasons.push(format!(
            "the most accurate level {:?} costs {:.1} core-hours, above the {:.1} core-hour budget",
            best.name, best.cost_core_hours, request.budget_core_hours
        ));
    }
    reasons.push(
        "escalation must be recorded with the run, including the reason and the approver"
            .to_string(),
    );
    Ok(FidelityPlan {
        selected: Some(best.name.clone()),
        escalation_path,
        meets_target,
        budget_exceeded,
        estimated_cost_core_hours: best.cost_core_hours,
        expected_error: best.expected_error,
        validation_required: best.requires_validation,
        reasons,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn level(name: &str, cost: f64, error: f64) -> FidelityLevel {
        FidelityLevel {
            name: name.to_string(),
            cost_core_hours: cost,
            expected_error: error,
            requires_validation: false,
        }
    }

    fn request() -> FidelityRequest {
        FidelityRequest {
            domain: Domain::Structures,
            target_error: 0.05,
            budget_core_hours: 100.0,
            available: vec![
                level("closed_form", 1.0, 0.20),
                level("beam_fem", 10.0, 0.04),
                level("shell_fem", 60.0, 0.01),
            ],
            previous: None,
        }
    }

    #[test]
    fn selects_cheapest_level_meeting_target() {
        let plan = plan(&request()).unwrap();
        assert_eq!(plan.selected.as_deref(), Some("beam_fem"));
        assert!(plan.meets_target);
        assert!(!plan.budget_exceeded);
        assert_eq!(plan.escalation_path, vec!["closed_form", "beam_fem", "shell_fem"]);
    }

    #[test]
    fn escalates_when_target_is_unreachable() {
        let mut request = request();
        request.target_error = 0.001;
        let plan = plan(&request).unwrap();
        assert_eq!(plan.selected.as_deref(), Some("shell_fem"));
        assert!(!plan.meets_target);
        assert!(plan.reasons.iter().any(|r| r.contains("no available level")));
    }

    #[test]
    fn reports_budget_conflict() {
        let mut request = request();
        request.target_error = 0.001;
        request.budget_core_hours = 20.0;
        let plan = plan(&request).unwrap();
        assert_eq!(plan.selected.as_deref(), Some("shell_fem"));
        assert!(plan.budget_exceeded);
        assert!(plan.reasons.iter().any(|r| r.contains("above the")));
    }

    #[test]
    fn rejects_invalid_target() {
        let mut request = request();
        request.target_error = 0.0;
        assert!(matches!(plan(&request), Err(FidelityError::Invalid(_))));
    }

    #[test]
    fn rejects_duplicate_levels() {
        let mut request = request();
        request.available.push(level("beam_fem", 10.0, 0.04));
        assert!(matches!(plan(&request), Err(FidelityError::Invalid(_))));
    }

    #[test]
    fn records_fidelity_change() {
        let mut request = request();
        request.previous = Some("closed_form".to_string());
        let plan = plan(&request).unwrap();
        assert!(plan.reasons.iter().any(|r| r.contains("fidelity changed")));
    }
}
