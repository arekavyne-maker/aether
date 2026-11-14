// SPDX-License-Identifier: Aether-VPSL-1.0
// SPDX-FileCopyrightText: 2026 The Aether Consortium
//! Aether policy engine and RBAC (G1).
//!
//! The policy engine is the **only** place where authorization decisions
//! are made (SDP §7.4). G1 ships the role catalog and the decision point
//! surface; full CEL expression evaluation lands in G2 (when the
//! `cel-rust` crate or an equivalent is added as a kernel dependency).
//!
//! The Phase 0 kernel treats every actor as `allow` by default; G1 adds
//! the explicit role check. The principle is **fail-closed**: an
//! unrecognized role is denied; an unrecognized action is denied.

#![deny(unsafe_code)]
#![warn(missing_docs)]

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use thiserror::Error;

pub mod rbac;
pub mod policy;

pub use rbac::{Action, Role, RbacDecision, RbacError, RbacPolicy, ROLE_CATALOG};
pub use policy::{PolicyDecision, PolicyEngine, PolicyError, PolicyRequest, PolicyResult};

/// Top-level policy result. Both RBAC and the (G2) CEL engine contribute.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Decision {
    pub allow: bool,
    pub rationale: String,
    pub rbac: RbacDecision,
    pub policy: PolicyDecision,
}

#[derive(Debug, Error)]
pub enum DecisionError {
    #[error("policy error: {0}")]
    Policy(#[from] PolicyError),
    #[error("rbac error: {0}")]
    Rbac(#[from] RbacError),
}

pub type DecisionResult = Result<Decision, DecisionError>;

use crate::rbac::RbacRequest;

/// Evaluate both RBAC and (G2 stub) policy for a request.
pub fn evaluate(
    request: &PolicyRequest,
    rbac: &RbacPolicy,
    policy_engine: &PolicyEngine,
) -> DecisionResult {
    let rbac_req = RbacRequest {
        subject: request.subject.clone(),
        action: parse_action(&request.action)?,
    };
    let rbac = rbac.evaluate(&rbac_req)?;
    let policy = policy_engine.evaluate(request);
    let allow = matches!(rbac, RbacDecision::Allow) && matches!(policy, PolicyDecision::Allow);
    let rationale = if allow {
        "rbac allow + policy allow".to_string()
    } else {
        format!(
            "denied: rbac={:?} policy={:?}",
            rbac, policy
        )
    };
    Ok(Decision {
        allow,
        rationale,
        rbac,
        policy,
    })
}

fn parse_action(s: &str) -> Result<crate::rbac::Action, DecisionError> {
    use crate::rbac::Action::*;
    Ok(match s {
        "project.init" => Action::ProjectInit,
        "project.open" => Action::ProjectOpen,
        "graph.query" => Action::GraphQuery,
        "graph.mutate" => Action::GraphMutate,
        "graph.chain_verify" => Action::GraphChainVerify,
        "requirement.add" => Action::RequirementAdd,
        "requirement.link" => Action::RequirementLink,
        "requirement.verify_status" => Action::RequirementVerifyStatus,
        "geometry.import" => Action::GeometryImport,
        "geometry.export" => Action::GeometryExport,
        "simulation.run" => Action::SimulationRun,
        "workflow.run" => Action::WorkflowRun,
        "plugin.install" => Action::PluginInstall,
        "plugin.certify" => Action::PluginCertify,
        "package.create" => Action::PackageCreate,
        "package.sign" => Action::PackageSign,
        "package.verify" => Action::PackageVerify,
        "audit.export" => Action::AuditExport,
        "policy.edit" => Action::PolicyEdit,
        "admin.rbac" => Action::AdminRbac,
        other => {
            return Err(DecisionError::Rbac(crate::rbac::RbacError::UnknownAction(
                other.to_string(),
            )))
        }
    })
}

/// A subject (person, service, plugin instance) requesting an action.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Subject {
    pub id: String,
    pub role: String,
}

/// Re-export BTreeSet for downstream code that wants to use it directly.
pub type SubjectSet = BTreeSet<Subject>;
