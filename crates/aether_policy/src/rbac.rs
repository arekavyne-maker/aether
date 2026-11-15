// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! Role-Based Access Control for Aether (SDP §7.4).
//!
//! G1 ships the role catalog and the static action matrix. Dynamic
//! attribute checks (e.g. "is this actor the owner of this Requirement?")
//! land with the CEL evaluator in G2.
//!
//! Roles (initial catalog, SDP §7.4):
//! - `admin`                — full control
//! - `platform-operator`    — install / configure plugins
//! - `project-engineer`     — manage requirements, geometry, workflow
//! - `analyst`              — run simulations, view results
//! - `manufacturing-engineer` — manage process plans, inspection, test
//! - `test-engineer`        — manage test rigs
//! - `quality-auditor`      — read all, write none
//! - `service-account`      — automated agents (plugins, scripts)

#![deny(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

use crate::Subject;

#[derive(Debug, Error)]
pub enum RbacError {
    #[error("unknown role: {0}")]
    UnknownRole(String),
    #[error("unknown action: {0}")]
    UnknownAction(String),
}

pub type RbacResult<T> = Result<T, RbacError>;

/// Every action a subject can request. Each action is a typed label; the
/// catalog maps role -> set of allowed actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    ProjectInit,
    ProjectOpen,
    GraphQuery,
    GraphMutate,
    GraphChainVerify,
    RequirementAdd,
    RequirementLink,
    RequirementVerifyStatus,
    GeometryImport,
    GeometryExport,
    SimulationRun,
    WorkflowRun,
    PluginInstall,
    PluginCertify,
    PackageCreate,
    PackageSign,
    PackageVerify,
    AuditExport,
    PolicyEdit,
    AdminRbac,
}

impl Action {
    pub fn as_str(&self) -> &'static str {
        match self {
            Action::ProjectInit => "project.init",
            Action::ProjectOpen => "project.open",
            Action::GraphQuery => "graph.query",
            Action::GraphMutate => "graph.mutate",
            Action::GraphChainVerify => "graph.chain_verify",
            Action::RequirementAdd => "requirement.add",
            Action::RequirementLink => "requirement.link",
            Action::RequirementVerifyStatus => "requirement.verify_status",
            Action::GeometryImport => "geometry.import",
            Action::GeometryExport => "geometry.export",
            Action::SimulationRun => "simulation.run",
            Action::WorkflowRun => "workflow.run",
            Action::PluginInstall => "plugin.install",
            Action::PluginCertify => "plugin.certify",
            Action::PackageCreate => "package.create",
            Action::PackageSign => "package.sign",
            Action::PackageVerify => "package.verify",
            Action::AuditExport => "audit.export",
            Action::PolicyEdit => "policy.edit",
            Action::AdminRbac => "admin.rbac",
        }
    }
}

/// The static role catalog. Each role is the set of actions it may take.
#[derive(Debug, Clone)]
pub struct Role {
    pub name: &'static str,
    pub description: &'static str,
    pub actions: &'static [Action],
}

pub const ROLE_CATALOG: &[Role] = &[
    Role {
        name: "admin",
        description: "Full control over the platform",
        actions: &[
            Action::ProjectInit,
            Action::ProjectOpen,
            Action::GraphQuery,
            Action::GraphMutate,
            Action::GraphChainVerify,
            Action::RequirementAdd,
            Action::RequirementLink,
            Action::RequirementVerifyStatus,
            Action::GeometryImport,
            Action::GeometryExport,
            Action::SimulationRun,
            Action::WorkflowRun,
            Action::PluginInstall,
            Action::PluginCertify,
            Action::PackageCreate,
            Action::PackageSign,
            Action::PackageVerify,
            Action::AuditExport,
            Action::PolicyEdit,
            Action::AdminRbac,
        ],
    },
    Role {
        name: "platform-operator",
        description: "Install and configure plugins and policies",
        actions: &[
            Action::ProjectOpen,
            Action::GraphQuery,
            Action::PluginInstall,
            Action::PluginCertify,
            Action::PackageVerify,
            Action::AuditExport,
            Action::PolicyEdit,
        ],
    },
    Role {
        name: "project-engineer",
        description: "Manage requirements, geometry, and workflows",
        actions: &[
            Action::ProjectInit,
            Action::ProjectOpen,
            Action::GraphQuery,
            Action::GraphMutate,
            Action::RequirementAdd,
            Action::RequirementLink,
            Action::RequirementVerifyStatus,
            Action::GeometryImport,
            Action::GeometryExport,
            Action::WorkflowRun,
            Action::PackageCreate,
        ],
    },
    Role {
        name: "analyst",
        description: "Run simulations and view results",
        actions: &[
            Action::ProjectOpen,
            Action::GraphQuery,
            Action::SimulationRun,
            Action::WorkflowRun,
        ],
    },
    Role {
        name: "manufacturing-engineer",
        description: "Manage process plans, inspection, and test",
        actions: &[
            Action::ProjectOpen,
            Action::GraphQuery,
            Action::GraphMutate,
            Action::WorkflowRun,
            Action::PackageCreate,
        ],
    },
    Role {
        name: "test-engineer",
        description: "Manage test rigs and inspection devices",
        actions: &[
            Action::ProjectOpen,
            Action::GraphQuery,
            Action::GraphMutate,
            Action::WorkflowRun,
        ],
    },
    Role {
        name: "quality-auditor",
        description: "Read all, write none",
        actions: &[
            Action::ProjectOpen,
            Action::GraphQuery,
            Action::GraphChainVerify,
            Action::PackageVerify,
            Action::AuditExport,
        ],
    },
    Role {
        name: "service-account",
        description: "Automated agents (plugins, scripts)",
        actions: &[
            Action::ProjectOpen,
            Action::GraphQuery,
            Action::GraphMutate,
            Action::RequirementAdd,
            Action::WorkflowRun,
        ],
    },
];

