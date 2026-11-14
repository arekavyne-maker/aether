// SPDX-License-Identifier: Aether-VPSL-1.0
// SPDX-FileCopyrightText: 2026 The Aether Consortium
//! Aether Engineering Graph engine — Phase 0 walking skeleton (b).
//!
//! This module contains the *generic mechanics* of the graph: identity
//! minting, append-only event log, materialized views, and time-travel
//! queries. The kernel's full graph engine lands in Phase 1 (G1) and
//! extends this skeleton with the full AQL query language and PostgreSQL
//! team-mode backend (SDP §3.5, ADR-0018).
//!
//! Phase 0 deliverable: validate SQLite at 10M-node event-sourced scale.

#![deny(unsafe_code)]
#![warn(missing_docs)]

pub mod engine;
pub mod event;
pub mod scale;
pub mod storage;
pub mod aql;
pub mod error;
pub mod types;
pub mod catalog;
#[cfg(feature = "postgres")]
pub mod postgres_storage;

pub use engine::{GraphEngine, GraphSnapshot, SnapshotAt};
pub use error::{GraphError, GraphResult};
pub use event::{Event, EventKind, NodeStateChanged, NodeTombstoned, NodeUpserted, EdgeAdded, EdgeRemoved};
pub use scale::{run_scale_test, ScaleTestConfig, ScaleTestResult};
pub use storage::{Storage, StorageBackend, SqliteStorage, OpenMode, MappedRow};
pub use types::{NodeId, EdgeId, EventId, Node, Edge, NodeState, NodeTypeSpec, EdgeTypeSpec, Cardinality, Properties};
