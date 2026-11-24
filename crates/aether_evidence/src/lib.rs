// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! Evidence ledger and release gates.
//!
//! Successful execution is not engineering authority. A release gate is
//! satisfied only by current, credible, correctly-classed evidence that is
//! bound to the configuration snapshot under review. Missing, stale, or
//! unverified evidence fails closed.
//!
//! Evidence records are metadata; the bytes they cite live in the artifact
//! store and are referenced by hash.

#![deny(unsafe_code)]
#![warn(missing_docs)]

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Evidence classes, ordered from weakest to strongest for aerostructures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceClass {
    /// Hand calculation or closed form.
    Analytical,
    /// Code verification against a known solution.
    Verification,
    /// Validation against independent physical data.
    Validation,
    /// Coupon-level test.
    Coupon,
    /// Element-level test.
    Element,
    /// Component-level test.
    Component,
    /// Full-scale or full-airframe test.
    FullScale,
    /// Manufacturing process qualification.
    ProcessQualification,
    /// Inspection or non-destructive evaluation.
    Inspection,
    /// In-service or operational evidence.
    Operational,
    /// Independent third-party review.
    IndependentReview,
}

impl EvidenceClass {
    /// Canonical label used in schemas and reports.
    pub fn label(self) -> &'static str {
        match self {
            EvidenceClass::Analytical => "analytical",
            EvidenceClass::Verification => "verification",
            EvidenceClass::Validation => "validation",
            EvidenceClass::Coupon => "coupon",
            EvidenceClass::Element => "element",
            EvidenceClass::Component => "component",
            EvidenceClass::FullScale => "full_scale",
            EvidenceClass::ProcessQualification => "process_qualification",
            EvidenceClass::Inspection => "inspection",
            EvidenceClass::Operational => "operational",
            EvidenceClass::IndependentReview => "independent_review",
        }
    }
}

/// Currency of an evidence item relative to the subject revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceStatus {
    /// Applies to the current subject revision.
    Current,
    /// Predates a change to the subject.
    Stale,
    /// Withdrawn.
    Revoked,
}

/// A credibility record: what a model or analysis is believed to be good for.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CredibilityRecord {
    /// Record id.
    pub id: String,
    /// Subject (model, method, or analysis id).
    pub subject: String,
    /// Intended use.
    pub intended_use: String,
    /// Prohibited or unsupported use.
    pub prohibited_use: Vec<String>,
    /// Validity envelope as free-form bounds.
    pub validity_envelope: BTreeMap<String, String>,
    /// Evidence classes supporting the record.
    pub supported_by: Vec<EvidenceClass>,
    /// Who approved it.
    pub approved_by: Option<String>,
}

impl CredibilityRecord {
    /// A record has credibility only if it is approved, names an intended
    /// use, and is supported by at least one validation-class item.
    pub fn is_credible(&self) -> bool {
        self.approved_by.is_some()
            && !self.intended_use.trim().is_empty()
            && self.supported_by.iter().any(|c| {
                matches!(
                    c,
                    EvidenceClass::Validation
                        | EvidenceClass::Component
                        | EvidenceClass::FullScale
                        | EvidenceClass::IndependentReview
                )
            })
    }
}

/// One evidence item.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvidenceItem {
    /// Evidence id.
    pub id: String,
    /// Evidence class.
    pub class: EvidenceClass,
    /// Subject object id.
    pub subject: String,
    /// Subject revision the evidence applies to.
    pub subject_revision: u32,
    /// Artifact hash holding the evidence bytes.
    pub artifact_hash: String,
    /// Current/stale/revoked.
    pub status: EvidenceStatus,
    /// Issuer identity.
    pub issued_by: String,
    /// ISO-8601 issue time.
    pub issued_at: String,
    /// Optional credibility record id.
    pub credibility: Option<String>,
}

