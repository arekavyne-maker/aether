// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! Typed graph errors (SDP §4.5). All variants are **fatal** to the
//! operation that produced them; a contract violation halts the run
//! (I-12). The error taxonomy is part of the kernel ABI and must not
//! change without an ADR.

#![deny(unsafe_code)]

use thiserror::Error;

pub type GraphResult<T> = Result<T, GraphError>;

/// Every failure mode of the graph engine. Each variant is a typed
/// contract obligation; the CLI maps them to the documented exit codes
/// (SDP §15.1).
#[derive(Debug, Error)]
pub enum GraphError {
    /// `ContractViolation` (fatal) — a typed command violated the graph
    /// schema catalog or the engine invariants (SDP §3.6).
    #[error("contract violation: {0}")]
    ContractViolation(String),

    /// `PolicyDenial` (fatal) — a policy engine refused the operation.
    /// Used by the policy service after G1.
    #[error("policy denial: {0}")]
    PolicyDenial(String),

    /// `DataMismatch` (fatal) — a referenced artifact could not be
    /// resolved or has a mismatched hash.
    #[error("data mismatch: {0}")]
    DataMismatch(String),

    /// `ValidationFailure` (fatal) — a node or edge failed schema
    /// validation. The error includes the path that failed.
    #[error("validation failure: {0}")]
    ValidationFailure(String),

    /// `ResourceUnavailable` (retryable) — the underlying storage SPI
    /// could not complete the operation; the caller may retry per policy.
    #[error("resource unavailable: {0}")]
    ResourceUnavailable(String),

    /// `NotFound` — a referenced node or edge does not exist.
    #[error("not found: {0}")]
    NotFound(String),

    /// `AlreadyExists` — an attempt to create a node/edge with a duplicate
    /// identity.
    #[error("already exists: {0}")]
    AlreadyExists(String),

    /// `Internal` — an unexpected internal error; recorded as evidence.
    #[error("internal: {0}")]
    Internal(String),
    /// `NotImplemented` — a feature of the trait is not yet implemented
    /// for this backend (e.g., the synchronous trait method on the
    /// async-only PostgreSQL backend; team-mode sites use the async API).
    #[error("not implemented: {0}")]
    NotImplemented(String),
}
