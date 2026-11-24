// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! Health checks and dependency status (spec §14.1).
//!
//! A health report is deterministic and side-effect free. It reports
//! capability presence honestly: optional tools that are absent are
//! `unavailable`, not `healthy`, and no dependency is silently assumed.

#![deny(unsafe_code)]
#![warn(missing_docs)]

use std::collections::BTreeMap;
use std::process::Command;

use serde::{Deserialize, Serialize};

/// Health status of one component.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Health {
    /// Present and responding.
    Healthy,
    /// Present but degraded.
    Degraded,
    /// Absent or unreachable.
    Unavailable,
}

/// A full health report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HealthReport {
    /// Overall status.
    pub overall: Health,
    /// Component statuses.
    pub components: BTreeMap<String, Health>,
    /// Honest note.
    pub note: String,
}

fn has_command(name: &str) -> bool {
    Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {name} >/dev/null 2>&1"))
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Probe local dependencies without mutating state.
pub fn probe() -> HealthReport {
    let mut components = BTreeMap::new();
    components.insert("python".to_string(), if has_command("python3") { Health::Healthy } else { Health::Unavailable });
    components.insert("cargo".to_string(), if has_command("cargo") { Health::Healthy } else { Health::Unavailable });
    components.insert("sbatch".to_string(), if has_command("sbatch") { Health::Healthy } else { Health::Unavailable });
    components.insert("sandbox_exec".to_string(), if has_command("sandbox-exec") { Health::Healthy } else { Health::Unavailable });
    components.insert("bwrap".to_string(), if has_command("bwrap") { Health::Healthy } else { Health::Unavailable });
    let overall = if components.values().any(|h| *h == Health::Unavailable) {
        Health::Degraded
    } else {
        Health::Healthy
    };
    HealthReport {
        overall,
        components,
        note: "absent optional dependencies are reported, not assumed".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_reports_components() {
        let report = probe();
        assert!(report.components.contains_key("python"));
        assert!(report.components.contains_key("sbatch"));
        assert!(!report.note.is_empty());
    }

    #[test]
    fn absent_optional_tool_is_not_healthy() {
        let report = probe();
        if !has_command("sbatch") {
            assert_eq!(report.components["sbatch"], Health::Unavailable);
            assert_eq!(report.overall, Health::Degraded);
        }
    }
}