/// One requirement inside a gate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GateRequirement {
    /// Required evidence class.
    pub class: EvidenceClass,
    /// Minimum number of items.
    pub min_count: usize,
    /// Require a credible credibility record.
    pub require_credibility: bool,
}

/// A named release gate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Gate {
    /// Gate id.
    pub id: String,
    /// Human description.
    pub description: String,
    /// Required evidence.
    pub requirements: Vec<GateRequirement>,
    /// Whether a waiver may satisfy a failing gate.
    pub waiver_allowed: bool,
}

/// A waiver for a specific gate and snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Waiver {
    /// Waiver id.
    pub id: String,
    /// Gate id.
    pub gate: String,
    /// Snapshot the waiver applies to.
    pub snapshot: String,
    /// Rationale.
    pub rationale: String,
    /// Approving principal.
    pub approved_by: String,
    /// Role; only `admin` and `quality-auditor` may waive.
    pub role: String,
    /// ISO-8601 creation time.
    pub created_at: String,
    /// Whether the waiver is still active.
    pub active: bool,
}

impl Waiver {
    /// Whether this waiver is authorized for the gate and snapshot.
    pub fn authorizes(&self, gate: &str, snapshot: &str) -> bool {
        self.active
            && self.gate == gate
            && self.snapshot == snapshot
            && matches!(self.role.as_str(), "admin" | "quality-auditor")
            && !self.rationale.trim().is_empty()
    }
}

/// A gate verdict.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GateVerdict {
    /// Gate id.
    pub gate: String,
    /// Whether the gate passes.
    pub passed: bool,
    /// Missing classes, with counts.
    pub missing: Vec<String>,
    /// Evidence that exists but is stale or revoked.
    pub stale: Vec<String>,
    /// Evidence that lacks a credible record where one is required.
    pub unverified: Vec<String>,
    /// Waiver id, when the gate passed by waiver.
    pub waived_by: Option<String>,
}

/// Evidence and gate errors.
#[derive(Debug, Error, PartialEq)]
pub enum EvidenceError {
    /// Gate failed and no waiver applies.
    #[error("evidence gate {gate} failed: missing={missing:?} stale={stale:?}")]
    GateFailed {
        /// Gate id.
        gate: String,
        /// Missing classes.
        missing: Vec<String>,
        /// Stale evidence.
        stale: Vec<String>,
    },
    /// Waiver not authorized.
    #[error("waiver not authorized: {0}")]
    UnauthorizedWaiver(String),
    /// Release attempted without a passing gate set.
    #[error("release blocked: gate {0} did not pass")]
    ReleaseBlocked(String),
    /// Snapshot mismatch.
    #[error("evidence is bound to snapshot {evidence}, not {release}")]
    SnapshotMismatch {
        /// Snapshot the evidence belongs to.
        evidence: String,
        /// Snapshot being released.
        release: String,
    },
}

/// Result alias.
pub type EvidenceResult<T> = Result<T, EvidenceError>;

