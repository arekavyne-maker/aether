// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! Structural genome registry (spec §4.5).
//!
//! A structural genome is a typed, multi-scale asset linking constituent →
//! microstructure → mesostructure → coupon → element → component → assembly
//! → aircraft-level load path. Every property carries source, test method,
//! condition, statistical basis and rights metadata. Nominal properties,
//! distributions, temperature/rate dependence and process dependence are
//! kept distinct.

#![deny(unsafe_code)]
#![warn(missing_docs)]

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

/// The multi-scale level of a genome entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScaleLevel {
    Constituent,
    Microstructure,
    Mesostructure,
    Coupon,
    Element,
    Component,
    Assembly,
    LoadPath,
}

/// Statistical basis of a property.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Basis {
    Nominal,
    Distribution,
    TemperatureDependent,
    RateDependent,
    ProcessDependent,
    Environmental,
}

/// A property with provenance and statistical basis.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GenomeProperty {
    /// Property name.
    pub name: String,
    /// Numeric value.
    pub value: f64,
    /// Unit.
    pub unit: String,
    /// Statistical basis.
    pub basis: Basis,
    /// Source document.
    pub source: String,
    /// Test method.
    pub test_method: String,
    /// Condition (temperature, rate, environment).
    pub condition: String,
    /// Rights statement.
    pub rights: String,
}

/// A node in the structural genome.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GenomeNode {
    /// Stable id.
    pub id: String,
    /// Scale level.
    pub level: ScaleLevel,
    /// Parent node id, if any.
    pub parent: Option<String>,
    /// Properties.
    pub properties: Vec<GenomeProperty>,
    /// Evidence ids.
    pub evidence: Vec<String>,
}

/// Genome errors.
#[derive(Debug, Error, PartialEq)]
pub enum GenomeError {
    /// A node is malformed or duplicated.
    #[error("invalid genome: {0}")]
    Invalid(String),
    /// A link crosses scales illegally.
    #[error("invalid scale link: {0} -> {1}")]
    InvalidLink(String, String),
}

/// The structural genome registry.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct GenomeRegistry {
    /// Nodes by id.
    pub nodes: BTreeMap<String, GenomeNode>,
}

impl GenomeRegistry {
    /// Empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a node.
    pub fn add(&mut self, node: GenomeNode) -> Result<(), GenomeError> {
        if node.id.trim().is_empty() {
            return Err(GenomeError::Invalid("empty id".into()));
        }
        if self.nodes.contains_key(&node.id) {
            return Err(GenomeError::Invalid(format!("duplicate {}", node.id)));
        }
        if let Some(parent) = &node.parent {
            let parent_node = self
                .nodes
                .get(parent)
                .ok_or_else(|| GenomeError::Invalid(format!("missing parent {parent}")))?;
            if parent_node.level >= node.level {
                return Err(GenomeError::InvalidLink(
                    parent_node.level.as_str_label().to_string(),
                    node.level.as_str_label().to_string(),
                ));
            }
        }
        self.nodes.insert(node.id.clone(), node);
        Ok(())
    }

    /// Nodes at a given scale.
    pub fn at_level(&self, level: ScaleLevel) -> Vec<&GenomeNode> {
        self.nodes.values().filter(|n| n.level == level).collect()
    }

    /// Trace a node up to the load path.
    pub fn lineage(&self, id: &str) -> Vec<&GenomeNode> {
        let mut out = Vec::new();
        let mut current = self.nodes.get(id);
        while let Some(node) = current {
            out.push(node);
            current = node.parent.as_ref().and_then(|p| self.nodes.get(p));
        }
        out
    }
}

impl ScaleLevel {
    fn as_str_label(self) -> &'static str {
        match self {
            ScaleLevel::Constituent => "constituent",
            ScaleLevel::Microstructure => "microstructure",
            ScaleLevel::Mesostructure => "mesostructure",
            ScaleLevel::Coupon => "coupon",
            ScaleLevel::Element => "element",
            ScaleLevel::Component => "component",
            ScaleLevel::Assembly => "assembly",
            ScaleLevel::LoadPath => "load_path",
        }
    }
}

/// Hash a genome node.
pub fn genome_hash(node: &GenomeNode) -> String {
    let bytes = serde_json::to_vec(node).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prop(basis: Basis) -> GenomeProperty {
        GenomeProperty {
            name: "E".to_string(), value: 140e9, unit: "Pa".to_string(), basis,
            source: "MMPDS".to_string(), test_method: "ASTM D3039".to_string(),
            condition: "23C".to_string(), rights: "internal".to_string(),
        }
    }

    #[test]
    fn multi_scale_links_are_accepted() {
        let mut registry = GenomeRegistry::new();
        registry.add(GenomeNode {
            id: "fiber".into(), level: ScaleLevel::Constituent, parent: None,
            properties: vec![prop(Basis::Nominal)], evidence: vec![],
        }).unwrap();
        registry.add(GenomeNode {
            id: "laminate".into(), level: ScaleLevel::Coupon,
            parent: Some("fiber".into()), properties: vec![prop(Basis::Distribution)],
            evidence: vec![],
        }).unwrap();
        assert_eq!(registry.lineage("laminate").len(), 2);
    }

    #[test]
    fn reverse_scale_link_is_rejected() {
        let mut registry = GenomeRegistry::new();
        registry.add(GenomeNode {
            id: "component".into(), level: ScaleLevel::Component, parent: None,
            properties: vec![], evidence: vec![],
        }).unwrap();
        let err = registry.add(GenomeNode {
            id: "coupon".into(), level: ScaleLevel::Coupon,
            parent: Some("component".into()), properties: vec![], evidence: vec![],
        }).unwrap_err();
        assert!(matches!(err, GenomeError::InvalidLink(_, _)));
    }

    #[test]
    fn basis_types_are_distinct() {
        assert_ne!(Basis::Nominal, Basis::TemperatureDependent);
    }
}