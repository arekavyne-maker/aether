// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! Verification and validation cases, coverage, and review decisions.
//!
//! A requirement is not "done" because someone wrote a test. It is done when
//! a *verification case* declares a method, that case is linked to the
//! requirement, and the case is backed by *current evidence* from an
//! authorized source. This crate makes that chain executable and auditable.
//!
//! Four object families, matching the Aether 3.0 ASM:
//!
//! - [`Requirement`] — a stakeholder need or derived requirement.
//! - [`VerificationCase`] — how a requirement will be verified (analysis,
//!   test, inspection, demonstration, similarity).
//! - [`ValidationCase`] — how the model will be validated against reality.
//! - [`ComplianceFinding`] — a conformance statement against a standard.
//! - [`ReviewDecision`] — an authorized human decision.
//!
//! [`coverage`] then computes the honest coverage report: requirements with
//! no verification case are gaps, and cases with no current evidence are
//! gaps too. There is no path by which a requirement is reported verified
//! without evidence.

#![deny(unsafe_code)]
#![warn(missing_docs)]

use std::collections::{BTreeMap, BTreeSet};

use aether_evidence::{EvidenceItem, EvidenceStatus};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

/// The verification method declared by a case.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationMethod {
    /// Closed-form or hand analysis.
    Analysis,
    /// Physical or numerical test.
    Test,
    /// Measurement against a drawing.
    Inspection,
    /// Demonstration in the intended environment.
    Demonstration,
    /// Similarity to an already-qualified article.
    Similarity,
}

impl VerificationMethod {
    /// Wire label.
    pub fn as_str(self) -> &'static str {
        match self {
            VerificationMethod::Analysis => "analysis",
            VerificationMethod::Test => "test",
            VerificationMethod::Inspection => "inspection",
            VerificationMethod::Demonstration => "demonstration",
            VerificationMethod::Similarity => "similarity",
        }
    }

    /// Parse a wire label.
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "analysis" => VerificationMethod::Analysis,
            "test" => VerificationMethod::Test,
            "inspection" => VerificationMethod::Inspection,
            "demonstration" => VerificationMethod::Demonstration,
            "similarity" => VerificationMethod::Similarity,
            _ => return None,
        })
    }
}

/// Criticality of a requirement, driving how strong its evidence must be.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Criticality {
    /// No safety effect.
    NoEffect,
    /// Minor.
    Minor,
    /// Major.
    Major,
    /// Hazardous.
    Hazardous,
    /// Catastrophic.
    Catastrophic,
}

/// A requirement.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Requirement {
    /// Stable id.
    pub id: String,
    /// Text of the requirement.
    pub text: String,
    /// Declared verification method.
    pub method: VerificationMethod,
    /// Criticality.
    pub criticality: Criticality,
    /// Parent requirement, if derived.
    #[serde(default)]
    pub parent: Option<String>,
    /// Owner.
    #[serde(default)]
    pub owner: Option<String>,
}

impl Requirement {
    /// Canonical hash of the requirement definition.
    pub fn content_hash(&self) -> String {
        let canonical = serde_json::json!({
            "id": self.id,
            "text": self.text,
            "method": self.method.as_str(),
            "criticality": format!("{:?}", self.criticality).to_lowercase(),
            "parent": self.parent,
            "owner": self.owner,
        });
        hash_json(&canonical)
    }
}

/// A verification case: how a requirement is shown to be met.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VerificationCase {
    /// Case id.
    pub id: String,
    /// Requirement this case verifies.
    pub requirement_id: String,
    /// Method.
    pub method: VerificationMethod,
    /// What will be measured or demonstrated.
    pub acceptance_criterion: String,
    /// Evidence ids that satisfy this case.
    #[serde(default)]
    pub evidence_ids: Vec<String>,
    /// Revision of the requirement this case verifies.
    #[serde(default)]
    pub requirement_revision: u32,
}

impl VerificationCase {
    /// Canonical hash.
    pub fn content_hash(&self) -> String {
        let canonical = serde_json::json!({
            "id": self.id,
            "requirement_id": self.requirement_id,
            "method": self.method.as_str(),
            "acceptance_criterion": self.acceptance_criterion,
            "evidence_ids": self.evidence_ids,
            "requirement_revision": self.requirement_revision,
        });
        hash_json(&canonical)
    }
}