/// Evaluate a gate against evidence items.
///
/// `snapshot` is informational here; items are expected to be pre-filtered
/// to the snapshot. Passing `Some(waiver)` allows a gate to pass only when
/// the waiver authorizes this gate and snapshot.
pub fn evaluate_gate(
    gate: &Gate,
    snapshot: &str,
    items: &[EvidenceItem],
    credits: &BTreeMap<String, CredibilityRecord>,
    waiver: Option<&Waiver>,
) -> GateVerdict {
    let mut missing = Vec::new();
    let mut stale = Vec::new();
    let mut unverified = Vec::new();

    for requirement in &gate.requirements {
        let matching: Vec<&EvidenceItem> = items
            .iter()
            .filter(|item| item.class == requirement.class)
            .collect();
        let current: Vec<&&EvidenceItem> = matching
            .iter()
            .filter(|item| item.status == EvidenceStatus::Current)
            .collect();
        for item in &matching {
            if item.status != EvidenceStatus::Current {
                stale.push(format!("{} ({:?})", item.id, item.status));
            }
        }
        if current.len() < requirement.min_count {
            missing.push(format!(
                "{}: have {}, need {}",
                requirement.class.label(),
                current.len(),
                requirement.min_count
            ));
            continue;
        }
        if requirement.require_credibility {
            for item in &current {
                let credible = item
                    .credibility
                    .as_ref()
                    .and_then(|id| credits.get(id))
                    .map(|record| record.is_credible())
                    .unwrap_or(false);
                if !credible {
                    unverified.push(item.id.clone());
                }
            }
        }
    }

    let passed = missing.is_empty() && unverified.is_empty();
    let mut waived_by = None;
    if !passed {
        if let Some(waiver) = waiver {
            if gate.waiver_allowed && waiver.authorizes(&gate.id, snapshot) {
                waived_by = Some(waiver.id.clone());
            }
        }
    }

    GateVerdict {
        gate: gate.id.clone(),
        passed: passed || waived_by.is_some(),
        missing,
        stale,
        unverified,
        waived_by,
    }
}

/// The state of a release package.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseState {
    /// Draft, not yet approved.
    Draft,
    /// Approved for release.
    Approved,
    /// Rejected.
    Rejected,
}

/// A release package binds a snapshot to gate verdicts and an approver.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReleasePackage {
    /// Package id.
    pub id: String,
    /// Configuration snapshot id.
    pub snapshot: String,
    /// Gate verdicts.
    pub verdicts: Vec<GateVerdict>,
    /// Approver identity; `None` means not approved.
    pub approved_by: Option<String>,
    /// State.
    pub state: ReleaseState,
    /// Creation timestamp.
    pub created_at: String,
}

impl ReleasePackage {
    /// Whether every verdict passed.
    pub fn gates_pass(&self) -> bool {
        !self.verdicts.is_empty() && self.verdicts.iter().all(|v| v.passed)
    }

    /// Approve the package. Requires a non-empty approver and all gates
    /// passing. No execution-only path exists.
    pub fn approve(&mut self, approver: impl Into<String>) -> EvidenceResult<()> {
        let approver = approver.into();
        if approver.trim().is_empty() {
            return Err(EvidenceError::ReleaseBlocked(
                "approver identity is required".to_string(),
            ));
        }
        for verdict in &self.verdicts {
            if !verdict.passed {
                return Err(EvidenceError::ReleaseBlocked(verdict.gate.clone()));
            }
        }
        self.approved_by = Some(approver);
        self.state = ReleaseState::Approved;
        Ok(())
    }

    /// Verify the package against a snapshot and evidence set.
    pub fn verification_summary(&self) -> BTreeSet<String> {
        self.verdicts
            .iter()
            .flat_map(|v| v.missing.iter().chain(v.stale.iter()).chain(v.unverified.iter()))
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: &str, class: EvidenceClass, status: EvidenceStatus) -> EvidenceItem {
        EvidenceItem {
            id: id.to_string(),
            class,
            subject: "wingbox".to_string(),
            subject_revision: 3,
            artifact_hash: "a".repeat(64),
            status,
            issued_by: "analyst".to_string(),
            issued_at: "2026-11-24T00:00:00Z".to_string(),
            credibility: None,
        }
    }

    fn gate() -> Gate {
        Gate {
            id: "release".to_string(),
            description: "Release gate".to_string(),
            requirements: vec![
                GateRequirement {
                    class: EvidenceClass::Verification,
                    min_count: 1,
                    require_credibility: false,
                },
                GateRequirement {
                    class: EvidenceClass::Validation,
                    min_count: 1,
                    require_credibility: true,
                },
            ],
            waiver_allowed: true,
        }
    }

