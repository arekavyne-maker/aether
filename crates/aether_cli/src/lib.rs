// SPDX-License-Identifier: Aether-VPSL-1.0
// SPDX-FileCopyrightText: 2026 The Aether Consortium
//! Library exports for the Aether CLI (G1).
//!
//! Exposes:
//! - the CLI command tree (so SDKs and tests can introspect it)
//! - the TUF verifier (Phase 0)
//! - shared command-result types

#![deny(unsafe_code)]
#![warn(missing_docs)]

pub mod tuf;
pub mod commands;
pub mod engine_bridge;
pub mod spine_handlers;
pub mod spine;
pub mod workflow_cmd;
pub mod g3_commands;
pub mod g4_commands;
pub mod g7_commands;

pub use tuf::verify::{
    canonical_json, canonical_payload, verify_bundle, Check, Metadata, SignatureEntry, VerifySummary,
};
