// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! Research-to-production promotion gate (ADR-0038).
//!
//! A research method is production-grade *code* but not production-*approved*
//! until it passes this gate. Promotion requires grouped and external
//! benchmark evidence, uncertainty characterisation, an OOD detector,
//! physical-constraint checks, manufacturing feasibility review and a named
//! approver. A method cannot approve itself. Rejections are retained.

#![deny(unsafe_code)]
#![warn(missing_docs)]

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Lifecycle state of a research method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResearchState {
    /// Sandbox only.
    Research,
    /// Benchmark evidence submitted.
    Submitted,
    /// Benchmarked with grouped validation.
    Benchmarked,
    /// Independently validated on external data.
    IndependentlyValidated,
    /// Approved for decision support.
    Approved,
    /// Approved for bounded automation.
    BoundedAutomation,
    /// Rejected, with reasons retained.
    Rejected,
    /// Rolled back.
    RolledBack,
}

/// Evidence requirements for promotion.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PromotionEvidence {
    /// Benchmark ids with grouped splits.
    pub grouped_benchmarks: Vec<String>,
    /// Benchmark ids with external validation.
    pub external_benchmarks: Vec<String>,
    /// Uncertainty method name.
    pub uncertainty_method: String,
    /// OOD detector name.
    pub ood_detector: String,
    /// Physical-constraint check names.
    pub constraint_checks: Vec<String>,
    /// Manufacturing feasibility reviewer.
    pub manufacturing_reviewer: String,
}

/// A promotion request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PromotionRequest {
    /// Method id.
    pub method_id: String,
    /// Evidence supplied.
    pub evidence: PromotionEvidence,
    /// Approver identity.
    pub approver: String,
    /// Approver role.
    pub approver_role: String,
}

/// A promotion decision.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PromotionDecision {
    /// Whether promotion is allowed.
    pub allowed: bool,
    /// Method id.
    pub method_id: String,
    /// Resulting state if allowed.
    pub state: ResearchState,
    /// Reason codes.
    pub reason_codes: Vec<String>,
    /// Human explanation.
    pub reason: String,
}

/// Research errors.
#[derive(Debug, Error, PartialEq)]
pub enum ResearchError {
    /// A field is empty.
    #[error("invalid: {0}")]
    Invalid(String),
}

/// Evaluate a promotion request. Returns a decision; never mutates state.
pub fn evaluate(request: &PromotionRequest) -> Result<PromotionDecision, ResearchError> {
    let mut reasons = Vec::new();
    if request.method_id.trim().is_empty() {
        return Err(ResearchError::Invalid("method_id is empty".into()));
    }
    if request.approver.trim().is_empty() {
        return Err(ResearchError::Invalid("approver is empty".into()));
    }
    if request.method_id == request.approver {
        reasons.push("SELF_APPROVAL".to_string());
    }
    if !matches!(
        request.approver_role.as_str(),
        "chief-engineer" | "quality-auditor" | "research-board"
    ) {
        reasons.push("UNAUTHORIZED_APPROVER".to_string());
    }
    if request.evidence.grouped_benchmarks.is_empty() {
        reasons.push("NO_GROUPED_BENCHMARK".to_string());
    }
    if request.evidence.external_benchmarks.is_empty() {
        reasons.push("NO_EXTERNAL_BENCHMARK".to_string());
    }
    if request.evidence.uncertainty_method.trim().is_empty() {
        reasons.push("NO_UNCERTAINTY_METHOD".to_string());
    }
    if request.evidence.ood_detector.trim().is_empty() {
        reasons.push("NO_OOD_DETECTOR".to_string());
    }
    if request.evidence.constraint_checks.is_empty() {
        reasons.push("NO_PHYSICAL_CONSTRAINT_CHECK".to_string());
    }
    if request.evidence.manufacturing_reviewer.trim().is_empty() {
        reasons.push("NO_MANUFACTURING_REVIEW".to_string());
    }
    let allowed = reasons.is_empty();
    Ok(PromotionDecision {
        allowed,
        method_id: request.method_id.clone(),
        state: if allowed {
            ResearchState::Approved
        } else {
            ResearchState::Rejected
        },
        reason_codes: reasons.clone(),
        reason: if allowed {
            "evidence complete; approved for decision support".to_string()
        } else {
            format!("promotion refused: {}", reasons.join(", "))
        },
    })
}

/// The set of checks every promoted method must retain.
pub fn required_checks() -> BTreeSet<&'static str> {
    [
        "grouped_benchmark",
        "external_benchmark",
        "uncertainty",
        "ood_detector",
        "physical_constraints",
        "manufacturing_review",
    ]
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> PromotionRequest {
        PromotionRequest {
            method_id: "neural-operator-v1".to_string(),
            evidence: PromotionEvidence {
                grouped_benchmarks: vec!["bench-grouped-1".to_string()],
                external_benchmarks: vec!["bench-external-1".to_string()],
                uncertainty_method: "ensemble".to_string(),
                ood_detector: "domain-check".to_string(),
                constraint_checks: vec!["energy-balance".to_string()],
                manufacturing_reviewer: "manufacturing-engineer".to_string(),
            },
            approver: "c.engineer".to_string(),
            approver_role: "chief-engineer".to_string(),
        }
    }

    #[test]
    fn complete_evidence_approves() {
        let decision = evaluate(&request()).unwrap();
        assert!(decision.allowed);
        assert_eq!(decision.state, ResearchState::Approved);
    }

    #[test]
    fn self_approval_is_refused() {
        let mut r = request();
        r.approver = r.method_id.clone();
        let decision = evaluate(&r).unwrap();
        assert!(!decision.allowed);
        assert!(decision.reason_codes.contains(&"SELF_APPROVAL".to_string()));
    }

    #[test]
    fn missing_external_benchmark_is_refused() {
        let mut r = request();
        r.evidence.external_benchmarks.clear();
        let decision = evaluate(&r).unwrap();
        assert!(decision.reason_codes.contains(&"NO_EXTERNAL_BENCHMARK".to_string()));
    }

    #[test]
    fn unauthorized_role_is_refused() {
        let mut r = request();
        r.approver_role = "analyst".to_string();
        assert!(!evaluate(&r).unwrap().allowed);
    }

    #[test]
    fn rejection_retains_reason_codes() {
        let mut r = request();
        r.evidence.constraint_checks.clear();
        let decision = evaluate(&r).unwrap();
        assert_eq!(decision.state, ResearchState::Rejected);
        assert!(!decision.reason_codes.is_empty());
    }
}