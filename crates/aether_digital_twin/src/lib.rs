// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! Digital twin maturity and in-service feedback (spec §11).
//!
//! A twin is not a generic label. Levels 1–3 are supported; level 4 and 5
//! require operational validation and a separate safety programme. Sensor
//! updates are proposals and never change a released allowable.

#![deny(unsafe_code)]
#![warn(missing_docs)]

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Twin maturity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TwinLevel {
    /// Structured model and documents.
    Representation,
    /// Measured data flows in; no automated physical control.
    Shadow,
    /// Forecasts behaviour and degradation.
    Predictive,
    /// Proposes maintenance or engineering actions.
    DecisionSupport,
    /// Sends authorised control actions to a physical process.
    ClosedLoop,
}

/// An in-service observation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Observation {
    /// Sensor or source id.
    pub source: String,
    /// Measurement kind.
    pub kind: String,
    /// Numeric value.
    pub value: f64,
    /// Unit.
    pub unit: String,
    /// ISO-8601 time.
    pub at: String,
}

/// A model update proposal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelUpdateProposal {
    /// Proposal id.
    pub id: String,
    /// Subject model id.
    pub subject: String,
    /// Proposed parameter changes.
    pub changes: BTreeMap<String, f64>,
    /// Evidence ids supporting the update.
    pub evidence: Vec<String>,
    /// Approver; `None` means not yet approved.
    pub approved_by: Option<String>,
}

/// Twin errors.
#[derive(Debug, Error, PartialEq)]
pub enum TwinError {
    /// The level is not supported without validation.
    #[error("level {0:?} requires separate validation and governance")]
    UnsupportedLevel(TwinLevel),
    /// A model update was applied without approval.
    #[error("model update is a proposal; approval is required")]
    UnapprovedUpdate,
}

/// A digital twin state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DigitalTwin {
    /// Twin id.
    pub id: String,
    /// Declared maturity level.
    pub level: TwinLevel,
    /// Latest observations.
    pub observations: Vec<Observation>,
    /// Pending model updates.
    pub updates: Vec<ModelUpdateProposal>,
}

impl DigitalTwin {
    /// Create a twin, refusing unsupported levels.
    pub fn new(id: &str, level: TwinLevel) -> Result<Self, TwinError> {
        if level > TwinLevel::Predictive {
            return Err(TwinError::UnsupportedLevel(level));
        }
        Ok(Self {
            id: id.to_string(),
            level,
            observations: Vec::new(),
            updates: Vec::new(),
        })
    }

    /// Ingest an observation. At shadow level or above this is allowed.
    pub fn observe(&mut self, observation: Observation) -> Result<(), TwinError> {
        if self.level < TwinLevel::Shadow {
            return Err(TwinError::UnsupportedLevel(self.level));
        }
        self.observations.push(observation);
        Ok(())
    }

    /// Submit a model update as a proposal. It is never applied here.
    pub fn propose_update(
        &mut self,
        mut proposal: ModelUpdateProposal,
    ) -> Result<(), TwinError> {
        if proposal.id.trim().is_empty() || proposal.subject.trim().is_empty() {
            return Err(TwinError::UnapprovedUpdate);
        }
        proposal.approved_by = None;
        self.updates.push(proposal);
        Ok(())
    }

    /// Approve an update with a named authority.
    pub fn approve(&mut self, id: &str, approver: &str) -> Result<(), TwinError> {
        if approver.trim().is_empty() {
            return Err(TwinError::UnapprovedUpdate);
        }
        let update = self
            .updates
            .iter_mut()
            .find(|u| u.id == id)
            .ok_or_else(|| TwinError::UnapprovedUpdate)?;
        update.approved_by = Some(approver.to_string());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observation() -> Observation {
        Observation {
            source: "strain-1".into(), kind: "strain".into(), value: 120e-6,
            unit: "m/m".into(), at: "2026-11-24T00:00:00Z".into(),
        }
    }

    #[test]
    fn representation_twin_cannot_ingest_measurements() {
        let mut twin = DigitalTwin::new("t1", TwinLevel::Representation).unwrap();
        assert!(twin.observe(observation()).is_err());
    }

    #[test]
    fn shadow_twin_ingests_measurements() {
        let mut twin = DigitalTwin::new("t2", TwinLevel::Shadow).unwrap();
        twin.observe(observation()).unwrap();
        assert_eq!(twin.observations.len(), 1);
    }

    #[test]
    fn decision_support_level_is_not_auto_granted() {
        assert!(DigitalTwin::new("t3", TwinLevel::DecisionSupport).is_err());
    }

    #[test]
    fn model_updates_require_approval() {
        let mut twin = DigitalTwin::new("t4", TwinLevel::Predictive).unwrap();
        let proposal = ModelUpdateProposal {
            id: "u1".into(), subject: "model-1".into(),
            changes: BTreeMap::from([("k".into(), 1.5)]), evidence: vec![], approved_by: None,
        };
        twin.propose_update(proposal).unwrap();
        assert!(twin.updates[0].approved_by.is_none());
        twin.approve("u1", "chief-engineer").unwrap();
        assert_eq!(twin.updates[0].approved_by.as_deref(), Some("chief-engineer"));
    }
}