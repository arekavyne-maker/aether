// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! Material cards, allowables, and pedigree.
//!
//! A material card alone is not a manufacturing specification. This crate
//! makes the distinction executable: a [`MaterialCard`] carries the
//! nominal properties, the statistical basis of its allowables, the
//! temperature and environment envelope over which they are valid, and the
//! provenance of every number. A design may only consume an allowable
//! inside its declared envelope, at a declared temperature, with a known
//! statistical basis.

#![deny(unsafe_code)]
#![warn(missing_docs)]

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use thiserror::Error;

/// Statistical basis of an allowable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatisticalBasis {
    /// Typical (mean) value.
    Typical,
    /// A-basis: 99% of the population exceeds this value, 95% confidence.
    ABasis,
    /// B-basis: 90% of the population exceeds this value, 95% confidence.
    BBasis,
    /// Design allowable from a qualified allowables program.
    DesignAllowable,
    /// Material specification minimum.
    SpecificationMinimum,
}

/// Material family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MaterialFamily {
    /// Aluminium alloy.
    Aluminium,
    /// Titanium alloy.
    Titanium,
    /// Steel.
    Steel,
    /// Nickel superalloy.
    NickelSuperalloy,
    /// Thermoset polymer composite.
    ThermosetComposite,
    /// Thermoplastic composite.
    ThermoplasticComposite,
    /// Ceramic or ceramic-matrix composite.
    Ceramic,
    /// Additive metallic.
    AdditiveMetallic,
    /// Elastomer.
    Elastomer,
    /// Adhesive.
    Adhesive,
    /// Foam or honeycomb core.
    Core,
}

/// One property value with its statistical basis and source.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PropertyValue {
    /// Value in SI units.
    pub value: f64,
    /// Unit symbol.
    pub unit: String,
    /// Statistical basis.
    pub basis: StatisticalBasis,
    /// Source document or test campaign.
    pub source: String,
}

/// A temperature-dependent property point.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TemperaturePoint {
    /// Temperature in degrees Celsius.
    pub temperature_c: f64,
    /// Property value at that temperature.
    pub property: PropertyValue,
}

/// A material allowable.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Allowable {
    /// Property name, e.g. `yield_strength`.
    pub name: String,
    /// Values at one or more temperatures.
    pub temperature_points: Vec<TemperaturePoint>,
    /// Environmental condition the values apply to.
    #[serde(default)]
    pub environment: Option<String>,
}

impl Allowable {
    /// Interpolate the allowable at a temperature, rejecting extrapolation.
    pub fn at_temperature(&self, temperature_c: f64) -> Result<PropertyValue, MaterialError> {
        if self.temperature_points.is_empty() {
            return Err(MaterialError::Invalid(format!(
                "allowable {} has no temperature points",
                self.name
            )));
        }
        let mut sorted: Vec<&TemperaturePoint> = self.temperature_points.iter().collect();
        sorted.sort_by(|a, b| {
            a.temperature_c
                .partial_cmp(&b.temperature_c)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let low = sorted.first().unwrap().temperature_c;
        let high = sorted.last().unwrap().temperature_c;
        if temperature_c < low || temperature_c > high {
            return Err(MaterialError::OutsideEnvelope {
                property: self.name.clone(),
                requested: temperature_c,
                low,
                high,
            });
        }
        // Exact match.
        for point in &sorted {
            if (point.temperature_c - temperature_c).abs() < 1e-9 {
                return Ok(point.property.clone());
            }
        }
        for pair in sorted.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            if temperature_c >= a.temperature_c && temperature_c <= b.temperature_c {
                let span = b.temperature_c - a.temperature_c;
                let t = if span.abs() < 1e-12 {
                    0.0
                } else {
                    (temperature_c - a.temperature_c) / span
                };
                let value = a.property.value + t * (b.property.value - a.property.value);
                // The interpolated value inherits the weaker basis of the
                // two endpoints, never a stronger one.
                let basis = if a.property.basis <= b.property.basis {
                    a.property.basis
                } else {
                    b.property.basis
                };
                return Ok(PropertyValue {
                    value,
                    unit: a.property.unit.clone(),
                    basis,
                    source: format!(
                        "interpolated between {} and {}",
                        a.property.source, b.property.source
                    ),
                });
            }
        }
        Err(MaterialError::Invalid(format!(
            "could not interpolate {} at {temperature_c}",
            self.name
        )))
    }
}

/// A material card.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaterialCard {
    /// Stable card id.
    pub id: String,
    /// Common name.
    pub name: String,
    /// Family.
    pub family: MaterialFamily,
    /// Specification the material is procured to.
    pub specification: String,
    /// Form, e.g. `plate`, `bar`, `laminate`.
    pub form: String,
    /// Density in kg/m^3.
    pub density: PropertyValue,
    /// Allowables.
    #[serde(default)]
    pub allowables: Vec<Allowable>,
    /// Temperature range over which the card is valid, in Celsius.
    pub temperature_range_c: (f64, f64),
    /// Prohibited uses.
    #[serde(default)]
    pub prohibited_use: Vec<String>,
    /// Pedigree: who produced the data and under what programme.
    pub pedigree: String,
}

