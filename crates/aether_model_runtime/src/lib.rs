// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! A real, bounded model-inference runtime.
//!
//! A model is only usable inside its declared validity envelope, with
//! current evidence, and after an authorized promotion. This crate does
//! not train models. It loads a pinned model artifact, checks the request
//! against the model card's envelope, runs inference, and returns a
//! prediction with an explicit out-of-distribution verdict and a fallback
//! instruction.
//!
//! The inference itself is deliberately simple and deterministic: a linear
//! model over declared features, which is enough to prove the governance
//! path end to end. A production site swaps the `Infer` implementation for
//! ONNX or a Python runtime behind the same interface.

#![deny(unsafe_code)]
#![warn(missing_docs)]

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use thiserror::Error;

/// A model's declared lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelState {
    /// Being developed.
    Research,
    /// Benchmarked but not approved.
    Benchmarked,
    /// Independently validated.
    IndependentlyValidated,
    /// Approved for bounded decision support.
    Approved,
    /// Withdrawn.
    Retired,
}

/// A model card, trimmed to what the runtime needs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelCard {
    /// Model id.
    pub id: String,
    /// Semantic version.
    pub version: String,
    /// Task.
    pub task: String,
    /// State.
    pub state: ModelState,
    /// Intended use.
    pub intended_use: Vec<String>,
    /// Prohibited use.
    #[serde(default)]
    pub prohibited_use: Vec<String>,
    /// Validity envelope: parameter -> allowed range as strings.
    #[serde(default)]
    pub validity_envelope: BTreeMap<String, String>,
    /// Required feature names, in order.
    #[serde(default)]
    pub features: Vec<String>,
    /// Linear coefficients aligned with `features`.
    #[serde(default)]
    pub coefficients: Vec<f64>,
    /// Intercept.
    #[serde(default)]
    pub intercept: f64,
    /// Hash of the model artifact.
    pub artifact_hash: String,
}

/// Inference errors.
#[derive(Debug, Error, PartialEq)]
pub enum RuntimeError {
    /// The model is not approved for use.
    #[error("model {id} is {state:?}, not approved")]
    NotApproved {
        /// Model id.
        id: String,
        /// State.
        state: ModelState,
    },
    /// A request lies outside the declared envelope.
    #[error("input {feature} value {value} is outside the declared envelope")]
    OutsideEnvelope {
        /// Feature.
        feature: String,
        /// Value.
        value: f64,
    },
    /// A required feature is missing.
    #[error("missing feature {0}")]
    MissingFeature(String),
    /// The feature and coefficient counts disagree.
    #[error("model has {features} features but {coefficients} coefficients")]
    ShapeMismatch {
        /// Feature count.
        features: usize,
        /// Coefficient count.
        coefficients: usize,
    },
    /// The artifact hash does not match.
    #[error("artifact hash mismatch: expected {expected}, got {actual}")]
    ArtifactMismatch {
        /// Expected hash.
        expected: String,
        /// Actual hash.
        actual: String,
    },
}

/// A prediction with its governance verdict.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Prediction {
    /// Model id.
    pub model_id: String,
    /// Model version.
    pub version: String,
    /// Predicted value.
    pub value: f64,
    /// Whether the request was inside the envelope.
    pub in_domain: bool,
    /// Features that fell outside the envelope.
    pub out_of_domain: Vec<String>,
    /// Fallback instruction when out of domain.
    pub fallback: Option<String>,
    /// Model artifact hash.
    pub artifact_hash: String,
}

/// Parse a range like `"1 .. 100"` or `"-55 .. 90"`.
pub fn parse_range(spec: &str) -> Option<(f64, f64)> {
    let parts: Vec<&str> = spec.split("..").collect();
    if parts.len() != 2 {
        return None;
    }
    let low = parts[0].trim().parse().ok()?;
    let high = parts[1].trim().parse().ok()?;
    Some((low, high))
}

