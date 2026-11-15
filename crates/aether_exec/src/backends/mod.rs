// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! Backend registry: dispatches to the right `Backend` implementation
//! based on the `BackendKind` (Local / Slurm / Apptainer / SSH-batch /
//! agent-pool). Phase 7 (G7) introduces the four production backends;
//! the conformance tests verify that the contract surface is identical
//! across all backends (the MVR-04/05/12 cross-hardware substrate).

#![deny(unsafe_code)]
#![warn(missing_docs)]

mod registry;

pub use registry::{
    AgentPoolBackend, ApptainerBackend, BackendKind, SlurmBackend, SshBatchBackend,
    make_backend,
};
