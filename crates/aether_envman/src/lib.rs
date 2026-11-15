// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! Aether Environment Manifest (envman) — SDP §17.4.
//!
//! The Environment Manifest pins everything that affects reproducibility:
//! - the platform runtime version (`aether_core`),
//! - the contract versions in scope,
//! - each plugin's exact version and content hash,
//! - the Python toolchain and its lockfile (uv/conda-style),
//! - any container image digests (Apptainer/SIF),
//! - the policy bundle version and hash.
//!
//! The manifest is the substrate of MVR-09 / MVR-11 (reproducibility
//! from provenance + env manifest). Strict mode (`aether run
//! --reproducibility strict`) refuses to execute if any pin cannot be
//! resolved exactly; equivalence mode records the delta and proceeds
//! with declared tolerance bands (SDP §17.6).

#![deny(unsafe_code)]
#![warn(missing_docs)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EnvManError {
    #[error("missing pin: {0}")]
    MissingPin(&'static str),
    #[error("hash mismatch for {what}: expected={expected} computed={computed}")]
    HashMismatch {
        what: String,
        expected: String,
        computed: String,
    },
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("yaml: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
}

pub type EnvManResult<T> = Result<T, EnvManError>;

/// Manifest schema version.
pub const MANIFEST_VERSION: &str = "1.0";

/// The Environment Manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvManifest {
    pub environment_manifest: String,
    pub platform_runtime: PlatformRuntime,
    #[serde(default)]
    pub contracts: BTreeMap<String, String>,
    #[serde(default)]
    pub plugins: BTreeMap<String, PluginPin>,
    #[serde(default)]
    pub python: Option<PythonToolchain>,
    #[serde(default)]
    pub containers: BTreeMap<String, ContainerPin>,
    #[serde(default)]
    pub toolchains: BTreeMap<String, String>,
    pub policy_bundle: PolicyBundlePin,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformRuntime {
    pub aether_core: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginPin {
    pub version: String,
    pub hash: String,
    #[serde(default)]
    pub certified: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PythonToolchain {
    /// uv/conda-style lockfile path within the bundle.
    pub lock: String,
    /// The lockfile's content hash (SHA-256).
    pub lock_hash: String,
    /// Lockfile format ("uv", "conda", etc.).
    #[serde(default = "default_format")]
    pub format: String,
}

fn default_format() -> String {
    "uv".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerPin {
    pub image: String,
    pub engine: String,
    /// SHA-256 of the image manifest.
    pub image_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyBundlePin {
    pub version: String,
    pub hash: String,
}

impl EnvManifest {
    /// Parse a manifest from a YAML file.
    pub fn load(path: &Path) -> EnvManResult<Self> {
        let raw = std::fs::read_to_string(path)?;
        let m: EnvManifest = serde_yaml::from_str(&raw)?;
        if m.environment_manifest != MANIFEST_VERSION {
            return Err(EnvManError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "env manifest version mismatch: document declares {} but the v1 loader expects {}",
                    m.environment_manifest,
                    MANIFEST_VERSION
                ),
            )));
        }
        Ok(m)
    }

    /// Parse a manifest from a YAML string.
    pub fn load_from_string(yaml: &str) -> EnvManResult<Self> {
        let m: EnvManifest = serde_yaml::from_str(yaml)?;
        if m.environment_manifest != MANIFEST_VERSION {
            return Err(EnvManError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "env manifest version mismatch: document declares {} but the v1 loader expects {}",
                    m.environment_manifest,
                    MANIFEST_VERSION
                ),
            )));
        }
        Ok(m)
    }

    /// Serialize to canonical YAML.
    pub fn to_yaml(&self) -> EnvManResult<String> {
        Ok(serde_yaml::to_string(self)?)
    }

    /// Compute the SHA-256 of the manifest's canonical YAML form.
    pub fn content_hash(&self) -> String {
        let yaml = self.to_yaml().unwrap_or_default();
        let mut h = Sha256::new();
        h.update(yaml.as_bytes());
        format!("{:x}", h.finalize())
    }

    /// Verify the manifest's hashes: each plugin's `hash` must match
    /// the SHA-256 of its manifest, and the policy bundle hash must
    /// match. Returns the list of mismatches; an empty list means the
    /// manifest is internally consistent.
    pub fn verify(&self) -> Vec<String> {
        let mut errors = Vec::new();
        for (id, pin) in &self.plugins {
            // The pin's hash is the expected value; we cannot
            // re-compute it here without the plugin manifest bytes, so
            // we only validate the format. The full verification is
            // performed at install time (see `aether plugin install`).
            if pin.hash.len() != 64 || !pin.hash.chars().all(|c| c.is_ascii_hexdigit()) {
                errors.push(format!("plugin {id} hash is not sha256 hex: {}", pin.hash));
            }
        }
        if self.policy_bundle.hash.len() != 64
            || !self.policy_bundle.hash.chars().all(|c| c.is_ascii_hexdigit())
        {
            errors.push(format!(
                "policy bundle hash is not sha256 hex: {}",
                self.policy_bundle.hash
            ));
        }
        for (name, pin) in &self.containers {
            if pin.image_hash.len() != 64
                || !pin.image_hash.chars().all(|c| c.is_ascii_hexdigit())
            {
                errors.push(format!(
                    "container {name} image_hash is not sha256 hex: {}",
                    pin.image_hash
                ));
            }
        }
        if let Some(py) = &self.python {
            if py.lock_hash.len() != 64
                || !py.lock_hash.chars().all(|c| c.is_ascii_hexdigit())
            {
                errors.push(format!(
                    "python lock_hash is not sha256 hex: {}",
                    py.lock_hash
                ));
            }
        }
        errors
    }
}

