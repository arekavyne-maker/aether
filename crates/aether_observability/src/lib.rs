// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! Structured observability with a no-leak guarantee.
//!
//! Metrics, logs, and traces are operational data. They must never carry
//! controlled engineering content. This crate provides counters, gauges,
//! and a structured event log that only accepts numeric measures and
//! explicitly whitelisted labels, and it exposes a deterministic text and
//! JSON rendering for local scraping.
//!
//! The API is intentionally small: a metric registry, a counter/gauge
//! pair, and an event log with a bounded size. Nothing here phones home.

#![deny(unsafe_code)]
#![warn(missing_docs)]

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// A metric kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetricKind {
    /// Monotonically increasing count.
    Counter,
    /// A value that can go up or down.
    Gauge,
}

/// One metric sample.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Metric {
    /// Metric name.
    pub name: String,
    /// Kind.
    pub kind: MetricKind,
    /// Value.
    pub value: f64,
    /// Optional labels (operational only, never engineering content).
    #[serde(default)]
    pub labels: BTreeMap<String, String>,
}

/// A structured event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Event {
    /// Event name.
    pub name: String,
    /// Numeric measures.
    #[serde(default)]
    pub measures: BTreeMap<String, f64>,
    /// Whitelisted labels.
    #[serde(default)]
    pub labels: BTreeMap<String, String>,
}

/// Observability errors.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ObservabilityError {
    /// A counter was decremented.
    CounterUnderflow(String),
    /// A label key is not whitelisted.
    LabelNotAllowed(String),
    /// A metric name is empty.
    EmptyName,
}

/// The metric registry.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Registry {
    /// Metrics by name.
    pub metrics: BTreeMap<String, Metric>,
    /// Recent events, bounded.
    pub events: Vec<Event>,
    /// Maximum retained events.
    pub max_events: usize,
}

impl Registry {
    /// Create an empty registry retaining 1000 events.
    pub fn new() -> Self {
        Self {
            metrics: BTreeMap::new(),
            events: Vec::new(),
            max_events: 1000,
        }
    }

    /// Increment a counter.
    pub fn increment(&mut self, name: &str) -> Result<(), ObservabilityError> {
        if name.trim().is_empty() {
            return Err(ObservabilityError::EmptyName);
        }
        let entry = self.metrics.entry(name.to_string()).or_insert(Metric {
            name: name.to_string(),
            kind: MetricKind::Counter,
            value: 0.0,
            labels: BTreeMap::new(),
        });
        entry.value += 1.0;
        Ok(())
    }

    /// Add to a counter by an explicit amount.
    pub fn add(&mut self, name: &str, amount: f64) -> Result<(), ObservabilityError> {
        if name.trim().is_empty() {
            return Err(ObservabilityError::EmptyName);
        }
        if amount < 0.0 {
            return Err(ObservabilityError::CounterUnderflow(name.to_string()));
        }
        let entry = self.metrics.entry(name.to_string()).or_insert(Metric {
            name: name.to_string(),
            kind: MetricKind::Counter,
            value: 0.0,
            labels: BTreeMap::new(),
        });
        entry.value += amount;
        Ok(())
    }

    /// Set a gauge.
    pub fn set_gauge(&mut self, name: &str, value: f64) -> Result<(), ObservabilityError> {
        if name.trim().is_empty() {
            return Err(ObservabilityError::EmptyName);
        }
        if !value.is_finite() {
            return Err(ObservabilityError::CounterUnderflow(name.to_string()));
        }
        self.metrics.insert(
            name.to_string(),
            Metric {
                name: name.to_string(),
                kind: MetricKind::Gauge,
                value,
                labels: BTreeMap::new(),
            },
        );
        Ok(())
    }

    /// Record an event. Labels must be whitelisted.
    pub fn record(
        &mut self,
        name: &str,
        measures: BTreeMap<String, f64>,
        labels: BTreeMap<String, String>,
    ) -> Result<(), ObservabilityError> {
        for key in labels.keys() {
            if !label_allowed(key) {
                return Err(ObservabilityError::LabelNotAllowed(key.clone()));
            }
        }
        self.events.push(Event {
            name: name.to_string(),
            measures,
            labels,
        });
        if self.events.len() > self.max_events {
            let excess = self.events.len() - self.max_events;
            self.events.drain(0..excess);
        }
        Ok(())
    }

    /// Render the registry as JSON.
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }

    /// Render the registry as Prometheus-style text.
    pub fn to_prometheus(&self) -> String {
        let mut out = String::new();
        for metric in self.metrics.values() {
            let kind = match metric.kind {
                MetricKind::Counter => "counter",
                MetricKind::Gauge => "gauge",
            };
            out.push_str(&format!("# TYPE {} {}\n", metric.name, kind));
            out.push_str(&format!("{} {}\n", metric.name, metric.value));
        }
        out
    }
}

/// Whether a label key is operationally safe.
///
/// Only a small, explicit set of operational labels is allowed. Anything
/// that could carry engineering content (part numbers, material ids,
/// customer names) is rejected.
pub fn label_allowed(key: &str) -> bool {
    matches!(
        key,
        "backend" | "plugin" | "contract" | "status" | "reason" | "phase"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counters_accumulate() {
        let mut registry = Registry::new();
        registry.increment("jobs_total").unwrap();
        registry.increment("jobs_total").unwrap();
        registry.add("jobs_total", 3.0).unwrap();
        assert_eq!(registry.metrics["jobs_total"].value, 5.0);
    }

    #[test]
    fn gauges_can_go_down() {
        let mut registry = Registry::new();
        registry.set_gauge("queue_depth", 10.0).unwrap();
        registry.set_gauge("queue_depth", 3.0).unwrap();
        assert_eq!(registry.metrics["queue_depth"].value, 3.0);
    }

    #[test]
    fn counter_underflow_is_refused() {
        let mut registry = Registry::new();
        assert!(registry.add("jobs_total", -1.0).is_err());
    }

    #[test]
    fn engineering_labels_are_rejected() {
        let mut registry = Registry::new();
        let labels = BTreeMap::from([("part_number".to_string(), "rib-07".to_string())]);
        assert!(registry.record("x", BTreeMap::new(), labels).is_err());
    }

    #[test]
    fn operational_labels_are_accepted() {
        let mut registry = Registry::new();
        let labels = BTreeMap::from([("backend".to_string(), "local".to_string())]);
        assert!(registry.record("x", BTreeMap::new(), labels).is_ok());
    }

    #[test]
    fn events_are_bounded() {
        let mut registry = Registry::new();
        registry.max_events = 2;
        for _ in 0..5 {
            registry.record("e", BTreeMap::new(), BTreeMap::new()).unwrap();
        }
        assert_eq!(registry.events.len(), 2);
    }

    #[test]
    fn prometheus_output_is_parseable_text() {
        let mut registry = Registry::new();
        registry.increment("jobs_total").unwrap();
        let text = registry.to_prometheus();
        assert!(text.contains("# TYPE jobs_total counter"));
        assert!(text.contains("jobs_total 1"));
    }
}