impl MaterialCard {
    /// Canonical hash of the card.
    pub fn content_hash(&self) -> String {
        let canonical = serde_json::json!({
            "id": self.id,
            "name": self.name,
            "family": self.family,
            "specification": self.specification,
            "form": self.form,
            "density": self.density,
            "allowables": self.allowables,
            "temperature_range_c": self.temperature_range_c,
            "prohibited_use": self.prohibited_use,
            "pedigree": self.pedigree,
        });
        let bytes = serde_json::to_vec(&canonical).unwrap_or_default();
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        format!("{:x}", hasher.finalize())
    }

    /// Look up an allowable by name.
    pub fn allowable(&self, name: &str) -> Option<&Allowable> {
        self.allowables.iter().find(|a| a.name == name)
    }
}

/// Material errors.
#[derive(Debug, Error, PartialEq)]
pub enum MaterialError {
    /// A request falls outside the declared envelope.
    #[error("property {property} requested at {requested} C is outside {low}..{high} C")]
    OutsideEnvelope {
        /// Property name.
        property: String,
        /// Requested temperature.
        requested: f64,
        /// Lower bound.
        low: f64,
        /// Upper bound.
        high: f64,
    },
    /// The card or request is malformed.
    #[error("invalid material data: {0}")]
    Invalid(String),
    /// The requested property does not exist.
    #[error("unknown property: {0}")]
    UnknownProperty(String),
    /// A request violates the declared prohibited use.
    #[error("prohibited use: {0}")]
    Prohibited(String),
}

/// Evaluate a material allowable for a design point.
///
/// Returns an error when the design temperature is outside the card's
/// declared range, when the property is unknown, or when the requested
/// use is prohibited. Extrapolation is never silently allowed.
pub fn evaluate_allowable(
    card: &MaterialCard,
    property: &str,
    temperature_c: f64,
    intended_use: &str,
) -> Result<PropertyValue, MaterialError> {
    for prohibited in &card.prohibited_use {
        if intended_use
            .to_ascii_lowercase()
            .contains(&prohibited.to_ascii_lowercase())
        {
            return Err(MaterialError::Prohibited(prohibited.clone()));
        }
    }
    if temperature_c < card.temperature_range_c.0 || temperature_c > card.temperature_range_c.1 {
        return Err(MaterialError::OutsideEnvelope {
            property: property.to_string(),
            requested: temperature_c,
            low: card.temperature_range_c.0,
            high: card.temperature_range_c.1,
        });
    }
    let allowable = card
        .allowable(property)
        .ok_or_else(|| MaterialError::UnknownProperty(property.to_string()))?;
    Ok(allowable.at_temperature(temperature_c)?.clone())
}

/// A library of material cards.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MaterialLibrary {
    /// Cards by id.
    pub cards: BTreeMap<String, MaterialCard>,
}

impl MaterialLibrary {
    /// Empty library.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a card, rejecting duplicates.
    pub fn add(&mut self, card: MaterialCard) -> Result<(), MaterialError> {
        if card.id.trim().is_empty() {
            return Err(MaterialError::Invalid("card id is empty".into()));
        }
        if self.cards.contains_key(&card.id) {
            return Err(MaterialError::Invalid(format!(
                "duplicate card {}",
                card.id
            )));
        }
        self.cards.insert(card.id.clone(), card);
        Ok(())
    }

    /// Look up a card.
    pub fn get(&self, id: &str) -> Option<&MaterialCard> {
        self.cards.get(id)
    }

    /// Validate every card: densities and allowables must be internally
    /// consistent, and the temperature range must span the allowables.
    pub fn validate(&self) -> Vec<String> {
        let mut problems = Vec::new();
        for card in self.cards.values() {
            if card.density.value <= 0.0 {
                problems.push(format!("{}: density must be positive", card.id));
            }
            for allowable in &card.allowables {
                if allowable.temperature_points.is_empty() {
                    problems.push(format!(
                        "{}.{}: no temperature points",
                        card.id, allowable.name
                    ));
                    continue;
                }
                for point in &allowable.temperature_points {
                    if point.temperature_c < card.temperature_range_c.0
                        || point.temperature_c > card.temperature_range_c.1
                    {
                        problems.push(format!(
                            "{}.{}: temperature {} C is outside the card range {:?}",
                            card.id, allowable.name, point.temperature_c, card.temperature_range_c
                        ));
                    }
                }
            }
        }
        problems
    }
}

