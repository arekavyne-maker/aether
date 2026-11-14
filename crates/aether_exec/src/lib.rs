// SPDX-License-Identifier: Aether-VPSL-1.0
// SPDX-FileCopyrightText: 2026 The Aether Consortium
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

pub use backend::{Backend, ExitRecord, JobHandle, JobSpec, JobState, NodeCapabilities, Placement};
pub use run_token::{RunToken, RunTokenError, RunTokenResult, RunTokenScope, signed_token};
pub use local::LocalBackend;
pub use supervisor::{ProcessSpec, Supervisor, SupervisorConfig, SupervisorEvent};
pub use handlers::{
    CalculiXAdapterHandler, MeshBaselineHandler, OpenFOAMAdapterHandler, SharedState,
    StdioPluginHandler, monorepo_root,
};