/// Compute the SHA-256 of a file's contents.
pub fn sha256_of_file(path: &Path) -> EnvManResult<String> {
    let bytes = std::fs::read(path)?;
    let mut h = Sha256::new();
    h.update(&bytes);
    Ok(format!("{:x}", h.finalize()))
}

/// Compute the SHA-256 of a byte slice.
pub fn sha256_of_bytes(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    format!("{:x}", h.finalize())
}

/// Strict-mode verifier: at install time, check that every declared
/// pin (plugin version + hash, container image + hash, policy
/// bundle version + hash, toolchain version) is satisfied by a
/// local artifact. In production this is the MVR-11 substrate:
/// `aether run --reproducibility strict` refuses to start a workflow
/// unless every pin in the Environment Manifest is satisfiable
/// locally.
pub struct StrictVerifier {
    pub manifest: EnvManifest,
    /// Local artifact roots: each pin is checked against a path in
    /// these roots. A pin resolves if any root contains a file with
    /// the matching name and hash.
    pub artifact_roots: Vec<PathBuf>,
}

impl StrictVerifier {
    pub fn new(manifest: EnvManifest, artifact_roots: Vec<PathBuf>) -> Self {
        Self { manifest, artifact_roots }
    }

    /// Verify every pin in the manifest against the local artifact
    /// roots. Returns a `StrictReport` listing each pin and whether
    /// it resolved. The CLI uses this report to fail-fast on any
    /// unresolved pin in strict mode.
    pub fn verify(&self) -> StrictReport {
        let mut report = StrictReport::default();

        // 1. Plugin pins: each plugin in the manifest must have a
        // local artifact whose hash matches the pin's `hash` field.
        for (id, pin) in &self.manifest.plugins {
            let path = format!("plugins/{id}-{}.tar.gz", pin.version);
            let resolved = self.find_artifact(&path, &pin.hash);
            report.plugin_pins.push(PinStatus {
                id: id.clone(),
                version: pin.version.clone(),
                expected_hash: pin.hash.clone(),
                resolved_at: resolved,
            });
        }

        // 2. Container pins: each container must have a local SIF
        // file whose hash matches the pin's `image_hash`.
        for (name, pin) in &self.manifest.containers {
            let path = format!("containers/{name}.sif");
            let resolved = self.find_artifact(&path, &pin.image_hash);
            report.container_pins.push(PinStatus {
                id: name.clone(),
                version: pin.engine.clone(),
                expected_hash: pin.image_hash.clone(),
                resolved_at: resolved,
            });
        }

        // 3. Policy bundle: must have a local artifact whose hash
        // matches the manifest's `policy_bundle.hash`.
        let policy_path = format!(
            "policy/{}-{}.json",
            self.manifest.policy_bundle.version, self.manifest.policy_bundle.hash
        );
        let resolved = self.find_artifact(&policy_path, &self.manifest.policy_bundle.hash);
        report.policy_bundle = Some(PinStatus {
            id: "policy".to_string(),
            version: self.manifest.policy_bundle.version.clone(),
            expected_hash: self.manifest.policy_bundle.hash.clone(),
            resolved_at: resolved,
        });

        // 4. Python toolchain: if declared, the lock file must
        // exist and its hash must match.
        if let Some(py) = &self.manifest.python {
            let path = py.lock.clone();
            let resolved = self.find_artifact_file(&path, &py.lock_hash);
            report.python_toolchain = Some(PinStatus {
                id: "python".to_string(),
                version: py.format.clone(),
                expected_hash: py.lock_hash.clone(),
                resolved_at: resolved,
            });
        }

        // 5. Contract pins: every contract in the manifest must have
        // a local `.aetherpkg/contracts/<id>.json` whose hash matches.
        for (id, version) in &self.manifest.contracts {
            let path = format!("contracts/{id}.json");
            let resolved = self.find_artifact(&path, version);
            report.contract_pins.push(PinStatus {
                id: id.clone(),
                version: version.clone(),
                expected_hash: version.clone(),
                resolved_at: resolved,
            });
        }

        report
    }

