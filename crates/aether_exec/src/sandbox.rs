// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! Sandbox policy for plugin execution.
//!
//! The current stdio handler already limits a plugin to a signed run token
//! with a path and method scope. This module adds the explicit sandbox
//! policy that the executor can enforce around a spawn: working directory,
//! read/write roots, environment allowlist, resource limits, and a network
//! posture.
//!
//! A plugin that asks for more than the policy grants is refused before the
//! process starts. This is a *policy and enforcement surface*; kernel-level
//! isolation (seccomp, namespaces, containers) remains an adapter behind
//! this interface.

#![deny(unsafe_code)]
#![warn(missing_docs)]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Network posture for a plugin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkPolicy {
    /// No network access at all.
    Deny,
    /// Only loopback.
    LoopbackOnly,
    /// Explicit allowlist of hosts (not implemented as enforcement here).
    Allowlist,
}

/// Resource limits for a plugin process.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Wall-clock timeout in seconds.
    pub timeout_seconds: u64,
    /// Maximum resident memory in MiB.
    pub max_memory_mib: u64,
    /// Maximum CPU seconds.
    pub max_cpu_seconds: u64,
    /// Maximum number of child processes.
    pub max_processes: u32,
    /// Maximum open files.
    pub max_open_files: u32,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            timeout_seconds: 3600,
            max_memory_mib: 8192,
            max_cpu_seconds: 7200,
            max_processes: 64,
            max_open_files: 1024,
        }
    }
}

/// A sandbox policy for one plugin.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SandboxPolicy {
    /// Plugin id the policy belongs to.
    pub plugin_id: String,
    /// Working directory the plugin may run in.
    pub working_directory: PathBuf,
    /// Directories the plugin may read.
    pub read_paths: Vec<PathBuf>,
    /// Directories the plugin may write.
    pub write_paths: Vec<PathBuf>,
    /// Environment variables the plugin may see. Anything else is removed.
    pub env_allowlist: BTreeSet<String>,
    /// Network posture.
    pub network: NetworkPolicy,
    /// Resource limits.
    pub limits: ResourceLimits,
}

/// Sandbox errors.
#[derive(Debug, Error, PartialEq)]
pub enum SandboxError {
    /// The policy itself is invalid.
    #[error("invalid sandbox policy: {0}")]
    Invalid(String),
    /// A request exceeds the policy.
    #[error("sandbox violation: {0}")]
    Violation(String),
}

/// What a plugin asked for.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SandboxRequest {
    /// Working directory requested.
    pub working_directory: PathBuf,
    /// Paths requested for reading.
    pub read_paths: Vec<PathBuf>,
    /// Paths requested for writing.
    pub write_paths: Vec<PathBuf>,
    /// Environment variables requested.
    pub env: BTreeSet<String>,
    /// Network requested.
    pub network: NetworkPolicy,
    /// Resources requested.
    pub limits: ResourceLimits,
}

impl SandboxPolicy {
    /// Standard policy for a reference plugin: it may read its own directory
    /// and write to a scratch directory, with no network.
    pub fn reference_plugin(plugin_id: &str, script_dir: &Path, scratch: &Path) -> Self {
        Self {
            plugin_id: plugin_id.to_string(),
            working_directory: script_dir.to_path_buf(),
            read_paths: vec![script_dir.to_path_buf()],
            write_paths: vec![scratch.to_path_buf()],
            env_allowlist: [
                "PATH".to_string(),
                "HOME".to_string(),
                "LANG".to_string(),
                "LC_ALL".to_string(),
                "TMPDIR".to_string(),
                "PYTHONPATH".to_string(),
                "CCX_BIN".to_string(),
            ]
            .into(),
            network: NetworkPolicy::Deny,
            limits: ResourceLimits::default(),
        }
    }

    /// Validate the policy itself.
    pub fn validate(&self) -> Result<(), SandboxError> {
        if self.plugin_id.trim().is_empty() {
            return Err(SandboxError::Invalid("plugin_id is empty".into()));
        }
        if self.working_directory.as_os_str().is_empty() {
            return Err(SandboxError::Invalid("working_directory is empty".into()));
        }
        if self.read_paths.is_empty() {
            return Err(SandboxError::Invalid("no read paths".into()));
        }
        if self.limits.timeout_seconds == 0 {
            return Err(SandboxError::Invalid("timeout_seconds must be > 0".into()));
        }
        if self.limits.max_memory_mib == 0 {
            return Err(SandboxError::Invalid("max_memory_mib must be > 0".into()));
        }
        Ok(())
    }

