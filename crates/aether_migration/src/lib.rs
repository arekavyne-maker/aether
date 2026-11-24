// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! Contract migration framework (ADR-0041).
//!
//! Migrations are ordered, deterministic functions over JSON values. Each
//! records its from/to version and is reversible where feasible. Applying a
//! migration is explicit; there is no implicit coercion of old documents.

#![deny(unsafe_code)]
#![warn(missing_docs)]

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

/// Migration errors.
#[derive(Debug, Error, PartialEq)]
pub enum MigrationError {
    /// No path exists between the versions.
    #[error("no migration from {from} to {to}")]
    NoPath { from: String, to: String },
    /// The document is malformed for the migration.
    #[error("invalid document for migration: {0}")]
    Invalid(String),
}

/// A migration step.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Migration {
    /// Source version.
    pub from: String,
    /// Target version.
    pub to: String,
    /// Human description.
    pub description: String,
}

impl Migration {
    /// Apply this migration to a JSON document.
    pub fn apply(&self, value: &Value) -> Result<Value, MigrationError> {
        let mut out = value.clone();
        let map = out
            .as_object_mut()
            .ok_or_else(|| MigrationError::Invalid("document is not an object".into()))?;
        // A migration records the version transition in provenance while
        // preserving all existing fields. Domain-specific migrations are
        // registered per contract; this generic step never drops data.
        let provenance = map
            .entry("migrations")
            .or_insert_with(|| Value::Array(Vec::new()));
        if let Some(list) = provenance.as_array_mut() {
            list.push(serde_json::json!({
                "from": self.from,
                "to": self.to,
            }));
        }
        map.insert("contract_version".to_string(), Value::String(self.to.clone()));
        Ok(out)
    }
}

/// A migration path between two versions.
pub fn migrate(
    value: &Value,
    steps: &[Migration],
    from: &str,
    to: &str,
) -> Result<Value, MigrationError> {
    let mut current = from.to_string();
    let mut document = value.clone();
    let mut guard = 0;
    while current != to {
        guard += 1;
        if guard > steps.len() + 1 {
            return Err(MigrationError::NoPath { from: from.to_string(), to: to.to_string() });
        }
        let step = steps
            .iter()
            .find(|s| s.from == current)
            .ok_or_else(|| MigrationError::NoPath {
                from: current.clone(),
                to: to.to_string(),
            })?;
        document = step.apply(&document)?;
        current = step.to.clone();
    }
    Ok(document)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn steps() -> Vec<Migration> {
        vec![
            Migration { from: "0.1.0".into(), to: "0.2.0".into(), description: "first".into() },
            Migration { from: "0.2.0".into(), to: "1.0.0".into(), description: "second".into() },
        ]
    }

    #[test]
    fn chained_migration_applies() {
        let value = serde_json::json!({"id": "x"});
        let out = migrate(&value, &steps(), "0.1.0", "1.0.0").unwrap();
        assert_eq!(out["contract_version"], "1.0.0");
        assert_eq!(out["migrations"].as_array().unwrap().len(), 2);
        assert_eq!(out["id"], "x");
    }

    #[test]
    fn missing_path_is_reported() {
        let value = serde_json::json!({"id": "x"});
        let err = migrate(&value, &steps(), "0.1.0", "2.0.0").unwrap_err();
        assert!(matches!(err, MigrationError::NoPath { .. }));
    }

    #[test]
    fn migration_preserves_data() {
        let value = serde_json::json!({"id": "x", "data": [1, 2, 3]});
        let out = migrate(&value, &steps(), "0.1.0", "0.2.0").unwrap();
        assert_eq!(out["data"], serde_json::json!([1, 2, 3]));
    }
}