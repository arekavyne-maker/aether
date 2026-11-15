// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! Rust implementation of the Aether contract validation harness.
//!
//! This is the second half of the two-implementation rule (ADR-0022).
//! The Python implementation in `aether_sdk/validation.py` is the
//! first. Both implementations MUST accept or reject the same set of
//! documents for every contract (MVR-01/02/06/07).
//!
//! The validation harness operates on JSON Schema 2020-12 documents and
//! is otherwise agnostic about the contract semantics.

#![deny(unsafe_code)]
#![warn(missing_docs)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use jsonschema::{Retrieve, Uri, Validator};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use walkdir::WalkDir;

/// A retriever that resolves `$id` URLs to local schema files.
#[derive(Debug, Default)]
struct LocalRetriever {
    by_id: BTreeMap<String, PathBuf>,
}

impl Retrieve for LocalRetriever {
    fn retrieve(
        &self,
        uri: &Uri<String>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let url = uri.as_str().to_string();
        if let Some(path) = self.by_id.get(&url) {
            let raw = std::fs::read_to_string(path)
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
            let v: Value = serde_json::from_str(&raw)
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
            return Ok(v);
        }
        Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("no local schema for {url}"),
        )))
    }
}

/// All known contract roots in the monorepo. Phase 0/G1: five v0
/// contracts. G3: ten. G4: thirteen. G5: sixteen. G6: nineteen.
pub const CONTRACT_ROOTS: &[&str] = &[
    "contracts/plugin.manifest",
    "contracts/physics.solver",
    "contracts/compute.backend",
    "contracts/io.format",
    "contracts/graph.model",
    "contracts/geometry.kernel",
    "contracts/geometry.recipe",
    "contracts/mesh.engine",
    "contracts/field.model",
    "contracts/requirement",
    "contracts/physics.composable",
    "contracts/uq.method",
    "contracts/si.governor",
    "contracts/optimizer",
    "contracts/twin.estimator",
    "contracts/surrogate",
    "contracts/manufacturing.process",
    "contracts/inspection.device",
    "contracts/test.rig",
];

/// Result of validating one document against one schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub contract: String,
    pub schema_path: String,
    pub document_path: String,
    pub valid: bool,
    pub errors: Vec<String>,
}

