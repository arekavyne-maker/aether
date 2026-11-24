// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! Data classification and handling policy.
//!
//! Aether is dual-use. The same substrate carries a civilian UAV wing and a
//! controlled programme. Classification is therefore a first-class,
//! project-configurable property of artifacts and graph objects, and the
//! policy engine must be able to refuse an export or a cross-boundary read
//! based on it.
//!
//! This crate does not decide which legal regime applies. It records the
//! declared classification and enforces the handling rules the project
//! configured, and it refuses to guess when a classification is missing.

#![deny(unsafe_code)]
#![warn(missing_docs)]

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Data classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Classification {
    /// Public.
    Public,
    /// Internal use only.
    Internal,
    /// Confidential.
    Confidential,
    /// Restricted.
    Restricted,
    /// Export-controlled.
    ExportControlled,
}

impl Classification {
    /// Wire label.
    pub fn as_str(self) -> &'static str {
        match self {
            Classification::Public => "public",
            Classification::Internal => "internal",
            Classification::Confidential => "confidential",
            Classification::Restricted => "restricted",
            Classification::ExportControlled => "export_controlled",
        }
    }

    /// Parse a wire label. Unknown labels are an error, never a default.
    pub fn parse(label: &str) -> Option<Self> {
        Some(match label {
            "public" => Classification::Public,
            "internal" => Classification::Internal,
            "confidential" => Classification::Confidential,
            "restricted" => Classification::Restricted,
            "export_controlled" => Classification::ExportControlled,
            _ => return None,
        })
    }
}

/// A handling decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Handling {
    /// May be read.
    Read,
    /// May be exported from the project.
    Export,
    /// May be shared with a federated peer.
    Federate,
}

/// Policy errors.
#[derive(Debug, Error, PartialEq)]
pub enum ClassificationError {
    /// The classification label is unknown.
    #[error("unknown classification {0}")]
    Unknown(String),
    /// The object has no classification.
    #[error("object {0} has no classification")]
    Unclassified(String),
    /// The handling is not permitted.
    #[error("{classification:?} data may not be {handling:?}")]
    Denied {
        /// Classification.
        classification: Classification,
        /// Handling.
        handling: Handling,
    },
}

/// A project's handling policy: the maximum classification allowed for
/// each handling operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HandlingPolicy {
    /// Highest classification that may be read.
    pub max_read: Classification,
    /// Highest classification that may be exported.
    pub max_export: Classification,
    /// Highest classification that may be federated.
    pub max_federate: Classification,
}

impl Default for HandlingPolicy {
    fn default() -> Self {
        Self {
            max_read: Classification::Restricted,
            max_export: Classification::Internal,
            max_federate: Classification::Confidential,
        }
    }
}

impl HandlingPolicy {
    /// Check whether a classification permits a handling operation.
    ///
    /// An unclassified object is denied: the platform never guesses.
    pub fn permits(
        &self,
        classification: Option<Classification>,
        handling: Handling,
        object_id: &str,
    ) -> Result<(), ClassificationError> {
        let classification = classification.ok_or_else(|| {
            ClassificationError::Unclassified(object_id.to_string())
        })?;
        let limit = match handling {
            Handling::Read => self.max_read,
            Handling::Export => self.max_export,
            Handling::Federate => self.max_federate,
        };
        if classification <= limit {
            Ok(())
        } else {
            Err(ClassificationError::Denied {
                classification,
                handling,
            })
        }
    }
}

/// A classification label attached to an object.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Classified<T> {
    /// The object.
    pub value: T,
    /// Its classification.
    pub classification: Classification,
}

/// A registry of object classifications.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ClassificationRegistry {
    /// Object id -> classification.
    pub objects: BTreeMap<String, Classification>,
}

impl ClassificationRegistry {
    /// Empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Classify an object.
    pub fn classify(
        &mut self,
        object_id: &str,
        label: &str,
    ) -> Result<(), ClassificationError> {
        let classification = Classification::parse(label)
            .ok_or_else(|| ClassificationError::Unknown(label.to_string()))?;
        self.objects.insert(object_id.to_string(), classification);
        Ok(())
    }

    /// Look up an object's classification.
    pub fn get(&self, object_id: &str) -> Option<Classification> {
        self.objects.get(object_id).copied()
    }

    /// Check a handling operation against the policy.
    pub fn check(
        &self,
        policy: &HandlingPolicy,
        object_id: &str,
        handling: Handling,
    ) -> Result<(), ClassificationError> {
        policy.permits(self.get(object_id), handling, object_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_may_be_exported() {
        let policy = HandlingPolicy::default();
        assert!(policy.permits(Some(Classification::Public), Handling::Export, "x").is_ok());
    }

    #[test]
    fn export_controlled_may_not_be_exported() {
        let policy = HandlingPolicy::default();
        let err = policy
            .permits(Some(Classification::ExportControlled), Handling::Export, "x")
            .unwrap_err();
        assert!(matches!(err, ClassificationError::Denied { .. }));
    }

    #[test]
    fn unclassified_is_denied() {
        let policy = HandlingPolicy::default();
        let err = policy.permits(None, Handling::Read, "x").unwrap_err();
        assert!(matches!(err, ClassificationError::Unclassified(_)));
    }

    #[test]
    fn unknown_label_is_rejected() {
        let mut registry = ClassificationRegistry::new();
        assert!(registry.classify("x", "top-secret").is_err());
    }

    #[test]
    fn registry_enforces_policy() {
        let mut registry = ClassificationRegistry::new();
        registry.classify("public-doc", "public").unwrap();
        registry.classify("secret-doc", "restricted").unwrap();
        let policy = HandlingPolicy::default();
        assert!(registry.check(&policy, "public-doc", Handling::Export).is_ok());
        assert!(registry.check(&policy, "secret-doc", Handling::Export).is_err());
        // Restricted may still be read.
        assert!(registry.check(&policy, "secret-doc", Handling::Read).is_ok());
    }
}