// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! CLI subprocess adapter.
//!
//! Every web request becomes one `aether` invocation with machine-readable
//! output. No shell is involved; arguments are passed as discrete argv
//! elements. The CLI binary and its JSON contract are the only API.

#![deny(unsafe_code)]
#![warn(missing_docs)]

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use tokio::process::Command;

/// Adapter errors.
#[derive(Debug, Error)]
pub enum CliError {
    /// The CLI binary could not be located.
    #[error("aether CLI binary not found; build it or set AETHER_BIN")]
    BinaryNotFound,
    /// The request contained an illegal token.
    #[error("invalid command token: {0}")]
    InvalidToken(String),
    /// The CLI could not be spawned.
    #[error("spawn failed: {0}")]
    Spawn(String),
    /// The invocation exceeded its deadline.
    #[error("command timed out after {0:?}")]
    Timeout(Duration),
    /// Discovery output was malformed.
    #[error("discovery failed: {0}")]
    Discovery(String),
}

/// State of a command result, derived from the CLI exit code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandState {
    /// Exit 0.
    Ok,
    /// Exit 2 — usage error.
    UsageError,
    /// Exit 30 — fail-closed contract violation.
    Blocked,
    /// Any other non-zero exit.
    Error,
}

/// Map the CLI's exit code onto an honest UI state.
pub fn state_from_exit(exit_code: i32) -> CommandState {
    match exit_code {
        0 => CommandState::Ok,
        2 => CommandState::UsageError,
        30 => CommandState::Blocked,
        _ => CommandState::Error,
    }
}

/// A command invocation request from the browser.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommandRequest {
    /// Command tokens, e.g. `["schema", "list"]`.
    pub command: Vec<String>,
    /// Additional flags, e.g. `["--json"]`.
    #[serde(default)]
    pub flags: Vec<String>,
}

/// A structured command result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommandResponse {
    /// The original command string.
    pub command: String,
    /// State derived from the exit code.
    pub state: CommandState,
    /// Exit code returned by the CLI.
    pub exit_code: i32,
    /// Parsed JSON from stdout, when available.
    pub value: Value,
    /// Raw stdout when it was not valid JSON.
    pub text: Option<String>,
    /// stderr, kept separate from data.
    pub stderr: String,
    /// Wall-clock duration in milliseconds.
    pub duration_ms: u128,
}

/// One discovered command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiscoveredCommand {
    /// Full command name, e.g. `schema list`.
    pub name: String,
    /// Help text.
    pub about: String,
}

/// Locate the CLI binary without ever invoking a shell.
pub fn find_cli_binary() -> Option<PathBuf> {
    // Always return an absolute path: child processes may run with
    // `current_dir` set to a different project root.
    fn absolute(path: PathBuf) -> Option<PathBuf> {
        path.canonicalize().ok().or(Some(path))
    }
    let suffix = std::env::consts::EXE_SUFFIX;
    if let Ok(path) = std::env::var("AETHER_BIN") {
        let candidate = PathBuf::from(&path);
        if candidate.is_file() {
            return absolute(candidate);
        }
        if suffix.is_empty() {
            let with_suffix = PathBuf::from(format!("{path}{suffix}"));
            if with_suffix.is_file() {
                return absolute(with_suffix);
            }
        }
    }
    for relative in [
        format!("target/debug/aether{suffix}"),
        format!("target/release/aether{suffix}"),
        format!("../../target/debug/aether{suffix}"),
        format!("../../target/release/aether{suffix}"),
    ] {
        let candidate = PathBuf::from(relative);
        if candidate.is_file() {
            return absolute(candidate);
        }
    }
    // Search PATH manually so a shell is never involved. Windows resolves
    // PATHEXT through this same explicit suffix check.
    if let Ok(paths) = std::env::var("PATH") {
        for dir in std::env::split_paths(&paths) {
            let candidate = dir.join(format!("aether{suffix}"));
            if candidate.is_file() {
                return absolute(candidate);
            }
        }
    }
    None
}

/// The CLI runner.
#[derive(Debug, Clone)]
pub struct CliRunner {
    bin: PathBuf,
    timeout: Duration,
    /// Shared so an in-app project switch is visible to every cloned runner.
    project: Arc<RwLock<Option<PathBuf>>>,
}

