// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! Model cards, promotion gates, and an auditable model registry.
//!
//! A model — a surrogate, a reduced-order model, or an AI-assisted design
//! service — is only usable inside a declared validity envelope, with
//! declared evidence, and only after an authorized promotion decision.
//! This crate makes those rules executable. It does not train models and
//! it does not decide engineering truth; it records and checks who was
//! allowed to promote which model, with what evidence, and why.

#![deny(unsafe_code)]
#![warn(missing_docs)]

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use thiserror::Error;

/// Evidence class attached to a model card.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceClass {
    /// Deterministic verification against a reference implementation.
    Verification,
    /// Physical or experimental validation.
    Validation,
    /// Independent benchmark comparison.
    Benchmark,
    /// Independent human review.
    IndependentReview,
}

/// Freshness of an evidence item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceStatus {
    /// Applicable to the current model version.
    Current,
    /// Recorded, but for an older model version.
    Stale,
    /// Referenced but not supplied.
    Missing,
}

/// One evidence reference.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvidenceRef {
    /// Evidence identifier.
    pub id: String,
    /// Evidence class.
    pub class: EvidenceClass,
    /// Model version the evidence applies to.
    pub subject_version: String,
    /// Freshness.
    pub status: EvidenceStatus,
    /// Whether the evidence was produced independently of the model author.
    #[serde(default)]
    pub independent: bool,
}

/// Model lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelState {
    /// Being built; not usable.
    Draft,
    /// Submitted with evidence; not yet promoted.
    Candidate,
    /// Evidence checked; promotion pending.
    Validated,
    /// Approved for use inside its validity envelope.
    Promoted,
    /// Withdrawn from use.
    Retired,
}

/// A model card.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelCard {
    /// Stable model identifier.
    pub id: String,
    /// Semantic version of the model artifact.
    pub version: String,
    /// Human name.
    pub name: String,
    /// What the model may be used for.
    pub intended_use: String,
    /// What the model must not be used for.
    #[serde(default)]
    pub prohibited_use: Vec<String>,
    /// Declared validity envelope. Values are data, not prose.
    #[serde(default)]
    pub validity_envelope: BTreeMap<String, Value>,
    /// Evidence attached to this version.
    #[serde(default)]
    pub evidence: Vec<EvidenceRef>,
    /// Authorized approver identity, if recorded.
    #[serde(default)]
    pub approved_by: Option<String>,
    /// Lifecycle state.
    pub state: ModelState,
}

/// Promotion gate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PromotionGate {
    /// Evidence classes that must be present and current.
    pub required: Vec<EvidenceClass>,
    /// Require at least one independent validation item.
    #[serde(default)]
    pub require_independent_validation: bool,
    /// Roles allowed to approve promotion.
    pub authorized_roles: Vec<String>,
}

impl Default for PromotionGate {
    fn default() -> Self {
        Self {
            required: vec![
                EvidenceClass::Verification,
                EvidenceClass::Validation,
                EvidenceClass::Benchmark,
            ],
            require_independent_validation: true,
            authorized_roles: vec![
                "chief-engineer".to_string(),
                "model-governance-board".to_string(),
            ],
        }
    }
}

/// Result of evaluating a promotion request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PromotionDecision {
    /// Whether promotion is allowed.
    pub passed: bool,
    /// Model identifier.
    pub model_id: String,
    /// Model version.
    pub version: String,
    /// Required classes with no evidence at all.
    pub missing: Vec<String>,
    /// Required classes with stale evidence.
    pub stale: Vec<String>,
    /// Human-readable reasons the decision failed.
    pub reasons: Vec<String>,
    /// Whether independent validation was found.
    pub independent_validation: bool,
    /// Approver role supplied to the decision.
    pub approver_role: String,
}

/// Registry errors.
#[derive(Debug, Error, PartialEq)]
pub enum RegistryError {
    /// A model version is already registered.
    #[error("model {id} version {version} is already registered")]
    Duplicate {
        /// Model id.
        id: String,
        /// Model version.
        version: String,
    },
    /// The model was not found.
    #[error("model {id} version {version} is not registered")]
    NotFound {
        /// Model id.
        id: String,
        /// Model version.
        version: String,
    },
    /// Promotion was refused by the gate.
    #[error("promotion refused: {}", .0.join("; "))]
    Refused(Vec<String>),
    /// The audit chain is invalid.
    #[error("registry audit chain invalid at sequence {0}")]
    AuditChain(u64),
}

/// One auditable registry event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegistryEvent {
    /// Monotonic sequence number.
    pub seq: u64,
    /// Action, e.g. `register`, `promote`, `retire`.
    pub action: String,
    /// Model id.
    pub model_id: String,
    /// Model version.
    pub version: String,
    /// Actor identity.
    pub actor: String,
    /// ISO-8601 timestamp.
    pub at: String,
    /// Hash of the previous event.
    pub prev_hash: String,
    /// Hash of this event's canonical body.
    pub hash: String,
}