/// A validation case: how a model is shown to represent reality.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValidationCase {
    /// Case id.
    pub id: String,
    /// Subject model or analysis id.
    pub subject: String,
    /// What real-world quantity is compared.
    pub quantity_of_interest: String,
    /// Domain over which validity is claimed.
    pub domain: BTreeMap<String, String>,
    /// Evidence ids.
    #[serde(default)]
    pub evidence_ids: Vec<String>,
}

/// A compliance finding against a named standard or regulation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComplianceFinding {
    /// Finding id.
    pub id: String,
    /// Requirement id this finding addresses.
    pub requirement_id: String,
    /// Standard or regulation cited.
    pub standard: String,
    /// Clause within the standard.
    pub clause: String,
    /// Compliance state.
    pub state: ComplianceState,
    /// Evidence ids supporting the state.
    #[serde(default)]
    pub evidence_ids: Vec<String>,
}

/// Compliance state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComplianceState {
    /// Compliant, with evidence.
    Compliant,
    /// Not compliant.
    NonCompliant,
    /// Not yet assessed.
    NotAssessed,
    /// Compliance is not applicable.
    NotApplicable,
}

/// An authorized human decision.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReviewDecision {
    /// Decision id.
    pub id: String,
    /// Subject of the decision (requirement, case, or finding).
    pub subject_id: String,
    /// Outcome.
    pub outcome: ReviewOutcome,
    /// Approver identity.
    pub approved_by: String,
    /// Role the approver held.
    pub approved_by_role: String,
    /// ISO-8601 time.
    pub decided_at: String,
    /// Rationale.
    pub rationale: String,
}

/// Outcome of a review decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewOutcome {
    /// Accepted.
    Approved,
    /// Rejected.
    Rejected,
    /// Deferred pending more information.
    Deferred,
}

/// V&V errors.
#[derive(Debug, Error, PartialEq)]
pub enum VvError {
    /// A referenced object does not exist.
    #[error("unresolved reference: {0}")]
    Unresolved(String),
    /// A required field is empty or malformed.
    #[error("invalid: {0}")]
    Invalid(String),
    /// An unauthorized approver attempted a decision.
    #[error("unauthorized approver: {0}")]
    Unauthorized(String),
    /// Duplicate identifier.
    #[error("duplicate id: {0}")]
    Duplicate(String),
}

/// One gap in the verification chain.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CoverageGap {
    /// Requirement with the gap.
    pub requirement_id: String,
    /// Requirement text.
    pub requirement_text: String,
    /// Criticality.
    pub criticality: String,
    /// Machine-readable reason code.
    pub reason_code: String,
    /// Human explanation.
    pub reason: String,
}

/// The honest coverage report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CoverageReport {
    /// Requirements considered.
    pub requirements_total: usize,
    /// Requirements fully covered by a case with current evidence.
    pub requirements_verified: usize,
    /// Requirements with a case but no current evidence.
    pub requirements_unverified: usize,
    /// Requirements with no case at all.
    pub requirements_uncased: usize,
    /// Gaps, most severe first.
    pub gaps: Vec<CoverageGap>,
    /// Explicit disclaimer carried with every report.
    pub claim: String,
}

/// The V&V model: requirements, cases, evidence, and decisions.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct VvModel {
    /// Requirements by id.
    pub requirements: BTreeMap<String, Requirement>,
    /// Verification cases by id.
    pub verification_cases: BTreeMap<String, VerificationCase>,
    /// Validation cases by id.
    pub validation_cases: BTreeMap<String, ValidationCase>,
    /// Compliance findings by id.
    pub compliance_findings: BTreeMap<String, ComplianceFinding>,
    /// Review decisions by id.
    pub review_decisions: BTreeMap<String, ReviewDecision>,
    /// Evidence items by id.
    pub evidence: BTreeMap<String, EvidenceItem>,
}

impl VvModel {
    /// Empty model.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a requirement, rejecting duplicates.
    pub fn add_requirement(&mut self, requirement: Requirement) -> Result<(), VvError> {
        if requirement.id.trim().is_empty() {
            return Err(VvError::Invalid("requirement id is empty".into()));
        }
        if requirement.text.trim().is_empty() {
            return Err(VvError::Invalid(format!(
                "requirement {} has empty text",
                requirement.id
            )));
        }
        if self.requirements.contains_key(&requirement.id) {
            return Err(VvError::Duplicate(requirement.id));
        }
        self.requirements.insert(requirement.id.clone(), requirement);
        Ok(())
    }

