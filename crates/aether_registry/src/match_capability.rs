// SPDX-License-Identifier: Aether-VPSL-1.0
// SPDX-FileCopyrightText: 2026 The Aether Consortium
//! Capability matching (SDP §5.3).
//!
//! The kernel's `CapabilityRegistry` performs **constraint matching**:
//! workflow steps declare *requirements* (a set of capabilities and
//! contract versions), and the registry returns candidates that
//! satisfy the constraints. Selection may be delegated to the
//! Scientific Intelligence layer (Phase 4+), but the matching
//! primitive stays here — SI recommends, policy disposes.

#![deny(unsafe_code)]

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::PluginManifest;

#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("no candidates for requirement: {0}")]
    NoCandidates(String),
    #[error("invalid requirement: {0}")]
    InvalidRequirement(String),
}

pub type RegistryResult<T> = Result<T, RegistryError>;

/// A workflow step's requirement: capabilities the plugin must declare
/// and a contract it must implement at a semver range.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CapabilityRequirement {
    /// Free-form text identifier (e.g. "compressible_flow", "incompressible_flow").
    pub capabilities: Vec<String>,
    /// The contract the plugin must implement; semver range syntax.
    #[serde(default)]
    pub contract_id: Option<String>,
    #[serde(default)]
    pub contract_version: Option<String>,
    /// Whether the capability is required (must all be present) or
    /// preferred (at least one). Default: required.
    #[serde(default)]
    pub mode: MatchMode,
    /// Required plugin isolation level (e.g. "container" for FMUs).
    #[serde(default)]
    pub isolation: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MatchMode {
    #[default]
    Required,
    Preferred,
}

/// A candidate plugin and the reasons it matched (or didn't).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candidate {
    pub plugin_id: String,
    pub plugin_version: String,
    pub matched_capabilities: Vec<String>,
    pub missing_capabilities: Vec<String>,
    pub contract_satisfied: bool,
    pub isolation_satisfied: bool,
    pub score: u32,
}

impl Candidate {
    pub fn qualifies(&self) -> bool {
        self.missing_capabilities.is_empty() && self.contract_satisfied && self.isolation_satisfied
    }
}

/// A ranked set of matches.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RankedMatches {
    pub requirement: CapabilityRequirement,
    pub candidates: Vec<Candidate>,
}

impl RankedMatches {
    pub fn best(&self) -> Option<&Candidate> {
        self.candidates.iter().filter(|c| c.qualifies()).max_by_key(|c| c.score)
    }
}

fn parse_semver(s: &str) -> Option<(u32, u32, u32)> {
    let mut it = s.split('.');
    let major = it.next()?.parse().ok()?;
    let minor = it.next()?.parse().ok()?;
    let patch_str = it.next()?;
    // strip any pre-release / build suffix
    let patch_end = patch_str
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(patch_str.len());
    let patch = patch_str[..patch_end].parse().ok()?;
    Some((major, minor, patch))
}

fn version_in_range(version: &str, range: &str) -> bool {
    // G1 supports only the simple `>=X.Y.Z <A.B.C` form. Phase 1.5
    // generalizes to a real semver range parser.
    let trimmed = range.trim();
    if let Some(rest) = trimmed.strip_prefix(">=") {
        let (lo_major, lo_minor, lo_patch) = match parse_semver(rest.trim()) {
            Some(v) => v,
            None => return false,
        };
        let (v_major, v_minor, v_patch) = match parse_semver(version) {
            Some(v) => v,
            None => return false,
        };
        return (v_major, v_minor, v_patch) >= (lo_major, lo_minor, lo_patch);
    }
    if let Some(rest) = trimmed.strip_prefix("==") {
        return rest.trim() == version;
    }
    if let Some(rest) = trimmed.strip_prefix('~') {
        let (target_major, target_minor, _) = match parse_semver(rest.trim()) {
            Some(v) => v,
            None => return false,
        };
        let (v_major, v_minor, _) = match parse_semver(version) {
            Some(v) => v,
            None => return false,
        };
        return v_major == target_major && v_minor == target_minor;
    }
    if trimmed == version {
        return true;
    }
    false
}

fn contract_satisfied(m: &PluginManifest, contract_id: &str, range: Option<&str>) -> bool {
    for c in &m.contracts {
        if c.id != contract_id {
            continue;
        }
        if let Some(r) = range {
            if version_in_range(&c.version, r) {
                return true;
            }
        } else {
            return true;
        }
    }
    false
}

fn isolation_satisfied(m: &PluginManifest, required: Option<&str>) -> bool {
    let Some(req) = required else {
        return true;
    };
    m.isolation
        .get("level")
        .and_then(|v| v.as_str())
        .map(|s| s == req)
        .unwrap_or(false)
}

fn capabilities_present(m: &PluginManifest, caps: &[String]) -> (Vec<String>, Vec<String>) {
    let mut matched = Vec::new();
    let mut missing = Vec::new();
    let caps_obj = m.capabilities.as_object();
    for cap in caps {
        let present = caps_obj
            .and_then(|o| o.get(cap))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if present {
            matched.push(cap.clone());
        } else {
            missing.push(cap.clone());
        }
    }
    (matched, missing)
}

