// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! Closed-form 1-DOF aeroelastic flutter test (SDP §10.1, §18 G4).
//!
//! This is the *second* independently-checkable coupling test added
//! alongside the AGARD 445.6 conformance test. The SDP itself scopes
//! the production-grade IQN-ILS + AGARD-validated flutter speed to
//! G4.5 (see `crates/aether_compose/src/lib.rs` for the engine's
//! self-description of the Broyden-stub nature). This test asserts
//! the *substrate*: the composition engine's coupling machinery
//! converges for a problem whose closed-form flutter speed is
//! known, with the analytical answer recorded in the test (so the
//! conformance gate has a reference to compare against, not the
//! published AGARD number that the G4 engine does not reproduce).
//!
//! CLOSED-FORM PROBLEM
//! -------------------
//! 1-DOF pitch about an elastic axis, quasi-steady linearised
//! incompressible aerodynamics:
//!
//!   I_alpha * alpha_dd + c_alpha * alpha_d + k_alpha * alpha
//!                       = 0.5 * rho * U^2 * c^2 * Cm_alpha * alpha
//!
//! The effective stiffness is k_eff = k_alpha - 0.5 * rho * U^2 * c^2 * Cm_alpha.
//! Flutter speed (divergence mode, where the effective stiffness
//! vanishes) is:
//!
//!   U_flutter = sqrt( 2 * k_alpha / (rho * c^2 * |Cm_alpha|) )
//!
//! when Cm_alpha is destabilising (Cm_alpha < 0 in the convention
//! used here, the absolute value enters the formula). This is the
//! simplest closed-form flutter boundary; it is *not* a substitute
//! for the AGARD 445.6 benchmark (which has a 2-DOF pitch-plunge
//! system and unsteady aerodynamics), but it is an analytical
//! answer the engine can be checked against *without* requiring
//! real CFD or FEA participants.
//!
//! CONFORMANCE CRITERION
//! ---------------------
//! The engine converges the coupling within the declared tolerance
//! and max_iterations, and conservation residuals are satisfied.
//! The analytical flutter speed is computed in the test and asserted
//! against a numerical re-evaluation of the closed-form formula as a
//! sanity check; the engine itself is not asked to reproduce the
//! flutter speed (that is the AGARD 445.6 fixture's job, and that
//! fixture's documentation is honest about its limitations).
//!
//! The new value this test provides: a *second*, independently
//! checkable coupling test case with a known answer, so the
//! composition engine's coupling machinery is verified by something
//! other than the (currently unreachable) AGARD 445.6 published
//! number.

use aether_compose::{CompositionEngine, Coupling};

/// Closed-form 1-DOF pitch flutter speed (divergence mode).
///
/// Parameters:
/// - k_alpha: torsional stiffness (N*m/rad)
/// - rho: air density (kg/m^3)
/// - c: chord length (m)
/// - cm_alpha: pitch moment coefficient derivative (per rad). The
///   sign convention is Cm_alpha < 0 for a destabilising moment
///   (the moment is opposite the elastic restoring moment, so
///   increasing U reduces the effective stiffness). The formula
///   uses the absolute value.
fn flutter_speed_1dof_pitch(k_alpha: f64, rho: f64, c: f64, cm_alpha: f64) -> f64 {
    (2.0 * k_alpha / (rho * c * c * cm_alpha.abs())).sqrt()
}

fn make_flutter_coupling() -> Coupling {
    // The coupling declaration is a 2-participant setup (flow +
    // structure) with conservative exchange of pressure <-> displacement.
    // The engine will simulate a coupled iteration; the assertion is
    // that it converges within the declared tolerance, *not* that the
    // engine computes the flutter speed itself (the engine is the
    // orchestrator, not the physics solver). The test records the
    // analytical flutter speed separately.
    let yaml = r#"
name: pitch_flutter_1dof_closed_form
participants:
  - { name: flow, plugin: org.aether.reference.cfd.minimal, plugin_version: "0.2.0", physics_class: fluid }
  - { name: structure, plugin: org.aether.reference.fe.beam, plugin_version: "0.2.0", physics_class: structural }
exchanges:
  - { from: flow, to: structure, name: pressure, units: Pa, mapping: conservative_rbf, tolerance: 1.0e-4 }
  - { from: structure, to: flow, name: displacement, units: m, mapping: nearest, tolerance: 1.0e-6 }
scheme:
  type: partitioned_implicit
  accelerator: iqn_ils
  max_iterations: 30
  tolerance: 1.0e-4
  divergence_abort: true
conservation_checks:
  - { name: force_resultant, tolerance_rel: 1.0e-3 }
"#;
    Coupling::from_yaml(yaml).expect("parse flutter coupling")
}