    /// Add a verification case, rejecting dangling requirement references.
    pub fn add_verification_case(
        &mut self,
        case: VerificationCase,
    ) -> Result<(), VvError> {
        if !self.requirements.contains_key(&case.requirement_id) {
            return Err(VvError::Unresolved(format!(
                "requirement {}",
                case.requirement_id
            )));
        }
        if self.verification_cases.contains_key(&case.id) {
            return Err(VvError::Duplicate(case.id));
        }
        self.verification_cases.insert(case.id.clone(), case);
        Ok(())
    }

    /// Add evidence.
    pub fn add_evidence(&mut self, item: EvidenceItem) -> Result<(), VvError> {
        if self.evidence.contains_key(&item.id) {
            return Err(VvError::Duplicate(item.id));
        }
        self.evidence.insert(item.id.clone(), item);
        Ok(())
    }

    /// Record a review decision. Approval of a safety-significant subject
    /// requires an authorized role.
    pub fn record_decision(
        &mut self,
        mut decision: ReviewDecision,
        authorized_roles: &[&str],
    ) -> Result<(), VvError> {
        if decision.approved_by.trim().is_empty() {
            return Err(VvError::Invalid("approver identity is empty".into()));
        }
        if decision.outcome == ReviewOutcome::Approved
            && !authorized_roles.contains(&decision.approved_by_role.as_str())
        {
            return Err(VvError::Unauthorized(decision.approved_by_role));
        }
        if self.review_decisions.contains_key(&decision.id) {
            return Err(VvError::Duplicate(decision.id));
        }
        decision.id = decision.id.trim().to_string();
        self.review_decisions
            .insert(decision.id.clone(), decision);
        Ok(())
    }

    /// Evidence items that satisfy a case and are current.
    pub fn current_evidence_for(&self, case: &VerificationCase) -> Vec<&EvidenceItem> {
        case.evidence_ids
            .iter()
            .filter_map(|id| self.evidence.get(id))
            .filter(|item| item.status == EvidenceStatus::Current)
            .collect()
    }

    /// Compute honest verification coverage.
    ///
    /// A requirement counts as verified only if it has a verification case
    /// whose method matches the requirement's declared method, and that
    /// case has at least one current evidence item bound to the requirement.
    pub fn coverage(&self) -> CoverageReport {
        let mut gaps = Vec::new();
        let mut verified = 0usize;
        let mut unverified = 0usize;
        let mut uncased = 0usize;

        for requirement in self.requirements.values() {
            let cases: Vec<&VerificationCase> = self
                .verification_cases
                .values()
                .filter(|c| c.requirement_id == requirement.id)
                .collect();

            if cases.is_empty() {
                uncased += 1;
                gaps.push(CoverageGap {
                    requirement_id: requirement.id.clone(),
                    requirement_text: requirement.text.clone(),
                    criticality: criticality_label(requirement.criticality),
                    reason_code: "NO_VERIFICATION_CASE".to_string(),
                    reason: "no verification case is linked to this requirement".to_string(),
                });
                continue;
            }

            let method_matched = cases.iter().any(|c| c.method == requirement.method);
            if !method_matched {
                gaps.push(CoverageGap {
                    requirement_id: requirement.id.clone(),
                    requirement_text: requirement.text.clone(),
                    criticality: criticality_label(requirement.criticality),
                    reason_code: "METHOD_MISMATCH".to_string(),
                    reason: format!(
                        "requirement declares {:?} but no case uses that method",
                        requirement.method
                    ),
                });
                continue;
            }

            let current: Vec<&EvidenceItem> = cases
                .iter()
                .flat_map(|c| self.current_evidence_for(c))
                .filter(|item| item.subject == requirement.id)
                .collect();

            if current.is_empty() {
                unverified += 1;
                gaps.push(CoverageGap {
                    requirement_id: requirement.id.clone(),
                    requirement_text: requirement.text.clone(),
                    criticality: criticality_label(requirement.criticality),
                    reason_code: "NO_CURRENT_EVIDENCE".to_string(),
                    reason: "a verification case exists but no current evidence is bound to it"
                        .to_string(),
                });
                continue;
            }

            verified += 1;
        }

        // Most severe criticality first, then by id for determinism.
        gaps.sort_by(|a, b| {
            criticality_rank(&a.criticality)
                .cmp(&criticality_rank(&b.criticality))
                .then_with(|| a.requirement_id.cmp(&b.requirement_id))
        });

        CoverageReport {
            requirements_total: self.requirements.len(),
            requirements_verified: verified,
            requirements_unverified: unverified,
            requirements_uncased: uncased,
            gaps,
            claim: "coverage only; this is not certification and does not imply airworthiness"
                .to_string(),
        }
    }

