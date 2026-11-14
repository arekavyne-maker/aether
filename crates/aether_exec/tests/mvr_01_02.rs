// SPDX-License-Identifier: Aether-VPSL-1.0
// SPDX-FileCopyrightText: 2026 The Aether Consortium
//! MVR-01 / MVR-02 conformance: two independent `aether.physics.solver`
//! plugins (OpenFOAM and CalculiX) execute the same AWL workflow
//! identically, with only the handler reference changed (SDP §20).
//!
//! This is the executable form of the two-impl rule (ADR-0022) for the
//! physics.solver contract: a CFD-class and a structural-class plugin
//! both implement the contract, and the executor routes to whichever
//! the workflow declares.

use std::sync::Arc;

use aether_exec::handlers::{monorepo_root, OpenFOAMAdapterHandler, SharedState};
use aether_exec::run_token::KernelSigningKey;
use aether_workflow::awl::Awl;
use aether_workflow::checkpoint::InMemoryCheckpointStore;
use aether_workflow::dag::CompiledWorkflow;
use aether_workflow::execute::{Executor, ExecutorConfig, StepHandler};

/// The OpenFOAM and CalculiX reference adapters are registered as
/// StepHandlers. The handler's `name()` is the human-readable label;
/// both implement the same contract.
#[test]
fn both_handlers_implement_aether_physics_solver() {
    let root = monorepo_root();
    let key = Arc::new(KernelSigningKey::generate());
    let python = std::env::var("AETHER_PYTHON").unwrap_or_else(|_| "python3".to_string());

    let of = OpenFOAMAdapterHandler::new(&root, &python, Arc::clone(&key));
    // The CalculiX handler is added in this test file (the binary has
    // it via aether_exec::handlers but we re-export it here for the
    // conformance test).
    // Import the CalculiX handler.
    use aether_exec::handlers::CalculiXAdapterHandler;
    let cx = CalculiXAdapterHandler::new(&root, &python, Arc::clone(&key));

    // Both handlers declare the same contract id.
    assert_eq!(of.contract_id(), "aether.physics.solver");
    assert_eq!(cx.contract_id(), "aether.physics.solver");

    // Different names so the workflow can distinguish them when needed.
    assert_ne!(of.name(), cx.name());
    assert!(of.name().contains("openfoam") || of.name().contains("OpenFOAM"));
    assert!(cx.name().contains("calculix") || cx.name().contains("CalculiX"));
}

/// A workflow that runs the OpenFOAM handler and one that runs the
/// CalculiX handler both succeed; the AWL is unchanged.
#[test]
fn workflow_swappable_between_two_solvers() {
    let root = monorepo_root();
    let key = Arc::new(KernelSigningKey::generate());
    let python = std::env::var("AETHER_PYTHON").unwrap_or_else(|_| "python3".to_string());
    use aether_exec::handlers::CalculiXAdapterHandler;

    let awl_yaml = r#"
awl: "1.0"
steps:
  - id: solve
    type: run
    contract: aether.physics.solver
    plugin: org.example.solver
    plugin_version: "0.1.0"
"#;
    let awl = Awl::parse(awl_yaml).expect("parse AWL");
    let compiled = CompiledWorkflow::from_awl(&awl).expect("compile");

    // Run 1: OpenFOAM
    let mut ex1 = Executor::new(ExecutorConfig::default(), Box::new(InMemoryCheckpointStore::new()));
    ex1.register_handler(Arc::new(OpenFOAMAdapterHandler::new(&root, &python, Arc::clone(&key))));
    let r1 = ex1.execute(&compiled).expect("execute OF");
    assert!(r1.success(), "OpenFOAM run failed: {:?}", r1.status);
    let of_step = r1.step_results.get("solve").expect("solve step");
    assert_eq!(of_step.status, "succeeded");

    // Run 2: CalculiX
    let mut ex2 = Executor::new(ExecutorConfig::default(), Box::new(InMemoryCheckpointStore::new()));
    ex2.register_handler(Arc::new(CalculiXAdapterHandler::new(&root, &python, Arc::clone(&key))));
    let r2 = ex2.execute(&compiled).expect("execute CCX");
    assert!(r2.success(), "CalculiX run failed: {:?}", r2.status);
    let ccx_step = r2.step_results.get("solve").expect("solve step");
    assert_eq!(ccx_step.status, "succeeded");

    // The AWL is identical; only the handler differs. The two
    // implementations both satisfy the contract, which is the
    // MVR-01/02 substrate.
    let _ = SharedState::default(); // keep the import alive
}
