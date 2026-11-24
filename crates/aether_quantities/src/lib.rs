// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! Typed physical quantities and coordinate frames.
//!
//! Storage is SI canonical. Unit conversion is explicit and dimensional.
//! A quantity whose dimension does not match the requested unit is a loud
//! error, never a silent coercion.

#![deny(unsafe_code)]
#![warn(missing_docs)]

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Number of base dimensions: length, mass, time, temperature, current,
/// amount, luminous intensity.
pub const DIMENSION_COUNT: usize = 7;

/// A physical dimension as integer exponents over the SI base dimensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dimension {
    /// Exponents in `[L, M, T, Theta, I, N, J]` order.
    pub exponents: [i8; DIMENSION_COUNT],
}

impl Dimension {
    /// Dimensionless.
    pub const NONE: Dimension = Dimension { exponents: [0; DIMENSION_COUNT] };
    /// Length.
    pub const LENGTH: Dimension = Dimension { exponents: [1, 0, 0, 0, 0, 0, 0] };
    /// Mass.
    pub const MASS: Dimension = Dimension { exponents: [0, 1, 0, 0, 0, 0, 0] };
    /// Time.
    pub const TIME: Dimension = Dimension { exponents: [0, 0, 1, 0, 0, 0, 0] };
    /// Thermodynamic temperature.
    pub const TEMPERATURE: Dimension = Dimension { exponents: [0, 0, 0, 1, 0, 0, 0] };
    /// Electric current.
    pub const CURRENT: Dimension = Dimension { exponents: [0, 0, 0, 0, 1, 0, 0] };
    /// Force: kg·m/s².
    pub const FORCE: Dimension = Dimension { exponents: [1, 1, -2, 0, 0, 0, 0] };
    /// Pressure/stress: N/m².
    pub const PRESSURE: Dimension = Dimension { exponents: [-1, 1, -2, 0, 0, 0, 0] };
    /// Energy: N·m.
    pub const ENERGY: Dimension = Dimension { exponents: [2, 1, -2, 0, 0, 0, 0] };
    /// Power: J/s.
    pub const POWER: Dimension = Dimension { exponents: [2, 1, -3, 0, 0, 0, 0] };
    /// Frequency: 1/s.
    pub const FREQUENCY: Dimension = Dimension { exponents: [0, 0, -1, 0, 0, 0, 0] };

    /// Multiply dimensions.
    pub fn mul(self, other: Dimension) -> Dimension {
        let mut e = [0i8; DIMENSION_COUNT];
        for i in 0..DIMENSION_COUNT {
            e[i] = self.exponents[i].saturating_add(other.exponents[i]);
        }
        Dimension { exponents: e }
    }

    /// Divide dimensions.
    pub fn div(self, other: Dimension) -> Dimension {
        let mut e = [0i8; DIMENSION_COUNT];
        for i in 0..DIMENSION_COUNT {
            e[i] = self.exponents[i].saturating_sub(other.exponents[i]);
        }
        Dimension { exponents: e }
    }

    /// Raise to an integer power.
    pub fn pow(self, n: i8) -> Dimension {
        let mut e = [0i8; DIMENSION_COUNT];
        for i in 0..DIMENSION_COUNT {
            e[i] = self.exponents[i].saturating_mul(n);
        }
        Dimension { exponents: e }
    }

    /// Human-readable dimension signature.
    pub fn signature(self) -> String {
        let names = ["L", "M", "T", "Theta", "I", "N", "J"];
        names
            .iter()
            .zip(self.exponents)
            .filter(|(_, e)| *e != 0)
            .map(|(n, e)| format!("{n}^{e}"))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// A unit with a dimension, a scale to SI, and an optional offset.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Unit {
    /// Canonical symbol, e.g. `m`, `mm`, `Pa`.
    pub symbol: String,
    /// Physical dimension.
    pub dimension: Dimension,
    /// Multiplicative scale to SI.
    pub scale: f64,
    /// Additive offset to SI (used for temperature scales).
    pub offset: f64,
}

impl Unit {
    /// Construct a linear unit.
    pub fn new(symbol: &str, dimension: Dimension, scale: f64) -> Self {
        Self { symbol: symbol.to_string(), dimension, scale, offset: 0.0 }
    }

