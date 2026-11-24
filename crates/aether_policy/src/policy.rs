// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! Policy engine: fail-closed RBAC evaluation.
//!
//! The engine is deliberately small. It resolves the subject's role through
//! the canonical role catalog (or a site-specific custom role), parses the
//! requested action, and denies unknown roles or actions. It does not execute
//! arbitrary expressions; expression bundles can be added behind this
//! interface later without changing the decision point.

#![deny(unsafe_code)]

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::rbac::{Action, RbacDecision, RbacPolicy};
use crate::Subject;

#[derive(Debug, Error)]
pub enum PolicyError {
    #[error("policy evaluation failed: {0}")]
    Eval(String),
}

pub type PolicyResult<T> = Result<T, PolicyError>;

/// A request for a policy decision.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRequest {
    pub subject: Subject,
    pub action: String,
    /// Free-form context (resource id, project, etc.).
    #[serde(default)]
    pub context: serde_json::Value,
}

/// The result of a policy evaluation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PolicyDecision {
    Allow,
    Deny { reason: String },
}

/// The policy engine. Default is fail-closed: unknown role/action is denied.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PolicyEngine {
    /// Optional test mode that denies everything.
    #[serde(default)]
    pub deny_all: bool,
    /// Site-specific role extensions.
    #[serde(default)]
    pub rbac: RbacPolicy,
}

impl PolicyEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn deny_all() -> Self {
        Self { deny_all: true, ..Self::default() }
    }

    /// Evaluate a request against the role catalog. Unknown actions and
    /// unknown roles are denied, never allowed by default.
    pub fn evaluate(&self, request: &PolicyRequest) -> PolicyDecision {
        if self.deny_all {
            return PolicyDecision::Deny {
                reason: "deny-all is set (test mode)".to_string(),
            };
        }
        let action = match parse_action(&request.action) {
            Some(a) => a,
            None => {
                return PolicyDecision::Deny {
                    reason: format!("unknown action {:?}", request.action),
                }
            }
        };
        let rbac_request = crate::rbac::RbacRequest {
            subject: request.subject.clone(),
            action,
        };
        match self.rbac.evaluate(&rbac_request) {
            Ok(RbacDecision::Allow) => PolicyDecision::Allow,
            Ok(RbacDecision::Deny { reason }) => PolicyDecision::Deny { reason },
            Err(e) => PolicyDecision::Deny {
                reason: format!("rbac error: {e}"),
            },
        }
    }
}

fn parse_action(s: &str) -> Option<Action> {
    Some(match s {
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
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req(role: &str, action: &str) -> PolicyRequest {
        PolicyRequest {
            subject: Subject { id: "u".to_string(), role: role.to_string() },
            action: action.to_string(),
            context: serde_json::json!({}),
        }
    }

    #[test]
    fn admin_can_mutate() {
        let e = PolicyEngine::new();
        assert_eq!(e.evaluate(&req("admin", "graph.mutate")), PolicyDecision::Allow);
    }

    #[test]
    fn auditor_cannot_mutate() {
        let e = PolicyEngine::new();
        assert!(matches!(
            e.evaluate(&req("quality-auditor", "graph.mutate")),
            PolicyDecision::Deny { .. }
        ));
    }

    #[test]
    fn unknown_action_is_denied() {
        let e = PolicyEngine::new();
        assert!(matches!(
            e.evaluate(&req("admin", "graph.teleport")),
            PolicyDecision::Deny { .. }
        ));
    }

    #[test]
    fn unknown_role_is_denied() {
        let e = PolicyEngine::new();
        assert!(matches!(
            e.evaluate(&req("intern", "graph.query")),
            PolicyDecision::Deny { .. }
        ));
    }

    #[test]
    fn deny_all_engine_denies() {
        let e = PolicyEngine::deny_all();
        assert!(matches!(
            e.evaluate(&req("admin", "graph.mutate")),
            PolicyDecision::Deny { .. }
        ));
    }
}