/// An in-memory, append-only model registry.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ModelRegistry {
    /// Registered cards keyed by `id@version`.
    pub cards: BTreeMap<String, ModelCard>,
    /// Append-only audit events.
    pub audit: Vec<RegistryEvent>,
}

impl ModelRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a model card.
    pub fn register(&mut self, card: ModelCard, actor: &str, at: &str) -> Result<(), RegistryError> {
        let key = key(&card.id, &card.version);
        if self.cards.contains_key(&key) {
            return Err(RegistryError::Duplicate {
                id: card.id,
                version: card.version,
            });
        }
        self.cards.insert(key, card.clone());
        self.append("register", &card.id, &card.version, actor, at);
        Ok(())
    }

    /// Evaluate and, if allowed, apply promotion.
    pub fn promote(
        &mut self,
        id: &str,
        version: &str,
        gate: &PromotionGate,
        approver_role: &str,
        at: &str,
    ) -> Result<PromotionDecision, RegistryError> {
        let key = key(id, version);
        let card = self.cards.get(&key).ok_or_else(|| RegistryError::NotFound {
            id: id.to_string(),
            version: version.to_string(),
        })?;
        let decision = evaluate_promotion(card, gate, approver_role);
        if !decision.passed {
            return Err(RegistryError::Refused(decision.reasons.clone()));
        }
        let card = self.cards.get_mut(&key).expect("checked above");
        card.state = ModelState::Promoted;
        card.approved_by = Some(approver_role.to_string());
        let card = card.clone();
        self.append("promote", &card.id, &card.version, approver_role, at);
        Ok(decision)
    }

    /// Retire a model version.
    pub fn retire(
        &mut self,
        id: &str,
        version: &str,
        actor: &str,
        at: &str,
    ) -> Result<(), RegistryError> {
        let key = key(id, version);
        let card = self.cards.get_mut(&key).ok_or_else(|| RegistryError::NotFound {
            id: id.to_string(),
            version: version.to_string(),
        })?;
        card.state = ModelState::Retired;
        let card = card.clone();
        self.append("retire", &card.id, &card.version, actor, at);
        Ok(())
    }

    /// Verify the audit hash chain.
    pub fn verify_audit(&self) -> Result<(), RegistryError> {
        let mut previous = "genesis".to_string();
        for (index, event) in self.audit.iter().enumerate() {
            if event.seq != index as u64 + 1 || event.prev_hash != previous {
                return Err(RegistryError::AuditChain(event.seq));
            }
            if event.hash != event_hash(event) {
                return Err(RegistryError::AuditChain(event.seq));
            }
            previous = event.hash.clone();
        }
        Ok(())
    }

    fn append(&mut self, action: &str, id: &str, version: &str, actor: &str, at: &str) {
        let previous = self
            .audit
            .last()
            .map(|event| event.hash.clone())
            .unwrap_or_else(|| "genesis".to_string());
        let mut event = RegistryEvent {
            seq: self.audit.len() as u64 + 1,
            action: action.to_string(),
            model_id: id.to_string(),
            version: version.to_string(),
            actor: actor.to_string(),
            at: at.to_string(),
            prev_hash: previous,
            hash: String::new(),
        };
        event.hash = event_hash(&event);
        self.audit.push(event);
    }
}

/// Evaluate a promotion request without mutating a registry.
pub fn evaluate_promotion(
    card: &ModelCard,
    gate: &PromotionGate,
    approver_role: &str,
) -> PromotionDecision {
    let mut reasons = Vec::new();
    let mut missing = Vec::new();
    let mut stale = Vec::new();

    if card.intended_use.trim().is_empty() {
        reasons.push("intended_use is empty".to_string());
    }
    if card.validity_envelope.is_empty() {
        reasons.push("validity_envelope is empty".to_string());
    }
    if !is_semver(&card.version) {
        reasons.push(format!("version {:?} is not semantic", card.version));
    }
    if !matches!(card.state, ModelState::Candidate | ModelState::Validated) {
        reasons.push(format!(
            "model state {:?} is not promotable (expected candidate or validated)",
            card.state
        ));
    }
    if !gate
        .authorized_roles
        .iter()
        .any(|role| role == approver_role)
    {
        reasons.push(format!(
            "role {approver_role:?} is not authorized to promote models"
        ));
    }

    let current_classes: BTreeSet<EvidenceClass> = card
        .evidence
        .iter()
        .filter(|e| e.status == EvidenceStatus::Current && e.subject_version == card.version)
        .map(|e| e.class)
        .collect();
    for required in &gate.required {
        if current_classes.contains(required) {
            continue;
        }
        let has_stale = card.evidence.iter().any(|e| {
            e.class == *required
                && (e.status == EvidenceStatus::Stale
                    || e.subject_version != card.version)
        });
        if has_stale {
            stale.push(format!("{required:?}"));
        } else {
            missing.push(format!("{required:?}"));
        }
    }

    let independent_validation = card.evidence.iter().any(|e| {
        e.class == EvidenceClass::Validation
            && e.status == EvidenceStatus::Current
            && e.subject_version == card.version
            && e.independent
    });
    if gate.require_independent_validation && !independent_validation {
        reasons.push("no current independent validation evidence".to_string());
    }

    if !missing.is_empty() {
        reasons.push(format!("missing current evidence: {}", missing.join(", ")));
    }
    if !stale.is_empty() {
        reasons.push(format!("stale evidence: {}", stale.join(", ")));
    }

    PromotionDecision {
        passed: reasons.is_empty(),
        model_id: card.id.clone(),
        version: card.version.clone(),
        missing,
        stale,
        reasons,
        independent_validation,
        approver_role: approver_role.to_string(),
    }
}

