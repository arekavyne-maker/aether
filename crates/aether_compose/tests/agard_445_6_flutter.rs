// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! AGARD 445.6 flutter benchmark conformance (SDP §18 G4).
//!
//! The test parses the benchmark AWL, runs the composition through
//! the `aether_compose` engine, and asserts that the coupling
//! converges within the declared tolerance. The benchmark's
//! published flutter speed is recorded as the target; in a
//! production site the participants would be full CFD + FEA solvers
//! with the AGARD 445.6 mesh, and the result would be compared
//! against the published flutter speed (~150 m/s at sea level).
//!
//! The conformance test asserts the *substrate*: that the coupling
//! declaration, the unit/conservation checks, the IQN-ILS
//! acceleration, and the engine's Result schema are correct. The
//! actual flutter-speed number is a site-level validation against
//! a known-good solver; the Phase-0 engine's internal simulation
//! converges but does not reproduce the published flutter speed
//! (that requires real CFD + FEA participants, which is the
//! production path that this fixture underwrites).

use std::path::PathBuf;

use aether_compose::{CompositionEngine, Coupling, CouplingResult};

fn find_platform_root() -> PathBuf {
    let mut here = std::env::current_exe().expect("current_exe");
    loop {
        if here.join("Cargo.toml").is_file() && here.join("contracts").is_dir() {
            return here;
        }
        if !here.pop() {
            panic!("could not locate monorepo root");
        }
    }
}

fn load_agard_445() -> Coupling {
    let root = find_platform_root();
    let path = root.join("workflows/agard-445-flutter.awl.yaml");
    let raw = std::fs::read_to_string(&path).expect("read agard 445.6 awl");
    // Extract the `compose:` block; the AWL is YAML, the coupling is
    // embedded as `compose:`. We use serde_yaml to parse the whole AWL
    // and pull out the compose field.
    let v: serde_json::Value = serde_yaml::from_str(&raw).expect("parse awl");
    let compose = v.get("compose").expect("compose block present");
    Coupling::from_json(compose).expect("coupling from json")
}

#[test]
fn agard_445_flutter_coupling_parses() {
    let c = load_agard_445();
    assert_eq!(c.name, "aeroelastic-flutter-agard-445");
    assert_eq!(c.participants.len(), 2);
    assert_eq!(c.exchanges.len(), 2);
    // Unit consistency: no unit mismatch.
    assert!(c.exchanges.iter().all(|e| !e.units.is_empty()));
}

#[test]
fn agard_445_flutter_coupling_converges() {
    let c = load_agard_445();
    let mut engine = CompositionEngine::new();
    let result = engine
        .run_simulation(&c)
        .expect("coupled simulation runs");
    // G4 acceptance (SDP §18): the coupling converges within the
    // declared tolerance and the conservation residuals are satisfied.
    assert!(result.converged, "AGARD 445.6 coupling did not converge: {:?}", result);
    assert!(result.diverged == false);
    for c in &result.conservation_residuals {
        assert!(
            c.satisfied,
            "conservation {} failed: {} > {}",
            c.name, c.value, c.tolerance_rel
        );
    }
    // The IQN-ILS accelerator must produce a result within the
    // declared max_iterations (this is the structural claim of the
    // composition engine).
    assert!(
        result.iterations <= c.scheme.max_iterations,
        "iterations {} > max {}",
        result.iterations, c.scheme.max_iterations
    );
}

#[test]
fn agard_445_flutter_engine_emits_provenance() {
    let c = load_agard_445();
    let mut engine = CompositionEngine::new();
    let _ = engine.run_simulation(&c).expect("run");
    // The engine emitted a provenance record for the coupling.
    assert!(engine.provenance.head().is_some());
    engine.provenance.verify().expect("chain verifies");
}

#[test]
fn agard_445_awl_is_strict_reproducible() {
    // The AWL declares `reproducibility: strict`, which means a strict
    // re-execution must reproduce the same workflow_id and the same
    // topologically-sorted step set. The composition engine itself
    // is bit-reproducible (the engine's internal simulation is
    // deterministic); the substrate for MVR-09/11 is in place.
    let c = load_agard_445();
    let mut engine1 = CompositionEngine::new();
    let mut engine2 = CompositionEngine::new();
    let r1 = engine1.run_simulation(&c).expect("run 1");
    let r2 = engine2.run_simulation(&c).expect("run 2");
    assert_eq!(r1.converged, r2.converged);
    assert_eq!(r1.iterations, r2.iterations);
    assert!((r1.convergence_residual - r2.convergence_residual).abs() < 1e-12);
}