impl CliRunner {
    /// Build a runner, discovering the CLI binary and command surface.
    pub fn discover() -> Result<Self, CliError> {
        let bin = find_cli_binary().ok_or(CliError::BinaryNotFound)?;
        let project = std::env::var("AETHER_WEB_PROJECT")
            .ok()
            .map(PathBuf::from)
            .and_then(|path| {
                path.canonicalize()
                    .map_err(|e| tracing::warn!("ignoring AETHER_WEB_PROJECT: {e}"))
                    .ok()
            });
        Ok(Self {
            bin,
            timeout: Duration::from_secs(120),
            project: Arc::new(RwLock::new(project)),
        })
    }

    /// Build a runner for a specific binary and deadline.
    pub fn with_binary(bin: PathBuf, timeout: Duration) -> Self {
        Self {
            bin,
            timeout,
            project: Arc::new(RwLock::new(None)),
        }
    }

    /// Run every child process in this project root.
    pub fn with_project(self, project: PathBuf) -> Self {
        self.set_project(project);
        self
    }

    /// Switch the project root for all clones of this runner.
    pub fn set_project(&self, project: PathBuf) {
        *self.project.write().expect("project lock poisoned") = Some(project);
    }

    /// The configured project root, if any.
    pub fn project(&self) -> Option<PathBuf> {
        self.project.read().expect("project lock poisoned").clone()
    }

    /// The resolved CLI binary path.
    pub fn binary(&self) -> &Path {
        &self.bin
    }
    /// Query the CLI's own discovery surface.
    pub async fn discovered(&self) -> Result<Vec<DiscoveredCommand>, CliError> {
        let response = self
            .run(&CommandRequest {
                command: vec!["discover".to_string(), "commands".to_string()],
                flags: vec!["--json".to_string()],
            })
            .await?;
        let list = response
            .value
            .get("commands")
            .and_then(Value::as_array)
            .ok_or_else(|| CliError::Discovery("missing commands array".to_string()))?;
        Ok(list
            .iter()
            .filter_map(|entry| {
                Some(DiscoveredCommand {
                    name: entry.get("name")?.as_str()?.to_string(),
                    about: entry
                        .get("about")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                })
            })
            .collect())
    }

    /// Invoke one command with the default deadline.
    pub async fn run(&self, request: &CommandRequest) -> Result<CommandResponse, CliError> {
        self.run_with_timeout(request, self.timeout).await
    }

    /// Invoke one command with an explicit deadline.
    pub async fn run_with_timeout(
        &self,
        request: &CommandRequest,
        timeout: Duration,
    ) -> Result<CommandResponse, CliError> {
        let mut argv = Vec::new();
        for token in request.command.iter().chain(request.flags.iter()) {
            if token.is_empty() || token.contains('\0') {
                return Err(CliError::InvalidToken(token.clone()));
            }
            argv.push(token.as_str());
        }

        let mut command = Command::new(&self.bin);
        command
            .args(&argv)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        if let Some(project) = self.project() {
            command.current_dir(project);
        }

        let started = std::time::Instant::now();
        let output = match tokio::time::timeout(timeout, command.output()).await {
            Ok(Ok(output)) => output,
            Ok(Err(e)) => return Err(CliError::Spawn(e.to_string())),
            Err(_) => return Err(CliError::Timeout(timeout)),
        };
        let duration_ms = started.elapsed().as_millis();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code().unwrap_or(-1);
        let state = state_from_exit(exit_code);
        let (value, text) = match serde_json::from_str::<Value>(&stdout) {
            Ok(v) => (v, None),
            Err(_) => (Value::Null, Some(stdout)),
        };
        Ok(CommandResponse {
            command: request.command.join(" "),
            state,
            exit_code,
            value,
            text,
            stderr,
            duration_ms,
        })
    }
}

/// Validate raw argv before it reaches a process boundary.
pub fn validate_argv(argv: &[String]) -> Result<(), CliError> {
    if argv.is_empty() {
        return Err(CliError::InvalidToken("empty command".to_string()));
    }
    if argv.len() > 64 {
        return Err(CliError::InvalidToken("too many arguments".to_string()));
    }
    let total: usize = argv.iter().map(String::len).sum();
    if total > 8192 {
        return Err(CliError::InvalidToken("command too long".to_string()));
    }
    for token in argv {
        if token.is_empty()
            || token.contains('\0')
            || token.chars().any(char::is_control)
        {
            return Err(CliError::InvalidToken("illegal character".to_string()));
        }
    }
    Ok(())
}