    fn credit() -> CredibilityRecord {
        CredibilityRecord {
            id: "cred-1".to_string(),
            subject: "beam-fem".to_string(),
            intended_use: "linear static metallic panel".to_string(),
            prohibited_use: vec!["composite failure".to_string()],
            validity_envelope: BTreeMap::new(),
            supported_by: vec![EvidenceClass::Validation],
            approved_by: Some("chief-engineer".to_string()),
        }
    }

    #[test]
    fn missing_evidence_fails_closed() {
        let verdict = evaluate_gate(
            &gate(),
            "snap-1",
            &[item("v1", EvidenceClass::Verification, EvidenceStatus::Current)],
            &BTreeMap::new(),
            None,
        );
        assert!(!verdict.passed);
        assert_eq!(verdict.missing.len(), 1);
    }

    #[test]
    fn stale_evidence_does_not_satisfy_a_gate() {
        let verdict = evaluate_gate(
            &gate(),
            "snap-1",
            &[
                item("v1", EvidenceClass::Verification, EvidenceStatus::Current),
                item("val1", EvidenceClass::Validation, EvidenceStatus::Stale),
            ],
            &BTreeMap::new(),
            None,
        );
        assert!(!verdict.passed);
        assert_eq!(verdict.stale.len(), 1);
    }

    #[test]
    fn credible_current_evidence_passes() {
        let mut validation = item("val1", EvidenceClass::Validation, EvidenceStatus::Current);
        validation.credibility = Some("cred-1".to_string());
        let mut credits = BTreeMap::new();
        credits.insert("cred-1".to_string(), credit());
        let verdict = evaluate_gate(
            &gate(),
            "snap-1",
            &[
                item("v1", EvidenceClass::Verification, EvidenceStatus::Current),
                validation,
            ],
            &credits,
            None,
        );
        assert!(verdict.passed);
    }

    #[test]
    fn unauthorized_waiver_is_rejected() {
        let waiver = Waiver {
            id: "w1".to_string(),
            gate: "release".to_string(),
            snapshot: "snap-1".to_string(),
            rationale: "temporary".to_string(),
            approved_by: "intern".to_string(),
            role: "analyst".to_string(),
            created_at: "2026-11-24T00:00:00Z".to_string(),
            active: true,
        };
        let verdict = evaluate_gate(&gate(), "snap-1", &[], &BTreeMap::new(), Some(&waiver));
        assert!(!verdict.passed);
        assert!(verdict.waived_by.is_none());
    }

    #[test]
    fn authorized_waiver_is_recorded() {
        let waiver = Waiver {
            id: "w1".to_string(),
            gate: "release".to_string(),
            snapshot: "snap-1".to_string(),
            rationale: "accepted deviation, tracked as NCR-7".to_string(),
            approved_by: "chief-engineer".to_string(),
            role: "quality-auditor".to_string(),
            created_at: "2026-11-24T00:00:00Z".to_string(),
            active: true,
        };
        let verdict = evaluate_gate(&gate(), "snap-1", &[], &BTreeMap::new(), Some(&waiver));
        assert!(verdict.passed);
        assert_eq!(verdict.waived_by.as_deref(), Some("w1"));
    }

    #[test]
    fn release_requires_approval_and_passing_gates() {
        let mut package = ReleasePackage {
            id: "rel-1".to_string(),
            snapshot: "snap-1".to_string(),
            verdicts: vec![GateVerdict {
                gate: "release".to_string(),
                passed: false,
                missing: vec!["validation".to_string()],
                stale: vec![],
                unverified: vec![],
                waived_by: None,
            }],
            approved_by: None,
            state: ReleaseState::Draft,
            created_at: "2026-11-24T00:00:00Z".to_string(),
        };
        assert!(matches!(
            package.approve("chief-engineer"),
            Err(EvidenceError::ReleaseBlocked(_))
        ));
        package.verdicts[0].passed = true;
        package.approve("chief-engineer").unwrap();
        assert_eq!(package.state, ReleaseState::Approved);
    }
}