    /// Parse a known unit symbol. Unknown symbols are errors.
    pub fn parse(symbol: &str) -> Result<Unit, QuantityError> {
        let s = symbol.trim();
        let unit = match s {
            "1" | "" | "dimensionless" => Unit::new("1", Dimension::NONE, 1.0),
            "m" => Unit::new("m", Dimension::LENGTH, 1.0),
            "mm" => Unit::new("mm", Dimension::LENGTH, 1e-3),
            "cm" => Unit::new("cm", Dimension::LENGTH, 1e-2),
            "km" => Unit::new("km", Dimension::LENGTH, 1e3),
            "in" => Unit::new("in", Dimension::LENGTH, 0.0254),
            "s" => Unit::new("s", Dimension::TIME, 1.0),
            "ms" => Unit::new("ms", Dimension::TIME, 1e-3),
            "min" => Unit::new("min", Dimension::TIME, 60.0),
            "h" => Unit::new("h", Dimension::TIME, 3600.0),
            "kg" => Unit::new("kg", Dimension::MASS, 1.0),
            "g" => Unit::new("g", Dimension::MASS, 1e-3),
            "N" => Unit::new("N", Dimension::FORCE, 1.0),
            "kN" => Unit::new("kN", Dimension::FORCE, 1e3),
            "Pa" => Unit::new("Pa", Dimension::PRESSURE, 1.0),
            "kPa" => Unit::new("kPa", Dimension::PRESSURE, 1e3),
            "MPa" => Unit::new("MPa", Dimension::PRESSURE, 1e6),
            "GPa" => Unit::new("GPa", Dimension::PRESSURE, 1e9),
            "bar" => Unit::new("bar", Dimension::PRESSURE, 1e5),
            "psi" => Unit::new("psi", Dimension::PRESSURE, 6894.757293168),
            "J" => Unit::new("J", Dimension::ENERGY, 1.0),
            "kJ" => Unit::new("kJ", Dimension::ENERGY, 1e3),
            "W" => Unit::new("W", Dimension::POWER, 1.0),
            "Hz" => Unit::new("Hz", Dimension::FREQUENCY, 1.0),
            "K" => Unit::new("K", Dimension::TEMPERATURE, 1.0),
            "degC" | "°C" => Unit {
                symbol: "degC".to_string(),
                dimension: Dimension::TEMPERATURE,
                scale: 1.0,
                offset: 273.15,
            },
            "rad" => Unit::new("rad", Dimension::NONE, 1.0),
            "deg" | "°" => Unit::new("deg", Dimension::NONE, std::f64::consts::PI / 180.0),
            other => return Err(QuantityError::UnknownUnit(other.to_string())),
        };
        Ok(unit)
    }
}

impl fmt::Display for Unit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.symbol)
    }
}

/// A physical quantity: value plus unit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Quantity {
    /// Numeric value in `unit`.
    pub value: f64,
    /// Unit of the value.
    pub unit: Unit,
}

impl Quantity {
    /// Construct a quantity.
    pub fn new(value: f64, unit: Unit) -> Self {
        Self { value, unit }
    }

    /// Parse a value and unit symbol.
    pub fn parse(value: f64, symbol: &str) -> Result<Self, QuantityError> {
        Ok(Self::new(value, Unit::parse(symbol)?))
    }

    /// Convert to SI canonical value.
    pub fn to_si(&self) -> f64 {
        self.value * self.unit.scale + self.unit.offset
    }

    /// Convert to another unit, checking dimensions.
    pub fn to(&self, target: &Unit) -> Result<f64, QuantityError> {
        if self.unit.dimension != target.dimension {
            return Err(QuantityError::DimensionMismatch {
                expected: target.dimension.signature(),
                found: self.unit.dimension.signature(),
            });
        }
        Ok((self.to_si() - target.offset) / target.scale)
    }

    /// Convert into another unit and return a new quantity.
    pub fn converted(&self, target: Unit) -> Result<Quantity, QuantityError> {
        let value = self.to(&target)?;
        Ok(Quantity::new(value, target))
    }

    /// Add two quantities, returning the result in this quantity's unit.
    pub fn add(&self, other: &Quantity) -> Result<Quantity, QuantityError> {
        let other_value = other.to(&self.unit)?;
        Ok(Quantity::new(self.value + other_value, self.unit.clone()))
    }

    /// Subtract two quantities, returning the result in this quantity's unit.
    pub fn sub(&self, other: &Quantity) -> Result<Quantity, QuantityError> {
        let other_value = other.to(&self.unit)?;
        Ok(Quantity::new(self.value - other_value, self.unit.clone()))
    }

    /// Multiply by a dimensionless scalar.
    pub fn scale(&self, factor: f64) -> Quantity {
        Quantity::new(self.value * factor, self.unit.clone())
    }
}

