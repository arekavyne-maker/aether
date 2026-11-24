// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! Flight-regime and environment envelopes as structured data.
//!
//! A load case is not free text. This crate represents the operational
//! envelope as machine-checkable dimensions: Mach and Reynolds ranges,
//! altitude, dynamic pressure, load factor, thermal and acoustic
//! environments, icing and corrosion exposure, and mission duty cycle.
//!
//! The envelope is a *configuration-controlled input* to analysis. It does
//! not determine which regulations apply; that is a separate compliance
//! mapping.

#![deny(unsafe_code)]
#![warn(missing_docs)]

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use thiserror::Error;

/// Flight regime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlightRegime {
    /// Low-speed / high-lift.
    LowSpeedHighLift,
    /// Subsonic transport.
    Subsonic,
    /// Transonic.
    Transonic,
    /// Supersonic.
    Supersonic,
    /// Hypersonic atmospheric flight.
    Hypersonic,
}

impl FlightRegime {
    /// Wire label.
    pub fn as_str(self) -> &'static str {
        match self {
            FlightRegime::LowSpeedHighLift => "low_speed_high_lift",
            FlightRegime::Subsonic => "subsonic",
            FlightRegime::Transonic => "transonic",
            FlightRegime::Supersonic => "supersonic",
            FlightRegime::Hypersonic => "hypersonic",
        }
    }
}

/// An operating point in the flight envelope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OperatingPoint {
    /// Mach number.
    pub mach: f64,
    /// Altitude in metres.
    pub altitude_m: f64,
    /// Dynamic pressure in pascals.
    pub dynamic_pressure_pa: f64,
    /// Load factor (dimensionless).
    pub load_factor: f64,
    /// Reynolds number, if known.
    #[serde(default)]
    pub reynolds: Option<f64>,
    /// Static temperature in Kelvin.
    #[serde(default)]
    pub static_temperature_k: Option<f64>,
}

/// An environmental exposure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Environment {
    /// Gust.
    Gust,
    /// Buffet.
    Buffet,
    /// Flutter.
    Flutter,
    /// Acoustic.
    Acoustic,
    /// Thermal gradient.
    ThermalGradient,
    /// Aerodynamic heating.
    AerodynamicHeating,
    /// Icing.
    Icing,
    /// Rain and hail.
    RainHail,
    /// Sand and dust.
    SandDust,
    /// Salt and corrosion.
    SaltCorrosion,
    /// Fluids.
    Fluids,
    /// Ultraviolet.
    Uv,
    /// Bird strike.
    BirdStrike,
    /// Runway debris.
    RunwayDebris,
    /// Vibration.
    Vibration,
    /// Pressure cycling.
    PressureCycling,
    /// Lightning.
    Lightning,
    /// Electromagnetic.
    Electromagnetic,
}

/// A flight-regime envelope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FlightEnvelope {
    /// Regime.
    pub regime: FlightRegime,
    /// Mach range.
    pub mach_range: (f64, f64),
    /// Altitude range in metres.
    pub altitude_range_m: (f64, f64),
    /// Load factor range.
    pub load_factor_range: (f64, f64),
    /// Operating points.
    #[serde(default)]
    pub operating_points: Vec<OperatingPoint>,
    /// Environments that apply.
    #[serde(default)]
    pub environments: Vec<Environment>,
    /// Mission duration in hours.
    #[serde(default)]
    pub mission_duration_h: Option<f64>,
    /// Design service goal in flight hours.
    #[serde(default)]
    pub design_service_goals_h: Option<f64>,
    /// Inspection interval in flight hours.
    #[serde(default)]
    pub inspection_interval_h: Option<f64>,
}

/// Load errors.
#[derive(Debug, Error, PartialEq)]
pub enum LoadError {
    /// The envelope is internally inconsistent.
    #[error("invalid envelope: {0}")]
    Invalid(String),
    /// A point lies outside the declared range.
    #[error("operating point is outside the {regime} envelope")]
    OutsideEnvelope {
        /// Regime.
        regime: String,
        /// Which dimension failed.
        dimension: String,
    },
}

impl FlightEnvelope {
    /// Validate the envelope's internal consistency.
    pub fn validate(&self) -> Result<(), LoadError> {
        if self.mach_range.0 > self.mach_range.1 {
            return Err(LoadError::Invalid("mach_range is inverted".into()));
        }
        if self.altitude_range_m.0 > self.altitude_range_m.1 {
            return Err(LoadError::Invalid(
                "altitude_range_m is inverted".into(),
            ));
        }
        if self.load_factor_range.0 > self.load_factor_range.1 {
            return Err(LoadError::Invalid(
                "load_factor_range is inverted".into(),
            ));
        }
        for point in &self.operating_points {
            self.check_point(point)?;
        }
        Ok(())
    }

    /// Check one operating point against the envelope.
    pub fn check_point(&self, point: &OperatingPoint) -> Result<(), LoadError> {
        let regime = self.regime.as_str();
        if point.mach < self.mach_range.0 || point.mach > self.mach_range.1 {
            return Err(LoadError::OutsideEnvelope {
                regime: regime.to_string(),
                dimension: "mach".to_string(),
            });
        }
        if point.altitude_m < self.altitude_range_m.0
            || point.altitude_m > self.altitude_range_m.1
        {
            return Err(LoadError::OutsideEnvelope {
                regime: regime.to_string(),
                dimension: "altitude".to_string(),
            });
        }
        if point.load_factor < self.load_factor_range.0
            || point.load_factor > self.load_factor_range.1
        {
            return Err(LoadError::OutsideEnvelope {
                regime: regime.to_string(),
                dimension: "load_factor".to_string(),
            });
        }
        Ok(())
    }

