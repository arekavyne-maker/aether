// SPDX-License-Identifier: Aether-VPSL-1.0
// SPDX-FileCopyrightText: 2026 The Aether Consortium
//! Aether Workflow Language (AWL) v1 — compile + execute (G2).
//!
//! AWL is the platform's declarative workflow surface (SDP §4.2, §17, §18).
//! An AWL document is a YAML file declaring a DAG of steps, each with
//! typed inputs, outputs, and resource requirements. G2 ships the v1
//! executor: parameter sweeps, retries, checkpoints, and a deterministic
//! execution trace.
//!
//! Phase 0/G1 was the **declarative form** (`workflows/paper-run/awl/*.yaml`).
//! G2 makes it **executable**. The execution model is:
//!
//! 1. **Compile**: parse AWL, validate step types, build a typed DAG.
//! 2. **Plan**: resolve parameter spaces (sweeps, DOEs) into a sequence
//!    of step instances.
//! 3. **Execute**: run each step; capture provenance records; checkpoint
//!    after each step (Merkle hash).
//! 4. **Resume**: a `resume` call picks up at the last checkpoint.
//!
//! Workflow steps are typed; the kernel does not know what a step *does*,
//! only that it has a deterministic identity and produces outputs. In
//! G2 the step types are still primitive (`run`, `sweep`, `gate`); the
//! full step-type plugin registry lands in G3 (when at least two
//! production plugins exist per the two-impl rule).

#![deny(unsafe_code)]
#![warn(missing_docs)]

pub mod awl;
pub mod dag;
pub mod execute;
pub mod checkpoint;

pub use awl::{Awl, AwlError, AwlResult, Step, StepType, ParameterSpace, PlacementHint};
pub use dag::{CompiledWorkflow, StepInstance, StepStatus};
pub use execute::{ExecutionResult, Executor, ExecutorConfig};
pub use checkpoint::{Checkpoint, CheckpointStore, CheckpointError, CheckpointResult};