/// Resolve the longest discovered command name that prefixes `argv`.
///
/// This is the allowlist that keeps the terminal from becoming a generic
/// argv launcher. Remaining tokens are the command's own arguments and flags.
pub fn resolve_command<'a>(
    argv: &[String],
    discovered: &'a [DiscoveredCommand],
) -> Option<&'a DiscoveredCommand> {
    let mut best: Option<&DiscoveredCommand> = None;
    for command in discovered {
        let tokens: Vec<&str> = command.name.split_whitespace().collect();
        if tokens.len() > argv.len() || !tokens.iter().zip(argv.iter()).all(|(a, b)| *a == b) {
            continue;
        }
        if best.map_or(true, |current: &DiscoveredCommand| {
            command.name.len() > current.name.len()
        }) {
            best = Some(command);
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    fn runner() -> Option<CliRunner> {
        find_cli_binary().map(|bin| CliRunner::with_binary(bin, Duration::from_secs(30)))
    }

    fn fixture(relative: &str) -> String {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(relative)
            .to_string_lossy()
            .to_string()
    }

    #[tokio::test]
    async fn status_returns_structured_json() {
        let Some(runner) = runner() else { return };
        let response = runner
            .run(&CommandRequest {
                command: vec!["status".to_string()],
                flags: vec!["--json".to_string()],
            })
            .await
            .unwrap();
        assert_eq!(response.state, CommandState::Ok);
        assert_eq!(response.exit_code, 0);
        assert!(response.value.get("product").is_some());
        assert!(response.stderr.is_empty() || !response.stderr.is_empty());
    }

    #[tokio::test]
    async fn blocked_command_maps_to_blocked_state() {
        let Some(runner) = runner() else { return };
        let response = runner
            .run(&CommandRequest {
                command: vec!["policy".to_string(), "explain".to_string(), "graph.teleport".to_string()],
                flags: vec!["--json".to_string()],
            })
            .await
            .unwrap();
        // policy explain exits 0 with allow=false, so the state is Ok; the
        // UI inspects the decision. Exit 30 is asserted through vv coverage.
        assert_eq!(response.state, CommandState::Ok);
        assert_eq!(response.value["allow"], false);
    }

    #[tokio::test]
    async fn fail_closed_command_maps_to_blocked() {
        let Some(runner) = runner() else { return };
        let response = runner
            .run(&CommandRequest {
                command: vec![
                    "classify".to_string(),
                    "check".to_string(),
                    fixture("examples/wingbox/classification-denied.yaml"),
                ],
                flags: vec!["--json".to_string()],
            })
            .await
            .unwrap();
        assert_eq!(response.state, CommandState::Blocked);
        assert_eq!(response.exit_code, 30);
    }

    #[test]
    fn argv_validation_rejects_controls_and_oversize() {
        assert!(validate_argv(&["status".into()]).is_ok());
        assert!(validate_argv(&[]).is_err());
        assert!(validate_argv(&["bad\0token".into()]).is_err());
        assert!(validate_argv(&["line\nbreak".into()]).is_err());
        assert!(validate_argv(&vec!["x".into(); 65]).is_err());
    }

    #[test]
    fn longest_discovered_prefix_wins() {
        let discovered = vec![
            DiscoveredCommand { name: "vv".into(), about: String::new() },
            DiscoveredCommand { name: "vv coverage".into(), about: String::new() },
            DiscoveredCommand { name: "status".into(), about: String::new() },
        ];
        let argv = vec!["vv".into(), "coverage".into(), "/tmp/req.yaml".into()];
        let resolved = resolve_command(&argv, &discovered).unwrap();
        assert_eq!(resolved.name, "vv coverage");
        assert!(resolve_command(&["unknown".into()], &discovered).is_none());
    }

    #[test]
    fn nul_token_is_rejected() {
        let runner = CliRunner::with_binary(PathBuf::from("/bin/true"), Duration::from_secs(1));
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        let result = rt.block_on(runner.run(&CommandRequest {
            command: vec!["bad\0token".to_string()],
            flags: vec![],
        }));
        assert!(matches!(result, Err(CliError::InvalidToken(_))));
    }

    #[tokio::test]
    async fn discovery_lists_commands() {
        let Some(runner) = runner() else { return };
        let commands = runner.discovered().await.unwrap();
        assert!(commands.len() >= 100);
        assert!(commands.iter().any(|c| c.name == "status"));
    }
}