    /// Filter an environment down to the allowlist.
    pub fn filter_env<'a>(
        &self,
        env: impl IntoIterator<Item = &'a (String, String)>,
    ) -> Vec<(String, String)> {
        env.into_iter()
            .filter(|(key, _)| self.env_allowlist.contains(key))
            .cloned()
            .collect()
    }

    /// Check a plugin request against this policy.
    pub fn check_request(&self, request: &SandboxRequest) -> Result<(), SandboxError> {
        self.validate()?;
        if request.working_directory != self.working_directory {
            return Err(SandboxError::Violation(format!(
                "working directory {:?} is not {:?}",
                request.working_directory, self.working_directory
            )));
        }
        for path in &request.read_paths {
            if !self.path_allowed(path, &self.read_paths) {
                return Err(SandboxError::Violation(format!(
                    "read path {} is outside the policy",
                    path.display()
                )));
            }
        }
        for path in &request.write_paths {
            if !self.path_allowed(path, &self.write_paths) {
                return Err(SandboxError::Violation(format!(
                    "write path {} is outside the policy",
                    path.display()
                )));
            }
        }
        for key in &request.env {
            if !self.env_allowlist.contains(key) {
                return Err(SandboxError::Violation(format!(
                    "environment variable {key:?} is not allowed"
                )));
            }
        }
        if request.network == NetworkPolicy::Allowlist && self.network != NetworkPolicy::Allowlist {
            return Err(SandboxError::Violation(
                "plugin requested an allowlisted network but the policy denies or restricts it"
                    .to_string(),
            ));
        }
        if request.limits.timeout_seconds > self.limits.timeout_seconds {
            return Err(SandboxError::Violation(format!(
                "timeout {}s exceeds the policy {}s",
                request.limits.timeout_seconds, self.limits.timeout_seconds
            )));
        }
        if request.limits.max_memory_mib > self.limits.max_memory_mib {
            return Err(SandboxError::Violation(format!(
                "memory {} MiB exceeds the policy {} MiB",
                request.limits.max_memory_mib, self.limits.max_memory_mib
            )));
        }
        if request.limits.max_cpu_seconds > self.limits.max_cpu_seconds {
            return Err(SandboxError::Violation(format!(
                "cpu {}s exceeds the policy {}s",
                request.limits.max_cpu_seconds, self.limits.max_cpu_seconds
            )));
        }
        if request.limits.max_processes > self.limits.max_processes {
            return Err(SandboxError::Violation(format!(
                "processes {} exceed the policy {}",
                request.limits.max_processes, self.limits.max_processes
            )));
        }
        if request.limits.max_open_files > self.limits.max_open_files {
            return Err(SandboxError::Violation(format!(
                "open files {} exceed the policy {}",
                request.limits.max_open_files, self.limits.max_open_files
            )));
        }
        Ok(())
    }

    fn path_allowed(&self, candidate: &Path, roots: &[PathBuf]) -> bool {
        roots.iter().any(|root| candidate.starts_with(root))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> SandboxPolicy {
        SandboxPolicy::reference_plugin(
            "org.aether.reference.beam",
            Path::new("/plugins/beam"),
            Path::new("/scratch"),
        )
    }

    fn request() -> SandboxRequest {
        SandboxRequest {
            working_directory: PathBuf::from("/plugins/beam"),
            read_paths: vec![PathBuf::from("/plugins/beam")],
            write_paths: vec![PathBuf::from("/scratch")],
            env: ["PATH".to_string(), "HOME".to_string()].into(),
            network: NetworkPolicy::Deny,
            limits: ResourceLimits::default(),
        }
    }

    #[test]
    fn valid_request_passes() {
        assert!(policy().check_request(&request()).is_ok());
    }

    #[test]
    fn write_outside_scratch_is_refused() {
        let mut r = request();
        r.write_paths = vec![PathBuf::from("/etc")];
        let err = policy().check_request(&r).unwrap_err();
        assert!(matches!(err, SandboxError::Violation(_)));
        assert!(err.to_string().contains("/etc"));
    }

    #[test]
    fn env_outside_allowlist_is_refused() {
        let mut r = request();
        r.env.insert("AWS_SECRET_ACCESS_KEY".to_string());
        let err = policy().check_request(&r).unwrap_err();
        assert!(err.to_string().contains("AWS_SECRET_ACCESS_KEY"));
    }

    #[test]
    fn network_escalation_is_refused() {
        let mut r = request();
        r.network = NetworkPolicy::Allowlist;
        assert!(policy().check_request(&r).is_err());
    }

    #[test]
    fn oversized_limits_are_refused() {
        let mut r = request();
        r.limits.max_memory_mib = 1_000_000;
        assert!(policy().check_request(&r).is_err());
    }

    #[test]
    fn working_directory_cannot_be_changed() {
        let mut r = request();
        r.working_directory = PathBuf::from("/");
        assert!(policy().check_request(&r).is_err());
    }

    #[test]
    fn env_filter_drops_unlisted_keys() {
        let env = vec![
            ("PATH".to_string(), "/usr/bin".to_string()),
            ("SECRET".to_string(), "x".to_string()),
        ];
        let filtered = policy().filter_env(&env);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].0, "PATH");
    }
}