/// RBAC policy: a runtime instance that resolves a role name to its
/// actions and answers `evaluate` for a request.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RbacPolicy {
    /// Optional custom role additions (a future hook for site-specific
    /// roles; the canonical catalog is in `ROLE_CATALOG`).
    #[serde(default)]
    pub custom_roles: BTreeMap<String, Vec<Action>>,
}

impl RbacPolicy {
    pub fn new() -> Self {
        Self::default()
    }

    /// Look up the actions allowed for a role name.
    pub fn actions_for(&self, role: &str) -> RbacResult<&[Action]> {
        if let Some(actions) = self.custom_roles.get(role) {
            // A boxed slice would be nicer, but `Vec<Action>` is fine here
            // for a small cardinality. Return an empty slice if the role
            // exists but has no actions.
            let _ = actions;
        }
        if let Some(r) = ROLE_CATALOG.iter().find(|r| r.name == role) {
            return Ok(r.actions);
        }
        if self.custom_roles.contains_key(role) {
            return Ok(&[]);
        }
        Err(RbacError::UnknownRole(role.to_string()))
    }
}

/// The result of an RBAC check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RbacDecision {
    Allow,
    Deny { reason: String },
}

impl RbacPolicy {
    /// Evaluate an RBAC request.
    pub fn evaluate(&self, request: &RbacRequest) -> RbacResult<RbacDecision> {
        let actions = self.actions_for(&request.subject.role)?;
        if actions.contains(&request.action) {
            Ok(RbacDecision::Allow)
        } else {
            Ok(RbacDecision::Deny {
                reason: format!(
                    "role {:?} is not permitted to perform {:?}",
                    request.subject.role, request.action
                ),
            })
        }
    }
}

/// An RBAC evaluation request: subject + action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RbacRequest {
    pub subject: Subject,
    pub action: Action,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(role: &str) -> Subject {
        Subject { id: "u".to_string(), role: role.to_string() }
    }

    #[test]
    fn admin_can_do_anything() {
        let p = RbacPolicy::new();
        let r = p.evaluate(&RbacRequest { subject: s("admin"), action: Action::PluginInstall }).unwrap();
        assert_eq!(r, RbacDecision::Allow);
    }

    #[test]
    fn quality_auditor_cannot_mutate() {
        let p = RbacPolicy::new();
        let r = p.evaluate(&RbacRequest { subject: s("quality-auditor"), action: Action::GraphMutate }).unwrap();
        assert!(matches!(r, RbacDecision::Deny { .. }));
    }

    #[test]
    fn analyst_can_run_simulation() {
        let p = RbacPolicy::new();
        let r = p.evaluate(&RbacRequest { subject: s("analyst"), action: Action::SimulationRun }).unwrap();
        assert_eq!(r, RbacDecision::Allow);
    }

    #[test]
    fn unknown_role_denied() {
        let p = RbacPolicy::new();
        let err = p.evaluate(&RbacRequest { subject: s("intruder"), action: Action::ProjectOpen }).unwrap_err();
        assert!(matches!(err, RbacError::UnknownRole(_)));
    }

    #[test]
    fn service_account_cannot_install_plugins() {
        let p = RbacPolicy::new();
        let r = p.evaluate(&RbacRequest { subject: s("service-account"), action: Action::PluginInstall }).unwrap();
        assert!(matches!(r, RbacDecision::Deny { .. }));
    }
}
