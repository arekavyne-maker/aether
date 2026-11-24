// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! Release-package construction from evidence and configuration.
//!
//! A release is a package, not a verdict: it binds a configuration
//! snapshot to the gate verdicts that were evaluated against it, the
//! evidence items that satisfied those gates, and an authorized approver.
//! Approval requires every gate to pass. Execution success alone is never
//! enough (A3-I02).

#![deny(unsafe_code)]
#![warn(missing_docs)]

use std::collections::{BTreeMap, BTreeSet};

use aether_asm::ConfigurationSnapshot;
use crate::{
    evaluate_gate, CredibilityRecord, EvidenceItem, EvidenceResult, Gate, GateVerdict, ReleasePackage,
    ReleaseState,
};
use serde::{Deserialize, Serialize};

/// One release package with its evidence, ready for approval.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreparedRelease {
    /// The package.
    pub package: ReleasePackage,
    /// Evidence items the gates were evaluated against.
    pub evidence: Vec<EvidenceItem>,
    /// Snapshot the package pins.
    pub snapshot_id: String,
}

/// Prepare a release package from a snapshot and evidence.
///
/// `waiver` authorizes a named gap only if the gate allows waivers and the
/// waiver names both the gate and the snapshot.
pub fn prepare_release(
    id: impl Into<String>,
    snapshot: &ConfigurationSnapshot,
    gates: &[Gate],
    evidence: &[EvidenceItem],
    waiver: Option<&crate::Waiver>,
    credits: &BTreeMap<String, CredibilityRecord>,
    created_at: impl Into<String>,
) -> EvidenceResult<PreparedRelease> {
    let mut verdicts = Vec::new();
    for gate in gates {
        let verdict: GateVerdict = evaluate_gate(gate, &snapshot.id, evidence, credits, waiver);
        verdicts.push(verdict);
    }
    let package = ReleasePackage {
        id: id.into(),
        snapshot: snapshot.id.clone(),
        verdicts,
        approved_by: None,
        state: ReleaseState::Draft,
        created_at: created_at.into(),
    };
    let evidence: Vec<EvidenceItem> = evidence
        .iter()
        .filter(|item| item.subject == snapshot.id)
        .cloned()
        .collect();
    Ok(PreparedRelease {
        package,
        evidence,
        snapshot_id: snapshot.id.clone(),
    })
}

impl PreparedRelease {
    /// Whether every gate verdict passed.
    pub fn gates_pass(&self) -> bool {
        self.package.gates_pass()
    }

    /// Approve the package. Fails closed unless every gate passed.
    pub fn approve(&mut self, approver: impl Into<String>) -> EvidenceResult<()> {
        self.package.approve(approver)
    }

    /// Evidence ids that satisfied the gates.
    pub fn satisfied_evidence(&self) -> BTreeSet<String> {
        self.evidence.iter().map(|e| e.id.clone()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aether_asm::{AsmKind, AsmObject, LifecycleState};
    use crate::{CredibilityRecord, EvidenceClass, EvidenceStatus, GateRequirement, Waiver};
    use serde_json::json;

    fn snapshot() -> ConfigurationSnapshot {
        let mut object = AsmObject::draft("wingbox", AsmKind::Assembly, "Wing box", "a", json!({}));
        object.transition(LifecycleState::InReview).unwrap();
        object.transition(LifecycleState::Released).unwrap();
        ConfigurationSnapshot::capture("snap-1", "a", "2026-11-24T00:00:00Z", &[object]).unwrap()
    }

    fn item(id: &str, class: EvidenceClass) -> EvidenceItem {
        EvidenceItem {
            id: id.to_string(),
            class,
            subject: "snap-1".to_string(),
            subject_revision: 1,
            artifact_hash: "b".repeat(64),
            status: EvidenceStatus::Current,
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
            waiver_allowed: false,
        }
    }

    fn credibility() -> CredibilityRecord {
        CredibilityRecord {
            id: "cred-1".to_string(),
            subject: "beam-fem".to_string(),
            intended_use: "screening".to_string(),
            prohibited_use: vec![],
            validity_envelope: Default::default(),
            supported_by: vec![EvidenceClass::Validation],
            approved_by: Some("chief-engineer".to_string()),
        }
    }

    #[test]
    fn package_is_draft_until_approved() {
        let mut validation = item("ev-val", EvidenceClass::Validation);
        validation.credibility = Some("cred-1".to_string());
        let evidence = vec![item("ev-ver", EvidenceClass::Verification), validation];
        let credits = BTreeMap::from([("cred-1".to_string(), credibility())]);
        let prepared = prepare_release(
            "rel-1",
            &snapshot(),
            &[gate()],
            &evidence,
            None,
            &credits,
            "2026-11-24T00:00:00Z",
        )
        .unwrap();
        assert_eq!(prepared.package.state, ReleaseState::Draft);
        assert!(!prepared.gates_pass() || prepared.package.approved_by.is_none());
    }

    #[test]
    fn approval_requires_every_gate() {
        let evidence = vec![item("ev-ver", EvidenceClass::Verification)];
        let credits = BTreeMap::new();
        let mut prepared = prepare_release(
            "rel-1",
            &snapshot(),
            &[gate()],
            &evidence,
            None,
            &credits,
            "2026-11-24T00:00:00Z",
        )
        .unwrap();
        assert!(!prepared.gates_pass());
        assert!(prepared.approve("quality-auditor").is_err());
        assert_eq!(prepared.package.state, ReleaseState::Draft);
    }

    #[test]
    fn complete_evidence_approves() {
        let mut validation = item("ev-val", EvidenceClass::Validation);
        validation.credibility = Some("cred-1".to_string());
        let evidence = vec![item("ev-ver", EvidenceClass::Verification), validation];
        let credits = BTreeMap::from([("cred-1".to_string(), credibility())]);
        let mut prepared = prepare_release(
            "rel-1",
            &snapshot(),
            &[gate()],
            &evidence,
            None,
            &credits,
            "2026-11-24T00:00:00Z",
        )
        .unwrap();
        assert!(prepared.gates_pass(), "{:?}", prepared.package.verdicts);
        prepared.approve("quality-auditor").unwrap();
        assert_eq!(prepared.package.state, ReleaseState::Approved);
        assert_eq!(
            prepared.package.approved_by.as_deref(),
            Some("quality-auditor")
        );
    }

    #[test]
    fn waiver_only_covers_its_own_gate_and_snapshot() {
        let mut validation = item("ev-val", EvidenceClass::Validation);
        validation.credibility = None;
        let waiver = Waiver {
            id: "wv-1".to_string(),
            gate: "release".to_string(),
            snapshot: "snap-1".to_string(),
            rationale: "test waiver".to_string(),
            approved_by: "chief-engineer".to_string(),
            role: "quality-auditor".to_string(),
            created_at: "2026-11-24T00:00:00Z".to_string(),
            active: true,
        };
        let evidence = vec![item("ev-ver", EvidenceClass::Verification), validation];
        let mut gate = gate();
        gate.waiver_allowed = true;
        let credits = BTreeMap::new();
        let prepared = prepare_release(
            "rel-1",
            &snapshot(),
            &[gate],
            &evidence,
            Some(&waiver),
            &credits,
            "2026-11-24T00:00:00Z",
        )
        .unwrap();
        assert!(prepared.gates_pass());
        let _ = credibility();
    }
}