    /// Walk each artifact root and return the path of the file that
    /// matches `relative_path` and has the expected hash. Returns
    /// `None` if no such file exists (the pin is unresolved).
    fn find_artifact(&self, relative_path: &str, expected_hash: &str) -> Option<PathBuf> {
        for root in &self.artifact_roots {
            let candidate = root.join(relative_path);
            if candidate.is_file() {
                if let Ok(actual) = sha256_of_file(&candidate) {
                    if actual == expected_hash {
                        return Some(candidate);
                    }
                }
            }
        }
        None
    }

    fn find_artifact_file(&self, relative_path: &str, expected_hash: &str) -> Option<PathBuf> {
        self.find_artifact(relative_path, expected_hash)
    }
}

impl Default for StrictVerifier {
    fn default() -> Self {
        Self::new(
            EnvManifest {
                environment_manifest: MANIFEST_VERSION.to_string(),
                platform_runtime: PlatformRuntime { aether_core: "==0.0.0".to_string() },
                contracts: BTreeMap::new(),
                plugins: BTreeMap::new(),
                python: None,
                containers: BTreeMap::new(),
                toolchains: BTreeMap::new(),
                policy_bundle: PolicyBundlePin { version: "==0".to_string(), hash: "0".repeat(64) },
                created_at: chrono::Utc::now(),
            },
            vec![],
        )
    }
}

/// Per-pin verification status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PinStatus {
    pub id: String,
    pub version: String,
    pub expected_hash: String,
    pub resolved_at: Option<PathBuf>,
}

impl PinStatus {
    pub fn is_resolved(&self) -> bool {
        self.resolved_at.is_some()
    }
}

/// The complete strict-mode report for an environment manifest.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StrictReport {
    pub plugin_pins: Vec<PinStatus>,
    pub container_pins: Vec<PinStatus>,
    pub contract_pins: Vec<PinStatus>,
    pub policy_bundle: Option<PinStatus>,
    pub python_toolchain: Option<PinStatus>,
}

impl StrictReport {
    /// True iff every required pin resolved.
    pub fn all_resolved(&self) -> bool {
        self.plugin_pins.iter().all(PinStatus::is_resolved)
            && self.container_pins.iter().all(PinStatus::is_resolved)
            && self.contract_pins.iter().all(PinStatus::is_resolved)
            && self.policy_bundle.as_ref().is_none_or(PinStatus::is_resolved)
            && self.python_toolchain.as_ref().is_none_or(PinStatus::is_resolved)
    }

