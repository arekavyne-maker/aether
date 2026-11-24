// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! Graph access control (ADR-0037).
//!
//! `aether_graph` records `actor` and `authz_decision` on every event but
//! does not evaluate policy. This module is the decision point: every graph
//! operation must pass through [`authorize`] with a subject and an action.
//! Denials are typed errors; there is no path that defaults to allow.

#![deny(unsafe_code)]
#![warn(missing_docs)]

use thiserror::Error;

use aether_policy::{PolicyDecision, PolicyEngine, PolicyRequest, Subject};

/// Authorization errors.
#[derive(Debug, Error, PartialEq)]
pub enum AuthError {
    /// The action is denied.
    #[error("denied: {0}")]
    Denied(String),
}

/// A graph operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphAction {
    /// Read nodes and edges.
    Query,
    /// Create or update nodes.
    Mutate,
    /// Verify the event chain.
    VerifyChain,
}

impl GraphAction {
    /// Wire label understood by the policy engine.
    pub fn as_str(self) -> &'static str {
        match self {
            GraphAction::Query => "graph.query",
            GraphAction::Mutate => "graph.mutate",
            GraphAction::VerifyChain => "graph.chain_verify",
        }
    }
}

/// Authorize a graph operation. Returns the decision reason on success.
pub fn authorize(
    engine: &PolicyEngine,
    subject: &Subject,
    action: GraphAction,
) -> Result<String, AuthError> {
    let request = PolicyRequest {
        subject: subject.clone(),
        action: action.as_str().to_string(),
        context: serde_json::json!({ "boundary": "graph" }),
    };
    match engine.evaluate(&request) {
        PolicyDecision::Allow => Ok("rbac allow".to_string()),
        PolicyDecision::Deny { reason } => Err(AuthError::Denied(reason)),
    }
}

/// Authorize with the default fail-closed engine.
pub fn authorize_default(
    subject: &Subject,
    action: GraphAction,
) -> Result<String, AuthError> {
    authorize(&PolicyEngine::new(), subject, action)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn subject(role: &str) -> Subject {
        Subject { id: "user:test".to_string(), role: role.to_string() }
    }

    #[test]
    fn engineer_can_query_and_mutate() {
        assert!(authorize_default(&subject("project-engineer"), GraphAction::Query).is_ok());
        assert!(authorize_default(&subject("project-engineer"), GraphAction::Mutate).is_ok());
    }

    #[test]
    fn auditor_cannot_mutate() {
        let err = authorize_default(&subject("quality-auditor"), GraphAction::Mutate).unwrap_err();
        assert!(matches!(err, AuthError::Denied(_)));
    }

    #[test]
    fn analyst_cannot_mutate() {
        assert!(authorize_default(&subject("analyst"), GraphAction::Mutate).is_err());
    }

    #[test]
    fn unknown_role_is_denied() {
        assert!(authorize_default(&subject("intruder"), GraphAction::Query).is_err());
    }

    #[test]
    fn deny_all_engine_denies_everything() {
        let engine = PolicyEngine::deny_all();
        assert!(authorize(&engine, &subject("admin"), GraphAction::Query).is_err());
    }
}