/// Quantity and frame errors.
#[derive(Debug, Error, PartialEq)]
pub enum QuantityError {
    /// Unknown unit symbol.
    #[error("unknown unit: {0}")]
    UnknownUnit(String),
    /// Dimension mismatch.
    #[error("dimension mismatch: expected {expected}, found {found}")]
    DimensionMismatch {
        /// Expected dimension signature.
        expected: String,
        /// Found dimension signature.
        found: String,
    },
    /// Unknown frame.
    #[error("unknown frame: {0}")]
    UnknownFrame(String),
    /// Cyclic frame parent chain.
    #[error("cyclic frame graph at {0}")]
    CyclicFrame(String),
    /// Singular or invalid transform.
    #[error("invalid transform for frame {0}")]
    InvalidTransform(String),
}

/// Frame handedness.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Handedness {
    /// Right-handed.
    Right,
    /// Left-handed.
    Left,
}

/// A named coordinate frame with a transform to its parent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Frame {
    /// Stable frame id.
    pub id: String,
    /// Parent frame id; `None` for the root.
    pub parent: Option<String>,
    /// 4×4 homogeneous transform from this frame to the parent.
    pub transform: [[f64; 4]; 4],
    /// Handedness declaration.
    pub handedness: Handedness,
    /// Optional validity start (ISO-8601).
    pub valid_from: Option<String>,
    /// Optional validity end (ISO-8601).
    pub valid_to: Option<String>,
    /// Provenance of the transform (source, method, revision).
    pub provenance: Option<String>,
}

impl Frame {
    /// Identity frame.
    pub fn identity(id: &str) -> Self {
        Self {
            id: id.to_string(),
            parent: None,
            transform: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            handedness: Handedness::Right,
            valid_from: None,
            valid_to: None,
            provenance: None,
        }
    }
}

/// A graph of coordinate frames.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FrameGraph {
    /// Frames by id.
    pub frames: BTreeMap<String, Frame>,
}

impl FrameGraph {
    /// Empty graph.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a frame. Rejects duplicate ids.
    pub fn add(&mut self, frame: Frame) -> Result<(), QuantityError> {
        if self.frames.contains_key(&frame.id) {
            return Err(QuantityError::InvalidTransform(format!(
                "duplicate frame {}",
                frame.id
            )));
        }
        self.frames.insert(frame.id.clone(), frame);
        Ok(())
    }

    /// Look up a frame.
    pub fn get(&self, id: &str) -> Result<&Frame, QuantityError> {
        self.frames
            .get(id)
            .ok_or_else(|| QuantityError::UnknownFrame(id.to_string()))
    }

    /// Transform a point from `from` to the root frame.
    pub fn point_to_root(&self, from: &str, point: [f64; 3]) -> Result<[f64; 3], QuantityError> {
        let mut current = self.get(from)?.clone();
        let mut p = point;
        let mut seen = std::collections::BTreeSet::new();
        loop {
            if !seen.insert(current.id.clone()) {
                return Err(QuantityError::CyclicFrame(current.id));
            }
            p = apply(&current.transform, p)?;
            match &current.parent {
                None => return Ok(p),
                Some(parent) => current = self.get(parent)?.clone(),
            }
        }
    }

    /// Transform a point between two frames.
    pub fn point_between(
        &self,
        from: &str,
        to: &str,
        point: [f64; 3],
    ) -> Result<[f64; 3], QuantityError> {
        let root = self.point_to_root(from, point)?;
        self.point_from_root(to, root)
    }

    /// Transform a point from the root frame into `to`.
    pub fn point_from_root(&self, to: &str, point: [f64; 3]) -> Result<[f64; 3], QuantityError> {
        let chain = self.chain_to_root(to)?;
        let mut p = point;
        for frame in chain.iter().rev() {
            p = apply_inverse(&frame.transform, p)?;
        }
        Ok(p)
    }

    /// Transform a direction vector (no translation) from `from` to root.
    pub fn vector_to_root(&self, from: &str, vector: [f64; 3]) -> Result<[f64; 3], QuantityError> {
        let mut current = self.get(from)?.clone();
        let mut v = vector;
        let mut seen = std::collections::BTreeSet::new();
        loop {
            if !seen.insert(current.id.clone()) {
                return Err(QuantityError::CyclicFrame(current.id));
            }
            v = apply_vector(&current.transform, v);
            match &current.parent {
                None => return Ok(v),
                Some(parent) => current = self.get(parent)?.clone(),
            }
        }
    }

