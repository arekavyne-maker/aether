// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! Aether exec service (G2).
//!
//! The exec service is the **only** place that spawns plugin processes
//! (SDP §4.4, §5.7, §6.2). Plugins run as separate OS processes,
//! supervised by the exec service, communicating over stdio JSON-RPC
//! (ADR-0015). The local backend (`LocalBackend`) implements the
//! scheduler SPI for the workstation case; Slurm / Apptainer / SSH-batch
//! land in G7.
//!
//! The **RunToken** is the security primitive: a scoped, short-lived
//! capability that grants a plugin process the right to read/write
//! specific CAS paths and call specific RPC methods. Tokens are
//! Ed25519-signed by the kernel; plugins refuse to operate without one
//! (I-12 fail-closed).

#![deny(unsafe_code)]
#![warn(missing_docs)]

pub mod backend;
pub mod run_token;
pub mod local;
pub mod backends;
pub mod supervisor;
pub mod handlers;
pub mod sandbox;

pub use backend::{Backend, ExitRecord, JobHandle, JobSpec, JobState, NodeCapabilities, Placement};
pub use run_token::{RunToken, RunTokenError, RunTokenResult, RunTokenScope, signed_token};
pub use local::LocalBackend;
pub use supervisor::{ProcessSpec, Supervisor, SupervisorConfig, SupervisorEvent};
pub use handlers::{
    CalculiXAdapterHandler, MeshBaselineHandler, OpenFOAMAdapterHandler, SharedState,
    StdioPluginHandler, monorepo_root,
};
pub use sandbox::{
    NetworkPolicy, ResourceLimits, SandboxError, SandboxPolicy, SandboxRequest,
};

/// Spawn a process under an explicit sandbox policy.
///
/// The caller builds the command and arguments; this function applies the
/// policy by setting the working directory and *replacing* the environment
/// with only the allowlisted keys. A request that violates the policy is
/// refused before the process starts. Kernel-level isolation remains an
/// adapter behind this function.
pub fn spawn_sandboxed(
    policy: &SandboxPolicy,
    request: &SandboxRequest,
    program: &str,
    args: &[String],
) -> Result<std::process::Child, SandboxError> {
    policy.check_request(request)?;
    let mut command = std::process::Command::new(program);
    command
        .args(args)
        .current_dir(&policy.working_directory)
        .env_clear();
    for (key, value) in std::env::vars() {
        if policy.env_allowlist.contains(&key) {
            command.env(key, value);
        }
    }
    command
        .spawn()
        .map_err(|e| SandboxError::Violation(format!("spawn {program}: {e}")))
}
