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
pub mod help;
pub mod files;
pub mod atlas;
pub mod jobs;
pub mod api;
pub mod assets;

pub use cli::{CliRunner, CommandRequest, CommandResponse, CommandState, CliError};
pub use auth::TokenAuth;
pub use catalog::{Catalog, CommandSpec, ArgSpec, ArgKind, CommandKind};

/// Serve the embedded console and API on an existing loopback listener.
///
/// The desktop shell binds `127.0.0.1:0` itself so it knows the chosen port,
/// then hands the listener here. The canonical CLI remains the only backend.
pub async fn serve(
    listener: tokio::net::TcpListener,
    token: TokenAuth,
    runner: CliRunner,
) -> anyhow::Result<()> {
    axum::serve(listener, api::router(token, runner)).await?;
    Ok(())
}