/// The harness state.
pub struct Harness {
    /// Map of (contract id, schema file basename) -> compiled schema.
    compiled: BTreeMap<(String, String), Validator>,
    /// Map of (contract id) -> (schema id, schema path) for every
    /// declared schema. We use these to resolve a schema id to a path.
    schema_index: BTreeMap<String, Vec<SchemaEntry>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaEntry {
    pub schema_id: String,
    pub schema_path: String,
}

impl Harness {
    /// Build a harness from a monorepo root. Discovers every contract's
    /// `manifest.json` and compiles every declared `*.schema.json`.
    pub fn from_monorepo(monorepo: &Path) -> Result<Self> {
        let mut compiled: BTreeMap<(String, String), Validator> = BTreeMap::new();
        let mut schema_index: BTreeMap<String, Vec<SchemaEntry>> = BTreeMap::new();
        let mut by_id: BTreeMap<String, PathBuf> = BTreeMap::new();
        for root_rel in CONTRACT_ROOTS {
            let root = monorepo.join(root_rel);
            if !root.is_dir() {
                continue;
            }
            let manifest_path = root.join("manifest.json");
            if !manifest_path.is_file() {
                continue;
            }
            let raw = std::fs::read_to_string(&manifest_path)
                .with_context(|| format!("read {}", manifest_path.display()))?;
            let manifest: serde_json::Value = serde_json::from_str(&raw)
                .with_context(|| format!("parse {}", manifest_path.display()))?;
            let contract_id = manifest
                .get("contract_id")
                .and_then(|v| v.as_str())
                .unwrap_or(root_rel)
                .to_string();
            for entry in manifest.get("schemas").and_then(|v| v.as_array()).cloned().unwrap_or_default() {
                let schema_path_rel = entry
                    .get("path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("schema entry missing path"))?;
                let schema_abs = root.join(schema_path_rel);
                let schema_raw = std::fs::read_to_string(&schema_abs)
                    .with_context(|| format!("read {}", schema_abs.display()))?;
                let schema_value: serde_json::Value = serde_json::from_str(&schema_raw)
                    .with_context(|| format!("parse {}", schema_abs.display()))?;
                let schema_id = schema_value
                    .get("$id")
                    .and_then(|v| v.as_str())
                    .unwrap_or(schema_path_rel)
                    .to_string();
                by_id.insert(schema_id.clone(), schema_abs.clone());
                // Also index by basename for relative `$ref` resolution.
                if let Some(basename) = schema_path_rel.rsplit('/').next() {
                    by_id.insert(basename.to_string(), schema_abs.clone());
                }
                let retriever = LocalRetriever { by_id: by_id.clone() };
                let compiled_schema = jsonschema::options()
                    .with_retriever(retriever)
                    .build(&schema_value)
                    .with_context(|| format!("compile schema {}", schema_abs.display()))?;
                compiled.insert(
                    (contract_id.clone(), schema_path_rel.to_string()),
                    compiled_schema,
                );
                schema_index
                    .entry(contract_id.clone())
                    .or_default()
                    .push(SchemaEntry {
                        schema_id: schema_id.clone(),
                        schema_path: schema_path_rel.to_string(),
                    });
            }
        }
        Ok(Self {
            compiled,
            schema_index,
        })
    }

    /// Look up a compiled schema by contract id and schema id (which is
    /// typically the `$id` of the JSON Schema document).
    pub fn compiled(&self, contract: &str, schema_id: &str) -> Option<&Validator> {
        if let Some(entries) = self.schema_index.get(contract) {
            for entry in entries {
                if entry.schema_id == schema_id {
                    return self.compiled.get(&(contract.to_string(), entry.schema_path.clone()));
                }
            }
        }
        None
    }

    /// Validate a JSON value against a schema by id.
    pub fn validate(
        &self,
        contract: &str,
        schema_id: &str,
        document: &serde_json::Value,
    ) -> Result<ValidationResult> {
        let schema = self
            .compiled(contract, schema_id)
            .ok_or_else(|| anyhow!("schema {schema_id} not found in contract {contract}"))?;
        let result = schema.validate(document);
        let mut errors = Vec::new();
        if let Err(err) = result {
            errors.push(err.to_string());
        }
        Ok(ValidationResult {
            contract: contract.to_string(),
            schema_path: schema_id.to_string(),
            document_path: "<memory>".to_string(),
            valid: errors.is_empty(),
            errors,
        })
    }

    /// Return the registered contract id for a given leaf name, if any.
    pub fn full_contract_id(&self, leaf: &str) -> Option<String> {
        for key in self.schema_index.keys() {
            if let Some(suffix) = key.strip_prefix("aether.") {
                if suffix == leaf {
                    return Some(key.clone());
                }
            }
        }
        None
    }

    /// Validate every example document in a contract's `examples/`
    /// directory against its declared schema. Returns a list of
    /// `ValidationResult` per (schema, example) pair.
    pub fn validate_examples(&self, monorepo: &Path) -> Vec<ValidationResult> {
        let mut out = Vec::new();
        for root_rel in CONTRACT_ROOTS {
            // Match the contract id actually used in `from_monorepo`,
            // which is the `contract_id` field in `manifest.json`. For
            // robustness, we try both the leaf name and the `aether.*`
            // id when looking up schemas.
            let contract_leaf = root_rel.split('/').last().unwrap_or(root_rel).to_string();
            let contract_full = self.full_contract_id(&contract_leaf);
            let contract = contract_full.unwrap_or(contract_leaf);
            let examples = monorepo.join(root_rel).join("examples");
            if !examples.is_dir() {
                continue;
            }
            for entry in WalkDir::new(&examples).max_depth(1) {
                let Ok(entry) = entry else { continue };
                if !entry.file_type().is_file() {
                    continue;
                }
                let path = entry.path();
                let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
                if ext != "yaml" && ext != "yml" && ext != "json" {
                    continue;
                }
                let raw = match std::fs::read_to_string(path) {
                    Ok(r) => r,
                    Err(_) => continue,
                };
                let value: serde_json::Value = if ext == "json" {
                    match serde_json::from_str(&raw) {
                        Ok(v) => v,
                        Err(_) => continue,
                    }
                } else {
                    match serde_yaml::from_str(&raw) {
                        Ok(v) => v,
                        Err(_) => continue,
                    }
                };
                // The example must declare its schema via a
                // `_schema_id` field (a contract validation convention).
                if let Some(schema_id) = value.get("_schema_id").and_then(|v| v.as_str()) {
                    // Strip the harness-only `_schema_id` field before
                    // validating against the contract schema.
                    let mut cleaned = value.as_object().cloned().unwrap_or_default();
                    cleaned.remove("_schema_id");
                    let cleaned_value = serde_json::Value::Object(cleaned);
                    if let Ok(r) = self.validate(&contract, schema_id, &cleaned_value) {
                        let mut r = r;
                        r.document_path = path.display().to_string();
                        out.push(r);
                    }
                } else {
                    // Auto-discover: try the single schema for the
                    // contract (if there's exactly one).
                    let schemas = self
                        .schema_index
                        .get(&contract)
                        .map(|v| v.as_slice())
                        .unwrap_or(&[]);
                    if schemas.len() == 1 {
                        if let Ok(r) = self.validate(&contract, &schemas[0].schema_id, &value) {
                            let mut r = r;
                            r.document_path = path.display().to_string();
                            out.push(r);
                        }
                    }
                }
            }
        }
        out
    }
}