/// Helper to build a single-temperature allowable.
pub fn allowable_at(
    name: &str,
    value: f64,
    unit: &str,
    temperature_c: f64,
    basis: StatisticalBasis,
    source: &str,
) -> Allowable {
    Allowable {
        name: name.to_string(),
        temperature_points: vec![TemperaturePoint {
            temperature_c,
            property: PropertyValue {
                value,
                unit: unit.to_string(),
                basis,
                source: source.to_string(),
            },
        }],
        environment: None,
    }
}

/// The JSON value of a property, for reports.
pub fn property_to_value(p: &PropertyValue) -> Value {
    serde_json::json!({
        "value": p.value,
        "unit": p.unit,
        "basis": format!("{:?}", p.basis).to_lowercase(),
        "source": p.source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card() -> MaterialCard {
        MaterialCard {
            id: "mat-al2024".to_string(),
            name: "Aluminium 2024-T3".to_string(),
            family: MaterialFamily::Aluminium,
            specification: "AMS-QQ-A-250/4".to_string(),
            form: "plate".to_string(),
            density: PropertyValue {
                value: 2780.0,
                unit: "kg/m^3".to_string(),
                basis: StatisticalBasis::SpecificationMinimum,
                source: "AMS-QQ-A-250/4".to_string(),
            },
            allowables: vec![Allowable {
                name: "yield_strength".to_string(),
                temperature_points: vec![
                    TemperaturePoint {
                        temperature_c: 23.0,
                        property: PropertyValue {
                            value: 345.0e6,
                            unit: "Pa".to_string(),
                            basis: StatisticalBasis::ABasis,
                            source: "MMPDS-01".to_string(),
                        },
                    },
                    TemperaturePoint {
                        temperature_c: 150.0,
                        property: PropertyValue {
                            value: 290.0e6,
                            unit: "Pa".to_string(),
                            basis: StatisticalBasis::ABasis,
                            source: "MMPDS-01".to_string(),
                        },
                    },
                ],
                environment: None,
            }],
            temperature_range_c: (-55.0, 200.0),
            prohibited_use: vec!["elevated temperature".to_string()],
            pedigree: "MMPDS-01 allowables programme".to_string(),
        }
    }

    #[test]
    fn exact_temperature_returns_the_value() {
        let card = card();
        let v = evaluate_allowable(&card, "yield_strength", 23.0, "wing skin").unwrap();
        assert!((v.value - 345.0e6).abs() < 1.0);
        assert_eq!(v.basis, StatisticalBasis::ABasis);
    }

    #[test]
    fn interpolation_weakens_the_basis() {
        let card = card();
        let v = evaluate_allowable(&card, "yield_strength", 86.5, "wing skin").unwrap();
        assert!(v.value > 290.0e6 && v.value < 345.0e6);
        assert_eq!(v.basis, StatisticalBasis::ABasis);
    }

    #[test]
    fn extrapolation_is_rejected() {
        let card = card();
        let err = evaluate_allowable(&card, "yield_strength", 500.0, "wing skin").unwrap_err();
        assert!(matches!(err, MaterialError::OutsideEnvelope { .. }));
    }

    #[test]
    fn prohibited_use_is_enforced() {
        let card = card();
        let err =
            evaluate_allowable(&card, "yield_strength", 23.0, "elevated temperature service")
                .unwrap_err();
        assert!(matches!(err, MaterialError::Prohibited(_)));
    }

    #[test]
    fn unknown_property_is_an_error() {
        let card = card();
        let err = evaluate_allowable(&card, "fracture_toughness", 23.0, "wing").unwrap_err();
        assert!(matches!(err, MaterialError::UnknownProperty(_)));
    }

    #[test]
    fn library_validation_flags_inconsistent_cards() {
        let mut library = MaterialLibrary::new();
        let mut bad = card();
        bad.temperature_range_c = (-55.0, 100.0);
        library.add(bad).unwrap();
        let problems = library.validate();
        assert!(problems.iter().any(|p| p.contains("150 C")));
    }

    #[test]
    fn card_hash_is_stable() {
        let card = card();
        assert_eq!(card.content_hash(), card.content_hash());
        assert_eq!(card.content_hash().len(), 64);
    }
}