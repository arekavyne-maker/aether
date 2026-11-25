// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! Aether local web interface.
//!
//! This crate is a thin adapter over the canonical CLI. It never
//! reimplements engineering logic: every action spawns the `aether` binary
//! with machine-readable output and maps its exit code into a UI state.
//! The CLI source and behaviour are untouched.

#![deny(unsafe_code)]
#![warn(missing_docs)]

pub mod cli;
pub mod auth;
pub mod catalog;
pub mod api;
pub mod assets;

pub use cli::{CliRunner, CommandRequest, CommandResponse, CommandState, CliError};
pub use auth::TokenAuth;
pub use catalog::{Catalog, CommandSpec, ArgSpec, ArgKind, CommandKind};