    /// Find the monorepo root by walking up from the current directory.
    pub fn find_monorepo_root() -> Result<PathBuf> {
        let mut here = std::env::current_dir().context("current_dir")?;
        loop {
            if here.join("contracts").is_dir() && here.join("Cargo.toml").is_file() {
                return Ok(here);
            }
            if !here.pop() {
                return Err(anyhow!("monorepo root not found from {}", here.display()));
            }
        }
    }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn harness_compiles_all_contracts() {
        let root = find_monorepo_root().unwrap();
        let harness = Harness::from_monorepo(&root).unwrap();
        // Five contracts, each with at least one schema.
        let contracts: Vec<&str> = harness.schema_index.keys().map(|s| s.as_str()).collect();
        assert!(contracts.contains(&"aether.plugin.manifest"));
        assert!(contracts.contains(&"aether.physics.solver"));
        assert!(contracts.contains(&"aether.compute.backend"));
        assert!(contracts.contains(&"aether.io.format"));
        assert!(contracts.contains(&"aether.graph.model"));
    }

    #[test]
    fn i6_resource_requirement_rejects_gpu_model() {
        let root = find_monorepo_root().unwrap();
        let harness = Harness::from_monorepo(&root).unwrap();
        let bad = serde_json::json!({
            "memory_gb": 8,
            "cores": 4,
            "parallelism": "multi_process",
            "accelerator": "optional",
            "accelerator_class": "any",
            "architecture": ["x86_64"],
            "gpu_model": "H100"
        });
        let r = harness
            .validate(
                "aether.compute.backend",
                "https://aether.local/contracts/compute.backend/schema/resource-requirement.schema.json",
                &bad,
            )
            .unwrap();
        assert!(!r.valid, "I-6 violation: hardware-specific field must be rejected");
    }

    #[test]
    fn i7_result_requires_fidelity_metadata() {
        let root = find_monorepo_root().unwrap();
        let harness = Harness::from_monorepo(&root).unwrap();
        let bad = serde_json::json!({
            "run_id": "01900000-0000-0000-0000-000000000000",
            "produced_at": "2026-08-30T00:00:00Z",
            "outputs": {"scalars": [], "fields": []},
            "provenance_ref": "sha256:abc"
        });
        let r = harness
            .validate(
                "aether.physics.solver",
                "https://aether.local/contracts/physics.solver/schema/result-bundle.schema.json",
                &bad,
            )
            .unwrap();
        assert!(!r.valid, "I-7: result bundle missing fidelity_class/validation_status/uncertainty must be rejected");
    }
}
