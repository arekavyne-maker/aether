// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! Deterministic unit and frame checks for solver submissions.
//!
//! Adapters pass quantities as data. This module validates those quantities
//! against a declared signature before a solver is invoked, and validates
//! points between declared coordinate frames. A mismatch is an error, never
//! a silent coercion.

#![deny(unsafe_code)]
#![warn(missing_docs)]

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{Dimension, FrameGraph, Quantity, QuantityError, Unit};

/// One named quantity supplied to a solver.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolverQuantity {
    /// Parameter name, e.g. `tip_load`.
    pub name: String,
    /// Value in the declared unit.
    pub value: f64,
    /// Unit symbol.
    pub unit: String,
}

/// One named quantity the solver expects.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExpectedQuantity {
    /// Parameter name.
    pub name: String,
    /// Required unit symbol.
    pub unit: String,
}

/// What the submission must contain.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolverContract {
    /// Required quantities.
    pub required: Vec<ExpectedQuantity>,
    /// Declared coordinate frames, if spatial inputs are present.
    #[serde(default)]
    pub frames: Vec<String>,
}

/// A point expressed in a named frame.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FramePoint {
    /// Frame id.
    pub frame: String,
    /// Coordinates.
    pub point: [f64; 3],
}

/// Solver-submission validation errors.
#[derive(Debug, Error, PartialEq)]
pub enum SubmissionError {
    /// A required quantity is missing.
    #[error("missing required quantity {0}")]
    Missing(String),
    /// A quantity has the wrong unit or dimension.
    #[error("quantity {name}: expected unit {expected}, found {found} ({detail})")]
    UnitMismatch {
        /// Quantity name.
        name: String,
        /// Expected unit.
        expected: String,
        /// Found unit.
        found: String,
        /// Detail from the unit check.
        detail: String,
    },
    /// A frame is missing or a point cannot be transformed.
    #[error("frame error: {0}")]
    Frame(String),
}

/// Result of validating a submission.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubmissionReport {
    /// Whether the submission is acceptable.
    pub valid: bool,
    /// Quantities checked.
    pub quantities_checked: usize,
    /// Frame points checked.
    pub points_checked: usize,
    /// Problems found.
    pub issues: Vec<String>,
}

/// Validate quantities and frame points against a contract.
pub fn validate_submission(
    contract: &SolverContract,
    quantities: &[SolverQuantity],
    frames: Option<&FrameGraph>,
    points: &[FramePoint],
) -> Result<SubmissionReport, SubmissionError> {
    let mut issues = Vec::new();
    let mut checked = 0usize;

    for expected in &contract.required {
        let supplied = quantities.iter().find(|q| q.name == expected.name);
        let Some(supplied) = supplied else {
            issues.push(format!("missing required quantity {}", expected.name));
            continue;
        };
        checked += 1;
        let value = Quantity::parse(supplied.value, &supplied.unit).map_err(|e| {
            SubmissionError::UnitMismatch {
                name: supplied.name.clone(),
                expected: expected.unit.clone(),
                found: supplied.unit.clone(),
                detail: e.to_string(),
            }
        })?;
        let target = Unit::parse(&expected.unit).map_err(|e| SubmissionError::UnitMismatch {
            name: supplied.name.clone(),
            expected: expected.unit.clone(),
            found: supplied.unit.clone(),
            detail: e.to_string(),
        })?;
        if let Err(e) = value.to(&target) {
            issues.push(format!(
                "quantity {}: expected {}, found {} ({})",
                supplied.name, expected.unit, supplied.unit, e
            ));
        }
    }

    let mut points_checked = 0usize;
    if let Some(graph) = frames {
        for point in points {
            if !contract.frames.iter().any(|f| f == &point.frame)
                && !contract.frames.is_empty()
            {
                issues.push(format!("frame {} is not declared by the contract", point.frame));
                continue;
            }
            let root = graph
                .point_to_root(&point.frame, point.point)
                .map_err(|e| SubmissionError::Frame(e.to_string()))?;
            // Round-trip the point to prove the transform is invertible.
            let round_trip = graph
                .point_from_root(&point.frame, root)
                .map_err(|e| SubmissionError::Frame(e.to_string()))?;
            let max_error = point
                .point
                .iter()
                .zip(round_trip.iter())
                .map(|(a, b)| (a - b).abs())
                .fold(0.0_f64, f64::max);
            if max_error > 1e-9 {
                issues.push(format!(
                    "frame {} transform is not numerically invertible (error {max_error})",
                    point.frame
                ));
            }
            points_checked += 1;
        }
    }

    Ok(SubmissionReport {
        valid: issues.is_empty(),
        quantities_checked: checked,
        points_checked,
        issues,
    })
}