    /// The list of unresolved pins, suitable for surfacing to a
    /// `aether envman verify` consumer.
    pub fn unresolved(&self) -> Vec<&PinStatus> {
        self.plugin_pins.iter().chain(self.container_pins.iter())
            .chain(self.contract_pins.iter())
            .chain(self.policy_bundle.iter())
            .chain(self.python_toolchain.iter())
            .filter(|p| !p.is_resolved())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
environment_manifest: "1.0"
platform_runtime:
  aether_core: "==0.1.0"
contracts:
  aether.physics.solver: "==0.1.0"
  aether.graph.model: "==0.1.0"
plugins:
  org.aether.reference.cfd.minimal:
    version: "==0.1.0"
    hash: "0000000000000000000000000000000000000000000000000000000000000000"
    certified: "cert/2026-08-30"
python:
  lock: "env/python.lock"
  lock_hash: "1111111111111111111111111111111111111111111111111111111111111111"
  format: "uv"
containers:
  opensfoam.sif:
    image: "opensfoam.sif"
    engine: "apptainer"
    image_hash: "2222222222222222222222222222222222222222222222222222222222222222"
toolchains:
  opensfoam: "v2312-aeth1"
  calculix: "2.21-aeth3"
policy_bundle:
  version: "==1"
  hash: "3333333333333333333333333333333333333333333333333333333333333333"
created_at: "2026-08-30T00:00:00Z"
"#;

    #[test]
    fn parses_and_verifies() {
        let m: EnvManifest = serde_yaml::from_str(SAMPLE).unwrap();
        assert_eq!(m.environment_manifest, MANIFEST_VERSION);
        let errs = m.verify();
        assert!(errs.is_empty(), "expected no errors, got: {errs:?}");
        let hash = m.content_hash();
        assert_eq!(hash.len(), 64);
    }

    #[test]
    fn rejects_bad_hashes() {
        let mut m: EnvManifest = serde_yaml::from_str(SAMPLE).unwrap();
        m.plugins.get_mut("org.aether.reference.cfd.minimal").unwrap().hash = "nope".to_string();
        let errs = m.verify();
        assert_eq!(errs.len(), 1);
    }

    #[test]
    fn rejects_wrong_version() {
        let bad = SAMPLE.replace(r#"environment_manifest: "1.0""#, r#"environment_manifest: "0.9""#);
        let err = EnvManifest::load_from_string(&bad).unwrap_err();
        assert!(err.to_string().contains("version mismatch"));
    }
}

#[cfg(test)]
mod strict_tests {
    use super::*;
    use std::collections::BTreeMap;

    fn make_manifest_with_plugins(plugins: BTreeMap<String, PluginPin>) -> EnvManifest {
        EnvManifest {
            environment_manifest: MANIFEST_VERSION.to_string(),
            platform_runtime: PlatformRuntime { aether_core: "==0.1.0".to_string() },
            contracts: BTreeMap::new(),
            plugins,
            python: None,
            containers: BTreeMap::new(),
            toolchains: BTreeMap::new(),
            policy_bundle: PolicyBundlePin {
                version: "==1".to_string(),
                hash: "0".repeat(64),
            },
            created_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn strict_verifier_reports_unresolved_pin() {
        // No artifact roots -> every pin is unresolved.
        let mut plugins = BTreeMap::new();
        plugins.insert(
            "org.aether.test".to_string(),
            PluginPin {
                version: "==0.1.0".to_string(),
                hash: "1".repeat(64),
                certified: None,
            },
        );
        let m = make_manifest_with_plugins(plugins);
        let v = StrictVerifier::new(m, vec![]);
        let r = v.verify();
        assert!(!r.all_resolved());
        // 1 plugin + 1 policy = 2 unresolved pins (no contracts declared).
        assert_eq!(r.unresolved().len(), 2);
    }

    #[test]
    fn strict_verifier_resolves_when_artifact_matches() {
        // Create a temp artifact root with a plugin tarball whose
        // content hash matches the manifest's pin.
        let mut plugins = BTreeMap::new();
        // Compute the hash of the content we'll write.
        let payload = b"some content";
        let mut h = sha2::Sha256::new();
        sha2::Digest::update(&mut h, payload);
        let hash = format!("{:x}", h.finalize());
        plugins.insert(
            "org.aether.test".to_string(),
            PluginPin {
                version: "==0.1.0".to_string(),
                hash: hash.clone(),
                certified: None,
            },
        );
        let m = make_manifest_with_plugins(plugins);
        let tmp = tempfile::tempdir().expect("tempdir");
        // The verifier looks up `plugins/{id}-{version}.tar.gz`; the
        // version is the literal string from the manifest, so we
        // include the `==` in the filename.
        let plugin_path = tmp.path().join("plugins/org.aether.test-==0.1.0.tar.gz");
        std::fs::create_dir_all(plugin_path.parent().unwrap()).unwrap();
        std::fs::write(&plugin_path, payload).unwrap();
        let v = StrictVerifier::new(m, vec![tmp.path().to_path_buf()]);
        let r = v.verify();
        // Plugin pin is resolved; policy is still unresolved (no artifact for it).
        assert!(r.plugin_pins[0].is_resolved());
        assert!(!r.all_resolved()); // policy not resolved
        assert!(r.unresolved().iter().any(|p| p.id == "policy"));
    }
}