fn is_semver(version: &str) -> bool {
    let core = version.split(['-', '+']).next().unwrap_or(version);
    let parts: Vec<&str> = core.split('.').collect();
    parts.len() == 3
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_digit()))
}

fn key(id: &str, version: &str) -> String {
    format!("{id}@{version}")
}

fn event_hash(event: &RegistryEvent) -> String {
    let canonical = serde_json::json!({
        "seq": event.seq,
        "action": event.action,
        "model_id": event.model_id,
        "version": event.version,
        "actor": event.actor,
        "at": event.at,
        "prev_hash": event.prev_hash,
    });
    let bytes = serde_json::to_vec(&canonical).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn evidence(class: EvidenceClass, independent: bool) -> EvidenceRef {
        EvidenceRef {
            id: format!("ev-{class:?}"),
            class,
            subject_version: "0.2.0".to_string(),
            status: EvidenceStatus::Current,
            independent,
        }
    }

    fn card() -> ModelCard {
        ModelCard {
            id: "surrogate-wingbox".to_string(),
            version: "0.2.0".to_string(),
            name: "Wing-box deflection surrogate".to_string(),
            intended_use: "Screening inside the declared envelope".to_string(),
            prohibited_use: vec!["Certification credit".to_string()],
            validity_envelope: BTreeMap::from([(
                "tip_load_kN".to_string(),
                serde_json::json!("1 .. 100"),
            )]),
            evidence: vec![
                evidence(EvidenceClass::Verification, false),
                evidence(EvidenceClass::Validation, true),
                evidence(EvidenceClass::Benchmark, true),
            ],
            approved_by: None,
            state: ModelState::Candidate,
        }
    }

    #[test]
    fn complete_card_promotes() {
        let decision = evaluate_promotion(&card(), &PromotionGate::default(), "chief-engineer");
        assert!(decision.passed, "{:?}", decision.reasons);
        assert!(decision.independent_validation);
    }

    #[test]
    fn missing_validation_blocks_promotion() {
        let mut card = card();
        card.evidence
            .retain(|e| e.class != EvidenceClass::Validation);
        let decision = evaluate_promotion(&card, &PromotionGate::default(), "chief-engineer");
        assert!(!decision.passed);
        assert!(decision.missing.iter().any(|m| m.contains("Validation")));
    }

    #[test]
    fn stale_evidence_blocks_promotion() {
        let mut card = card();
        card.evidence[0].status = EvidenceStatus::Stale;
        let decision = evaluate_promotion(&card, &PromotionGate::default(), "chief-engineer");
        assert!(!decision.passed);
        assert!(decision.stale.iter().any(|m| m.contains("Verification")));
    }

    #[test]
    fn unauthorized_role_is_refused() {
        let decision = evaluate_promotion(&card(), &PromotionGate::default(), "analyst");
        assert!(!decision.passed);
        assert!(decision.reasons.iter().any(|r| r.contains("not authorized")));
    }

    #[test]
    fn non_independent_validation_is_refused() {
        let mut card = card();
        for evidence in &mut card.evidence {
            if evidence.class == EvidenceClass::Validation {
                evidence.independent = false;
            }
        }
        let decision = evaluate_promotion(&card, &PromotionGate::default(), "chief-engineer");
        assert!(!decision.passed);
        assert!(decision
            .reasons
            .iter()
            .any(|r| r.contains("independent validation")));
    }

    #[test]
    fn registry_audit_chain_is_valid_and_detects_tamper() {
        let mut registry = ModelRegistry::new();
        registry
            .register(card(), "analyst", "2026-11-24T00:00:00Z")
            .unwrap();
        let decision = registry
            .promote(
                "surrogate-wingbox",
                "0.2.0",
                &PromotionGate::default(),
                "chief-engineer",
                "2026-11-24T01:00:00Z",
            )
            .unwrap();
        assert!(decision.passed);
        assert_eq!(registry.audit.len(), 2);
        assert!(registry.verify_audit().is_ok());

        registry.audit[0].actor = "intruder".to_string();
        assert!(matches!(
            registry.verify_audit(),
            Err(RegistryError::AuditChain(1))
        ));
    }

    #[test]
    fn duplicate_registration_is_rejected() {
        let mut registry = ModelRegistry::new();
        registry
            .register(card(), "analyst", "2026-11-24T00:00:00Z")
            .unwrap();
        assert!(matches!(
            registry.register(card(), "analyst", "2026-11-24T00:00:00Z"),
            Err(RegistryError::Duplicate { .. })
        ));
    }
}