/// Run inference for a model card against a feature map.
///
/// The model must be approved. Requests outside the envelope return a
/// prediction with `in_domain: false` and a fallback instruction rather
/// than an extrapolated number.
pub fn infer(
    card: &ModelCard,
    features: &BTreeMap<String, f64>,
    artifact_hash: &str,
) -> Result<Prediction, RuntimeError> {
    if card.state != ModelState::Approved {
        return Err(RuntimeError::NotApproved {
            id: card.id.clone(),
            state: card.state,
        });
    }
    if card.artifact_hash != artifact_hash {
        return Err(RuntimeError::ArtifactMismatch {
            expected: card.artifact_hash.clone(),
            actual: artifact_hash.to_string(),
        });
    }
    if card.features.len() != card.coefficients.len() {
        return Err(RuntimeError::ShapeMismatch {
            features: card.features.len(),
            coefficients: card.coefficients.len(),
        });
    }

    let mut out_of_domain = Vec::new();
    for (name, spec) in &card.validity_envelope {
        if let Some(value) = features.get(name) {
            if let Some((low, high)) = parse_range(spec) {
                if *value < low || *value > high {
                    out_of_domain.push(name.clone());
                }
            }
        }
    }
    // Out-of-domain requests are not an error: they return a prediction
    // flagged `in_domain: false` with a fallback instruction. The caller
    // must not use the number for a controlled decision.

    let mut value = card.intercept;
    for (name, coefficient) in card.features.iter().zip(card.coefficients.iter()) {
        let feature = features
            .get(name)
            .ok_or_else(|| RuntimeError::MissingFeature(name.clone()))?;
        value += coefficient * feature;
    }

    let in_domain = out_of_domain.is_empty();
    Ok(Prediction {
        model_id: card.id.clone(),
        version: card.version.clone(),
        value,
        in_domain,
        out_of_domain,
        fallback: if in_domain {
            None
        } else {
            Some("escalate to a validated high-fidelity solver".to_string())
        },
        artifact_hash: card.artifact_hash.clone(),
    })
}

/// Canonical hash of a model card's numeric content.
pub fn card_hash(card: &ModelCard) -> String {
    let canonical = serde_json::json!({
        "id": card.id,
        "version": card.version,
        "features": card.features,
        "coefficients": card.coefficients,
        "intercept": card.intercept,
    });
    let bytes = serde_json::to_vec(&canonical).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

/// A JSON rendering of a prediction.
pub fn prediction_to_value(prediction: &Prediction) -> Value {
    serde_json::to_value(prediction).unwrap_or(Value::Null)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card() -> ModelCard {
        ModelCard {
            id: "surrogate-wingbox".to_string(),
            version: "0.2.0".to_string(),
            task: "reduced_order_structural_response".to_string(),
            state: ModelState::Approved,
            intended_use: vec!["screening".to_string()],
            prohibited_use: vec!["certification".to_string()],
            validity_envelope: BTreeMap::from([
                ("span_m".to_string(), "0.5 .. 12.0".to_string()),
                ("tip_load_kN".to_string(), "1 .. 100".to_string()),
            ]),
            features: vec!["span_m".to_string(), "tip_load_kN".to_string()],
            coefficients: vec![0.01, 0.005],
            intercept: 0.1,
            artifact_hash: "a".repeat(64),
        }
    }

    fn features() -> BTreeMap<String, f64> {
        BTreeMap::from([
            ("span_m".to_string(), 8.0),
            ("tip_load_kN".to_string(), 40.0),
        ])
    }

    #[test]
    fn approved_model_in_envelope_predicts() {
        let prediction = infer(&card(), &features(), &"a".repeat(64)).unwrap();
        assert!(prediction.in_domain);
        assert!(prediction.fallback.is_none());
        assert!((prediction.value - (0.1 + 0.01 * 8.0 + 0.005 * 40.0)).abs() < 1e-9);
    }

    #[test]
    fn unapproved_model_is_refused() {
        let mut card = card();
        card.state = ModelState::Benchmarked;
        let err = infer(&card, &features(), &"a".repeat(64)).unwrap_err();
        assert!(matches!(err, RuntimeError::NotApproved { .. }));
    }

    #[test]
    fn out_of_envelope_is_flagged_not_used() {
        let mut f = features();
        f.insert("tip_load_kN".to_string(), 500.0);
        let prediction = infer(&card(), &f, &"a".repeat(64)).unwrap();
        assert!(!prediction.in_domain);
        assert_eq!(prediction.out_of_domain, vec!["tip_load_kN"]);
        assert!(prediction.fallback.is_some());
    }

    #[test]
    fn artifact_hash_must_match() {
        let err = infer(&card(), &features(), &"b".repeat(64)).unwrap_err();
        assert!(matches!(err, RuntimeError::ArtifactMismatch { .. }));
    }

    #[test]
    fn missing_feature_is_an_error() {
        let f = BTreeMap::from([("span_m".to_string(), 8.0)]);
        let err = infer(&card(), &f, &"a".repeat(64)).unwrap_err();
        assert!(matches!(err, RuntimeError::MissingFeature(_)));
    }

    #[test]
    fn range_parsing_handles_negatives() {
        assert_eq!(parse_range("-55 .. 90"), Some((-55.0, 90.0)));
        assert_eq!(parse_range("nonsense"), None);
    }

    #[test]
    fn card_hash_is_stable() {
        let card = card();
        assert_eq!(card_hash(&card), card_hash(&card));
    }
}