/// A convenience check for a single quantity pair.
pub fn check_quantity(name: &str, value: f64, unit: &str, expected: &str) -> Result<(), QuantityError> {
    let _ = name;
    let quantity = Quantity::parse(value, unit)?;
    let target = Unit::parse(expected)?;
    quantity.to(&target).map(|_| ())
}

/// The dimension of a unit symbol.
pub fn dimension_of(unit: &str) -> Result<Dimension, QuantityError> {
    Ok(Unit::parse(unit)?.dimension)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Frame;

    fn contract() -> SolverContract {
        SolverContract {
            required: vec![
                ExpectedQuantity {
                    name: "tip_load".to_string(),
                    unit: "N".to_string(),
                },
                ExpectedQuantity {
                    name: "span".to_string(),
                    unit: "m".to_string(),
                },
            ],
            frames: vec!["wing".to_string()],
        }
    }

    fn quantities() -> Vec<SolverQuantity> {
        vec![
            SolverQuantity {
                name: "tip_load".to_string(),
                value: 40.0,
                unit: "kN".to_string(),
            },
            SolverQuantity {
                name: "span".to_string(),
                value: 8000.0,
                unit: "mm".to_string(),
            },
        ]
    }

    #[test]
    fn equivalent_units_pass() {
        let report =
            validate_submission(&contract(), &quantities(), None, &[]).unwrap();
        assert!(report.valid, "{:?}", report.issues);
        assert_eq!(report.quantities_checked, 2);
    }

    #[test]
    fn dimension_mismatch_is_rejected() {
        let mut q = quantities();
        q[0].unit = "m".to_string();
        let report = validate_submission(&contract(), &q, None, &[]).unwrap();
        assert!(!report.valid);
        assert!(report.issues.iter().any(|i| i.contains("tip_load")));
    }

    #[test]
    fn missing_quantity_is_reported() {
        let q = vec![quantities().remove(0)];
        let report = validate_submission(&contract(), &q, None, &[]).unwrap();
        assert!(!report.valid);
        assert!(report.issues.iter().any(|i| i.contains("missing")));
    }

    #[test]
    fn frame_round_trip_is_checked() {
        let mut graph = FrameGraph::new();
        graph.add(Frame::identity("wing")).unwrap();
        let points = vec![FramePoint {
            frame: "wing".to_string(),
            point: [1.0, 2.0, 3.0],
        }];
        let report = validate_submission(&contract(), &quantities(), Some(&graph), &points).unwrap();
        assert!(report.valid, "{:?}", report.issues);
        assert_eq!(report.points_checked, 1);
    }

    #[test]
    fn undeclared_frame_is_rejected() {
        let mut graph = FrameGraph::new();
        graph.add(Frame::identity("wing")).unwrap();
        let points = vec![FramePoint {
            frame: "tail".to_string(),
            point: [0.0, 0.0, 0.0],
        }];
        let report = validate_submission(&contract(), &quantities(), Some(&graph), &points).unwrap();
        assert!(!report.valid);
        assert!(report.issues.iter().any(|i| i.contains("tail")));
    }
}