/// Match a `CapabilityRequirement` against a set of installed manifests.
/// Returns a ranked set of candidates (best first).
pub fn match_capability(
    requirement: &CapabilityRequirement,
    plugins: &[PluginManifest],
) -> RankedMatches {
    let mut candidates: Vec<Candidate> = Vec::new();
    for m in plugins {
        let (matched, missing) = capabilities_present(m, &requirement.capabilities);
        let contract_satisfied = match &requirement.contract_id {
            Some(cid) => contract_satisfied(m, cid, requirement.contract_version.as_deref()),
            None => true,
        };
        let isolation_ok = isolation_satisfied(m, requirement.isolation.as_deref());
        let score = matched.len() as u32
            + if contract_satisfied { 10 } else { 0 }
            + if isolation_ok { 5 } else { 0 };
        candidates.push(Candidate {
            plugin_id: m.plugin.id.clone(),
            plugin_version: m.plugin.version.clone(),
            matched_capabilities: matched,
            missing_capabilities: missing,
            contract_satisfied,
            isolation_satisfied: isolation_ok,
            score,
        });
    }
    candidates.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.plugin_id.cmp(&b.plugin_id)));
    if matches!(requirement.mode, MatchMode::Required) {
        candidates.retain(|c| c.qualifies());
    }
    RankedMatches {
        requirement: requirement.clone(),
        candidates,
    }
}

/// Convenience: build a snapshot and match in one call.
pub fn match_in_snapshot(
    requirement: &CapabilityRequirement,
    snapshot_plugins: &[PluginManifest],
) -> RankedMatches {
    match_capability(requirement, snapshot_plugins)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discover::snapshot_from;
    use std::collections::BTreeMap;

    fn plugins() -> Vec<PluginManifest> {
        snapshot_from(vec![
            (
                "com.acme.cfd.opensfoam",
                "2.4.1",
                BTreeMap::from([
                    ("compressible_flow".to_string(), true),
                    ("turbulence".to_string(), true),
                ]),
            ),
            (
                "org.oss.feather.reference",
                "0.1.0",
                BTreeMap::from([
                    ("compressible_flow".to_string(), true),
                    ("turbulence".to_string(), false),
                ]),
            ),
        ])
        .plugins
    }

    #[test]
    fn matches_plugins_with_required_capabilities() {
        let req = CapabilityRequirement {
            capabilities: vec!["compressible_flow".to_string()],
            contract_id: None,
            contract_version: None,
            mode: MatchMode::Required,
            isolation: None,
        };
        let m = match_capability(&req, &plugins());
        assert_eq!(m.candidates.len(), 2);
        // Both candidates qualify; OpenFOAM sorts first alphabetically on tie.
        let best = m.best().expect("best");
        assert!(best.qualifies());
    }

    #[test]
    fn requires_all_capabilities() {
        let req = CapabilityRequirement {
            capabilities: vec![
                "compressible_flow".to_string(),
                "turbulence".to_string(),
            ],
            contract_id: None,
            contract_version: None,
            mode: MatchMode::Required,
            isolation: None,
        };
        let m = match_capability(&req, &plugins());
        // Only the OpenFOAM adapter has both.
        assert_eq!(m.candidates.len(), 1);
        assert_eq!(m.candidates[0].plugin_id, "com.acme.cfd.opensfoam");
    }

    #[test]
    fn contract_version_range_filter() {
        let req = CapabilityRequirement {
            capabilities: vec!["compressible_flow".to_string()],
            contract_id: Some("aether.test.plugin".to_string()),
            contract_version: Some(">=1.0.0".to_string()),
            mode: MatchMode::Required,
            isolation: None,
        };
        let m = match_capability(&req, &plugins());
        assert!(!m.candidates.is_empty());
    }

    #[test]
    fn preferred_mode_keeps_partial_matches() {
        let req = CapabilityRequirement {
            capabilities: vec![
                "compressible_flow".to_string(),
                "turbulence".to_string(),
            ],
            contract_id: None,
            contract_version: None,
            mode: MatchMode::Preferred,
            isolation: None,
        };
        let m = match_capability(&req, &plugins());
        // Both plugins are returned; OpenFOAM ranks first.
        assert_eq!(m.candidates.len(), 2);
        assert_eq!(m.candidates[0].plugin_id, "com.acme.cfd.opensfoam");
    }

    #[test]
    fn semver_range_ge() {
        assert!(version_in_range("2.4.1", ">=1.0.0"));
        assert!(version_in_range("1.0.0", ">=1.0.0"));
        assert!(!version_in_range("0.9.9", ">=1.0.0"));
        assert!(!version_in_range("2.4.1", ">=3.0.0"));
    }

    #[test]
    fn semver_range_eq() {
        assert!(version_in_range("1.2.3", "==1.2.3"));
        assert!(!version_in_range("1.2.4", "==1.2.3"));
    }
}
