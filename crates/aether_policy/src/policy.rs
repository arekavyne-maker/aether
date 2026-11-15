// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! Policy engine stub (G1; full CEL evaluation in G2).
//!
//! G1's policy engine returns a static `Allow` decision; the surface
//! is in place for the G2 evaluator to slot in. The integration point
//! is `PolicyEngine::evaluate(&PolicyRequest)`; the request carries
//! the action and a context object; the result is the decision.

#![deny(unsafe_code)]

use serde::{Deserialize, Serialize};
use thiserror::Error;

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

/// The policy engine. G1: a stub that always allows. G2: a CEL evaluator
/// loaded from a policy bundle (signed, versioned, SDP §7.4).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PolicyEngine {
    /// If true, the engine denies everything (used in tests).
    #[serde(default)]
    pub deny_all: bool,
}

impl PolicyEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn deny_all() -> Self {
        Self { deny_all: true }
    }

    /// Evaluate a request. G1 always returns Allow (or Deny if `deny_all`
    /// is set); G2 will compile the request context against the active
    /// CEL policy bundle.
    pub fn evaluate(&self, _request: &PolicyRequest) -> PolicyDecision {
        if self.deny_all {
            PolicyDecision::Deny {
                reason: "deny-all is set (test mode)".to_string(),
            }
        } else {
            PolicyDecision::Allow
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req() -> PolicyRequest {
        PolicyRequest {
            subject: Subject { id: "u".to_string(), role: "admin".to_string() },
            action: "graph.mutate".to_string(),
            context: serde_json::json!({}),
        }
    }

    #[test]
    fn default_engine_allows() {
        let e = PolicyEngine::new();
        assert_eq!(e.evaluate(&req()), PolicyDecision::Allow);
    }

    #[test]
    fn deny_all_engine_denies() {
        let e = PolicyEngine::deny_all();
        assert!(matches!(e.evaluate(&req()), PolicyDecision::Deny { .. }));
    }
}