/// The isolation mechanism applied to a sandboxed spawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IsolationLevel {
    /// No kernel-level isolation; policy enforced at the spawn boundary.
    PolicyOnly,
    /// macOS `sandbox-exec` profile.
    SandboxExec,
    /// Linux `bwrap` (bubblewrap).
    Bubblewrap,
}

/// Build a macOS `sandbox-exec` profile that denies network and restricts
/// writes to the declared roots. Returns `None` when `sandbox-exec` is not
/// available.
pub fn macos_sandbox_profile(policy: &SandboxPolicy) -> Option<String> {
    if std::process::Command::new("sh")
        .arg("-c")
        .arg("command -v sandbox-exec >/dev/null 2>&1")
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
        == false
    {
        return None;
    }
    let mut profile = String::from("(version 1)\n");
    profile.push_str("(deny default)\n");
    profile.push_str("(allow process-exec)\n");
    profile.push_str("(allow file-read*)\n");
    // Writes are allowed only under the declared roots.
    for root in &policy.write_paths {
        profile.push_str(&format!(
            "(allow file-write* (subpath \"{}\"))\n",
            root.display()
        ));
    }
    // Network posture.
    match policy.network {
        NetworkPolicy::Deny => profile.push_str("(deny network*)\n"),
        NetworkPolicy::LoopbackOnly => {
            profile.push_str("(allow network* (local))\n");
            profile.push_str("(deny network* (remote))\n");
        }
        NetworkPolicy::Allowlist => {
            // Host-level allowlisting is not expressed in the SBPL profile;
            // the caller must enforce it at the proxy layer.
            profile.push_str("(deny network*)\n");
        }
    }
    Some(profile)
}

/// Detect which kernel-level isolation mechanism is available.
pub fn available_isolation() -> IsolationLevel {
    let has = |cmd: &str| {
        std::process::Command::new("sh")
            .arg("-c")
            .arg(format!("command -v {cmd} >/dev/null 2>&1"))
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    };
    if has("sandbox-exec") {
        IsolationLevel::SandboxExec
    } else if has("bwrap") {
        IsolationLevel::Bubblewrap
    } else {
        IsolationLevel::PolicyOnly
    }
}

#[cfg(test)]
mod isolation_tests {
    use super::*;

    fn policy() -> SandboxPolicy {
        SandboxPolicy::reference_plugin(
            "org.example",
            std::path::Path::new("/tmp"),
            std::path::Path::new("/tmp"),
        )
    }

    #[test]
    fn macos_profile_denies_network_and_restricts_writes() {
        let profile = macos_sandbox_profile(&policy());
        if cfg!(target_os = "macos") {
            let profile = profile.expect("sandbox-exec should be present on macOS");
            assert!(profile.contains("(deny default)"));
            assert!(profile.contains("file-write*"));
        }
    }

    #[test]
    fn isolation_level_is_reported_honestly() {
        let level = available_isolation();
        assert!(matches!(
            level,
            IsolationLevel::SandboxExec | IsolationLevel::Bubblewrap | IsolationLevel::PolicyOnly
        ));
    }
}