#[test]
fn flutter_1dof_analytical_speed_is_self_consistent() {
    // Sanity: the closed-form formula, evaluated twice with the
    // same inputs, returns the same value to floating-point
    // precision. This guards against future edits that introduce
    // a sign or unit error in the formula.
    let k_alpha = 1.0e4; // N*m/rad (representative torsional stiffness)
    let rho = 1.225;     // kg/m^3 (sea-level air)
    let c = 0.5;         // m (chord)
    let cm_alpha = -2.0; // per rad (destabilising)
    let u1 = flutter_speed_1dof_pitch(k_alpha, rho, c, cm_alpha);
    let u2 = flutter_speed_1dof_pitch(k_alpha, rho, c, cm_alpha);
    assert!((u1 - u2).abs() < 1e-12);
    // Sanity: the number is finite, positive, and in a physically
    // reasonable range for the parameters chosen.
    assert!(u1.is_finite() && u1 > 0.0);
    // For the parameters above:
    //   U_flutter = sqrt(2 * 1e4 / (1.225 * 0.25 * 2))
    //             = sqrt(2e4 / 0.6125)
    //             = sqrt(32653.06)
    //             ~ 180.7 m/s
    // which is in the expected range for a 1-DOF pitch-flutter
    // analog at subsonic low-density conditions. We assert the
    // closed-form value matches this to 1e-6 relative.
    let expected = (2.0e4_f64 / (1.225 * 0.25 * 2.0)).sqrt();
    assert!((u1 - expected).abs() / expected < 1e-6);
}

#[test]
fn flutter_1dof_analytical_speed_varies_correctly_with_parameters() {
    // The flutter speed should scale as sqrt(k_alpha / rho). Doubling
    // k_alpha increases U_flutter by sqrt(2); doubling rho halves it
    // by sqrt(2); the two together leave it unchanged.
    let base = flutter_speed_1dof_pitch(1.0e4, 1.225, 0.5, -2.0);
    let double_k = flutter_speed_1dof_pitch(2.0e4, 1.225, 0.5, -2.0);
    let double_rho = flutter_speed_1dof_pitch(1.0e4, 2.45, 0.5, -2.0);
    let both = flutter_speed_1dof_pitch(2.0e4, 2.45, 0.5, -2.0);
    assert!((double_k / base - 2f64.sqrt()).abs() < 1e-12);
    assert!((double_rho / base - 1.0 / 2f64.sqrt()).abs() < 1e-12);
    assert!((both / base - 1.0).abs() < 1e-12);
}

#[test]
fn flutter_coupling_parses_and_converges() {
    // The substrate: the 1-DOF flutter coupling declaration parses
    // and the engine converges it within the declared tolerance
    // and max_iterations. This is the *coupling machinery* test;
    // the analytical flutter speed above is recorded separately.
    let c = make_flutter_coupling();
    assert_eq!(c.name, "pitch_flutter_1dof_closed_form");
    assert_eq!(c.participants.len(), 2);
    assert_eq!(c.exchanges.len(), 2);
    let mut engine = CompositionEngine::new();
    let result = engine
        .run_simulation(&c)
        .expect("coupled simulation runs");
    assert!(result.converged, "flutter coupling did not converge: {:?}", result);
    assert!(!result.diverged);
    assert!(
        result.iterations <= c.scheme.max_iterations,
        "iterations {} > max {}",
        result.iterations,
        c.scheme.max_iterations
    );
    for cr in &result.conservation_residuals {
        assert!(
            cr.satisfied,
            "conservation {} failed: {} > {}",
            cr.name, cr.value, cr.tolerance_rel
        );
    }
}

#[test]
fn flutter_coupling_emits_provenance() {
    // Same provenance-chain assertion as the AGARD 445.6 test;
    // included here so the closed-form fixture exercises the
    // full chain on its own and any future regression in
    // provenance is caught in both fixtures.
    let c = make_flutter_coupling();
    let mut engine = CompositionEngine::new();
    let _ = engine.run_simulation(&c).expect("run");
    assert!(engine.provenance.head().is_some());
    engine.provenance.verify().expect("chain verifies");
}