    /// Compliance findings that are not yet compliant.
    pub fn open_findings(&self) -> Vec<&ComplianceFinding> {
        self.compliance_findings
            .values()
            .filter(|f| {
                matches!(
                    f.state,
                    ComplianceState::NonCompliant | ComplianceState::NotAssessed
                )
            })
            .collect()
    }
}

fn criticality_label(c: Criticality) -> String {
    match c {
        Criticality::NoEffect => "no_effect".to_string(),
        Criticality::Minor => "minor".to_string(),
        Criticality::Major => "major".to_string(),
        Criticality::Hazardous => "hazardous".to_string(),
        Criticality::Catastrophic => "catastrophic".to_string(),
    }
}

fn criticality_rank(label: &str) -> u8 {
    match label {
        "catastrophic" => 0,
        "hazardous" => 1,
        "major" => 2,
        "minor" => 3,
        _ => 4,
    }
}

/// Hash a JSON value into a 64-char hex digest.
pub fn hash_json(value: &serde_json::Value) -> String {
    let bytes = serde_json::to_vec(value).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

/// A set of roles authorized to approve safety-significant decisions.
pub const DEFAULT_APPROVAL_ROLES: &[&str] = &[
    "chief-engineer",
    "quality-auditor",
    "certification-authority",
];

/// Collect the ids referenced by a model, for lineage export.
pub fn referenced_ids(model: &VvModel) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    for c in model.verification_cases.values() {
        ids.insert(c.requirement_id.clone());
        ids.extend(c.evidence_ids.iter().cloned());
    }
    for v in model.validation_cases.values() {
        ids.extend(v.evidence_ids.iter().cloned());
    }
    for f in model.compliance_findings.values() {
        ids.extend(f.evidence_ids.iter().cloned());
    }
    ids
}

#[cfg(test)]
mod tests {
    use super::*;
    use aether_evidence::{EvidenceClass, EvidenceStatus};

    fn requirement(id: &str, method: VerificationMethod) -> Requirement {
        Requirement {
            id: id.to_string(),
            text: format!("requirement {id}"),
            method,
            criticality: Criticality::Major,
            parent: None,
            owner: Some("systems".to_string()),
        }
    }

    fn case(id: &str, req: &str, method: VerificationMethod, evidence: &[&str]) -> VerificationCase {
        VerificationCase {
            id: id.to_string(),
            requirement_id: req.to_string(),
            method,
            acceptance_criterion: "measured value within band".to_string(),
            evidence_ids: evidence.iter().map(|s| s.to_string()).collect(),
            requirement_revision: 1,
        }
    }

    fn evidence(id: &str, subject: &str, status: EvidenceStatus) -> EvidenceItem {
        EvidenceItem {
            id: id.to_string(),
            class: EvidenceClass::Verification,
            subject: subject.to_string(),
            subject_revision: 1,
            artifact_hash: "a".repeat(64),
            status,
            issued_by: "analyst".to_string(),
            issued_at: "2026-11-24T00:00:00Z".to_string(),
            credibility: None,
        }
    }

    #[test]
    fn requirement_with_current_evidence_is_verified() {
        let mut model = VvModel::new();
        model.add_requirement(requirement("R1", VerificationMethod::Analysis)).unwrap();
        model.add_verification_case(case("V1", "R1", VerificationMethod::Analysis, &["E1"])).unwrap();
        model.add_evidence(evidence("E1", "R1", EvidenceStatus::Current)).unwrap();

        let report = model.coverage();
        assert_eq!(report.requirements_total, 1);
        assert_eq!(report.requirements_verified, 1);
        assert!(report.gaps.is_empty(), "{:?}", report.gaps);
    }