    fn chain_to_root(&self, id: &str) -> Result<Vec<Frame>, QuantityError> {
        let mut chain = Vec::new();
        let mut current = self.get(id)?.clone();
        let mut seen = std::collections::BTreeSet::new();
        loop {
            if !seen.insert(current.id.clone()) {
                return Err(QuantityError::CyclicFrame(current.id));
            }
            let parent = current.parent.clone();
            chain.push(current);
            match parent {
                None => return Ok(chain),
                Some(p) => current = self.get(&p)?.clone(),
            }
        }
    }
}

fn apply(m: &[[f64; 4]; 4], p: [f64; 3]) -> Result<[f64; 3], QuantityError> {
    let w = m[3][0] * p[0] + m[3][1] * p[1] + m[3][2] * p[2] + m[3][3];
    if w.abs() < 1e-15 {
        return Err(QuantityError::InvalidTransform("zero homogeneous w".into()));
    }
    Ok([
        (m[0][0] * p[0] + m[0][1] * p[1] + m[0][2] * p[2] + m[0][3]) / w,
        (m[1][0] * p[0] + m[1][1] * p[1] + m[1][2] * p[2] + m[1][3]) / w,
        (m[2][0] * p[0] + m[2][1] * p[1] + m[2][2] * p[2] + m[2][3]) / w,
    ])
}

fn apply_inverse(m: &[[f64; 4]; 4], p: [f64; 3]) -> Result<[f64; 3], QuantityError> {
    // Rigid-transform inverse: R^T (p - t).
    let t = [m[0][3], m[1][3], m[2][3]];
    let d = [p[0] - t[0], p[1] - t[1], p[2] - t[2]];
    Ok([
        m[0][0] * d[0] + m[1][0] * d[1] + m[2][0] * d[2],
        m[0][1] * d[0] + m[1][1] * d[1] + m[2][1] * d[2],
        m[0][2] * d[0] + m[1][2] * d[1] + m[2][2] * d[2],
    ])
}

fn apply_vector(m: &[[f64; 4]; 4], v: [f64; 3]) -> [f64; 3] {
    [
        m[0][0] * v[0] + m[0][1] * v[1] + m[0][2] * v[2],
        m[1][0] * v[0] + m[1][1] * v[1] + m[1][2] * v[2],
        m[2][0] * v[0] + m[2][1] * v[1] + m[2][2] * v[2],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_units() {
        let q = Quantity::parse(1.0, "m").unwrap();
        assert!((q.to(&Unit::parse("mm").unwrap()).unwrap() - 1000.0).abs() < 1e-9);
        assert!((q.to(&Unit::parse("in").unwrap()).unwrap() - 39.37007874015748).abs() < 1e-9);
    }

    #[test]
    fn rejects_dimension_mismatch() {
        let q = Quantity::parse(1.0, "Pa").unwrap();
        assert!(matches!(
            q.to(&Unit::parse("m").unwrap()),
            Err(QuantityError::DimensionMismatch { .. })
        ));
    }

    #[test]
    fn temperature_offset() {
        let c = Quantity::parse(0.0, "degC").unwrap();
        assert!((c.to(&Unit::parse("K").unwrap()).unwrap() - 273.15).abs() < 1e-9);
    }

    #[test]
    fn adds_compatible_quantities() {
        let a = Quantity::parse(1.0, "m").unwrap();
        let b = Quantity::parse(500.0, "mm").unwrap();
        let c = a.add(&b).unwrap();
        assert!((c.value - 1.5).abs() < 1e-12);
    }

    #[test]
    fn frame_translation_round_trip() {
        let mut g = FrameGraph::new();
        g.add(Frame::identity("root")).unwrap();
        let mut child = Frame::identity("wing");
        child.parent = Some("root".into());
        child.transform[0][3] = 10.0;
        g.add(child).unwrap();
        let p = g.point_between("root", "wing", [12.0, 0.0, 0.0]).unwrap();
        assert!((p[0] - 2.0).abs() < 1e-12);
        let back = g.point_between("wing", "root", p).unwrap();
        assert!((back[0] - 12.0).abs() < 1e-12);
    }

    #[test]
    fn cycle_is_rejected() {
        let mut g = FrameGraph::new();
        let mut a = Frame::identity("a");
        a.parent = Some("b".into());
        let mut b = Frame::identity("b");
        b.parent = Some("a".into());
        g.add(a).unwrap();
        g.add(b).unwrap();
        assert!(matches!(
            g.point_to_root("a", [0.0, 0.0, 0.0]),
            Err(QuantityError::CyclicFrame(_))
        ));
    }
}