    /// Whether an environment applies to this envelope.
    pub fn has_environment(&self, environment: Environment) -> bool {
        self.environments.contains(&environment)
    }

    /// The most severe load factor in the envelope.
    pub fn limit_load_factor(&self) -> f64 {
        self.load_factor_range.1
    }

    /// The ultimate load factor, applying a 1.5 safety factor.
    pub fn ultimate_load_factor(&self) -> f64 {
        self.load_factor_range.1 * 1.5
    }

    /// Canonical hash.
    pub fn content_hash(&self) -> String {
        let canonical = serde_json::json!({
            "regime": self.regime.as_str(),
            "mach_range": self.mach_range,
            "altitude_range_m": self.altitude_range_m,
            "load_factor_range": self.load_factor_range,
            "operating_points": self.operating_points,
            "environments": self.environments,
            "mission_duration_h": self.mission_duration_h,
            "design_service_goals_h": self.design_service_goals_h,
            "inspection_interval_h": self.inspection_interval_h,
        });
        let bytes = serde_json::to_vec(&canonical).unwrap_or_default();
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        format!("{:x}", hasher.finalize())
    }
}

/// A collection of load cases.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct LoadCaseLibrary {
    /// Cases by id.
    pub cases: BTreeMap<String, FlightEnvelope>,
}

impl LoadCaseLibrary {
    /// Empty library.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a case, validating it first.
    pub fn add(&mut self, id: &str, envelope: FlightEnvelope) -> Result<(), LoadError> {
        if id.trim().is_empty() {
            return Err(LoadError::Invalid("case id is empty".into()));
        }
        envelope.validate()?;
        if self.cases.contains_key(id) {
            return Err(LoadError::Invalid(format!("duplicate case {id}")));
        }
        self.cases.insert(id.to_string(), envelope);
        Ok(())
    }

    /// All cases whose regime matches.
    pub fn by_regime(&self, regime: FlightRegime) -> Vec<&FlightEnvelope> {
        self.cases
            .values()
            .filter(|c| c.regime == regime)
            .collect()
    }

    /// The most severe ultimate load factor across the library.
    pub fn worst_ultimate_load_factor(&self) -> Option<f64> {
        self.cases
            .values()
            .map(|c| c.ultimate_load_factor())
            .fold(None, |acc: Option<f64>, v| {
                Some(match acc {
                    Some(a) => a.max(v),
                    None => v,
                })
            })
    }
}

/// A JSON rendering of an environment list.
pub fn environments_to_value(environments: &[Environment]) -> Value {
    Value::Array(
        environments
            .iter()
            .map(|e| Value::String(format!("{e:?}").to_lowercase()))
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn envelope() -> FlightEnvelope {
        FlightEnvelope {
            regime: FlightRegime::Subsonic,
            mach_range: (0.2, 0.85),
            altitude_range_m: (0.0, 12000.0),
            load_factor_range: (-1.0, 2.5),
            operating_points: vec![OperatingPoint {
                mach: 0.5,
                altitude_m: 3000.0,
                dynamic_pressure_pa: 25000.0,
                load_factor: 1.0,
                reynolds: Some(5.0e7),
                static_temperature_k: Some(288.0),
            }],
            environments: vec![Environment::Gust, Environment::Icing],
            mission_duration_h: Some(2.0),
            design_service_goals_h: Some(60000.0),
            inspection_interval_h: Some(500.0),
        }
    }

    #[test]
    fn valid_envelope_passes() {
        assert!(envelope().validate().is_ok());
    }

    #[test]
    fn mach_outside_range_is_rejected() {
        let mut e = envelope();
        e.operating_points[0].mach = 1.5;
        let err = e.validate().unwrap_err();
        assert!(matches!(err, LoadError::OutsideEnvelope { .. }));
    }

    #[test]
    fn altitude_outside_range_is_rejected() {
        let mut e = envelope();
        e.operating_points[0].altitude_m = 20000.0;
        assert!(e.validate().is_err());
    }

    #[test]
    fn load_factor_outside_range_is_rejected() {
        let mut e = envelope();
        e.operating_points[0].load_factor = 9.0;
        assert!(e.validate().is_err());
    }

    #[test]
    fn ultimate_factor_is_limit_times_1_5() {
        let e = envelope();
        assert!((e.ultimate_load_factor() - 3.75).abs() < 1e-9);
    }

    #[test]
    fn environment_membership_is_checked() {
        let e = envelope();
        assert!(e.has_environment(Environment::Gust));
        assert!(!e.has_environment(Environment::Flutter));
    }

    #[test]
    fn library_rejects_duplicates() {
        let mut library = LoadCaseLibrary::new();
        library.add("case-1", envelope()).unwrap();
        assert!(library.add("case-1", envelope()).is_err());
    }

    #[test]
    fn library_filters_by_regime() {
        let mut library = LoadCaseLibrary::new();
        library.add("sub-1", envelope()).unwrap();
        assert_eq!(library.by_regime(FlightRegime::Subsonic).len(), 1);
        assert_eq!(library.by_regime(FlightRegime::Hypersonic).len(), 0);
    }

    #[test]
    fn hash_is_stable() {
        let e = envelope();
        assert_eq!(e.content_hash(), e.content_hash());
    }
}