    #[test]
    fn requirement_without_case_is_a_gap() {
        let mut model = VvModel::new();
        model.add_requirement(requirement("R1", VerificationMethod::Test)).unwrap();

        let report = model.coverage();
        assert_eq!(report.requirements_uncased, 1);
        assert_eq!(report.gaps[0].reason_code, "NO_VERIFICATION_CASE");
    }

    #[test]
    fn stale_evidence_does_not_verify() {
        let mut model = VvModel::new();
        model.add_requirement(requirement("R1", VerificationMethod::Analysis)).unwrap();
        model.add_verification_case(case("V1", "R1", VerificationMethod::Analysis, &["E1"])).unwrap();
        model.add_evidence(evidence("E1", "R1", EvidenceStatus::Stale)).unwrap();

        let report = model.coverage();
        assert_eq!(report.requirements_verified, 0);
        assert_eq!(report.gaps[0].reason_code, "NO_CURRENT_EVIDENCE");
    }

    #[test]
    fn evidence_for_another_requirement_does_not_count() {
        let mut model = VvModel::new();
        model.add_requirement(requirement("R1", VerificationMethod::Analysis)).unwrap();
        model.add_requirement(requirement("R2", VerificationMethod::Analysis)).unwrap();
        model.add_verification_case(case("V1", "R1", VerificationMethod::Analysis, &["E2"])).unwrap();
        model.add_evidence(evidence("E2", "R2", EvidenceStatus::Current)).unwrap();

        let report = model.coverage();
        assert_eq!(report.requirements_verified, 0);
    }

    #[test]
    fn method_mismatch_is_a_gap() {
        let mut model = VvModel::new();
        model.add_requirement(requirement("R1", VerificationMethod::Test)).unwrap();
        model.add_verification_case(case("V1", "R1", VerificationMethod::Analysis, &["E1"])).unwrap();
        model.add_evidence(evidence("E1", "R1", EvidenceStatus::Current)).unwrap();

        let report = model.coverage();
        assert_eq!(report.requirements_verified, 0);
        assert_eq!(report.gaps[0].reason_code, "METHOD_MISMATCH");
    }

    #[test]
    fn dangling_case_reference_is_rejected() {
        let mut model = VvModel::new();
        let err = model
            .add_verification_case(case("V1", "R-missing", VerificationMethod::Test, &[]))
            .unwrap_err();
        assert!(matches!(err, VvError::Unresolved(_)));
    }

    #[test]
    fn only_authorized_roles_can_approve() {
        let mut model = VvModel::new();
        let decision = ReviewDecision {
            id: "D1".to_string(),
            subject_id: "R1".to_string(),
            outcome: ReviewOutcome::Approved,
            approved_by: "engineer".to_string(),
            approved_by_role: "analyst".to_string(),
            decided_at: "2026-11-24T00:00:00Z".to_string(),
            rationale: "ok".to_string(),
        };
        let err = model
            .record_decision(decision, DEFAULT_APPROVAL_ROLES)
            .unwrap_err();
        assert!(matches!(err, VvError::Unauthorized(_)));
    }

    #[test]
    fn authorized_role_can_approve() {
        let mut model = VvModel::new();
        let decision = ReviewDecision {
            id: "D1".to_string(),
            subject_id: "R1".to_string(),
            outcome: ReviewOutcome::Approved,
            approved_by: "c. engineer".to_string(),
            approved_by_role: "chief-engineer".to_string(),
            decided_at: "2026-11-24T00:00:00Z".to_string(),
            rationale: "ok".to_string(),
        };
        model
            .record_decision(decision, DEFAULT_APPROVAL_ROLES)
            .unwrap();
        assert_eq!(model.review_decisions.len(), 1);
    }

    #[test]
    fn gaps_are_ordered_most_severe_first() {
        let mut model = VvModel::new();
        let mut minor = requirement("R-minor", VerificationMethod::Test);
        minor.criticality = Criticality::Minor;
        let mut catastrophic = requirement("R-cat", VerificationMethod::Test);
        catastrophic.criticality = Criticality::Catastrophic;
        model.add_requirement(catastrophic).unwrap();
        model.add_requirement(minor).unwrap();

        let report = model.coverage();
        assert_eq!(report.gaps[0].requirement_id, "R-cat");
        assert_eq!(report.gaps[1].requirement_id, "R-minor");
    }

    #[test]
    fn report_carries_a_disclaimer() {
        let model = VvModel::new();
        assert!(model.coverage().claim.contains("not certification"));
    }
}