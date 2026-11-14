# calculix-adapter — production-track native binary adapter

This adapter is a **real production-track structural FEA adapter** for
CalculiX 2.23. It implements the `aether.physics.solver` contract
surface (the subset used by Phase 0–2) and produces a verified
result against a closed-form analytical solution.

## What this adapter does

For an LE1 cantilever problem spec, the adapter:

1. **Validates** the input `ProblemSpec` against the contract
   schema subset (problem_id, physics_class, geometry, boundary
   conditions, solver_settings, the `case` field, the ccx binary
   presence).
2. **Builds a real `cantilever.inp` deck** from the problem
   parameters. The deck has N B31 elements along the length
   (default 8), a `*BEAM SECTION, SECTION=RECT` cross-section, an
   isotropic linear-elastic material, encastre root (DOFs 1–6
   clamped at x=0), and a tip point load in the load direction.
3. **Subprocesses the real `ccx_2.23` binary** in a per-call
   temporary directory (the SDP §5.6 native binary adapter
   pattern). The binary is found on `PATH` (or `$CCX_BIN`).
4. **Parses the real `.frd` output** (column-based parser that
   handles the fixed-width format correctly, including the
   sign-collapsing whitespace edge case).
5. **Reports the tip deflection** against the closed-form
   analytical formula `P L³ / (3 E I)`, where `I = W H³ / 12` for
   a rectangular cross section. The relative error is checked
   against a 5% tolerance.

## Validated case

| Parameter | Value |
|---|---|
| Length L | 1.0 m |
| Cross-section | 0.01 m × 0.01 m (W × H) |
| Material | Isotropic linear elastic, E = 200 GPa, ν = 0.3 |
| Load | -1000 N at the tip, perpendicular to the beam axis |
| Mesh | 8 B31 elements (Timoshenko beam) |
| Boundary | Encastre at x = 0 (DOFs 1–6 clamped) |

**Analytical tip deflection:** `v_tip = P L³ / (3 E I) = 2.0 m`

**FEA (8 B31 elements):** ~1.99 m (0.5% relative error)

The same problem spec is solved by `reference_plugins/fea-reference-beam/`
(closed-form Euler-Bernoulli beam FEM). The two plugins are **two
independent implementations of the same physics**, mutually checked
to 5% agreement in `test_cross_plugin_consistency.py`.

## Honest label (I-7)

The adapter is narrow on purpose: only the LE1 cantilever case is
closed-form-checked. The wider CalculiX capability surface (modal,
nonlinear, plate/shell, 3D brick, contact, etc.) is **out of
scope** for this adapter — it is a production-track narrow
adapter, not a general CalculiX wrapper. `discover()` and
`capabilities()` declare this scope explicitly.

The `is_real_physics: true` flag in `discover()` is the honesty
label: this is a real FEA, not a fixture. The `validated_cases`
list in `discover()` and `capabilities()` declares the
closed-form-checked case; any future addition to that list must
have a corresponding analytical or published-benchmark check
and a corresponding test.

## Dependencies

- **Required:** `ccx_2.23` (CalculiX 2.23). On Homebrew:
  `brew install calculix-ccx`. On this machine:
  `/opt/homebrew/bin/ccx_2.23`. Set `CCX_BIN` env var to override.
- **Optional:** `pytest` (test dependency).

If `ccx_2.23` is not on PATH, the adapter is **honest about it**:
`discover()` reports `solver_binary_resolved: false`; `validate()`
returns `overall_status: fail` with `ccx_binary_available: fail`;
`execute()` returns `verification.overall_status: fail` with a
diagnostic — it never fabricates a result (I-12).

## Tests

```bash
PYTHONPATH=python/aether_sdk:python/aether_testing \
  python3 -m pytest reference_plugins/calculix-adapter/tests/ -v
```

The tests cover:

- discover() declares `is_real_physics: true` and the validated case.
- capabilities() declares the LE1 cantilever case.
- validate() rejects unknown cases (overall_status: fail).
- validate() accepts the LE1 case (overall_status: pass when ccx is
  available; fail with `ccx_binary_available` otherwise).
- execute() runs the real ccx binary; tip deflection is within 5%
  of the analytical 2.0 m.
- execute() is bit-reproducible (same inputs → same cas_hash).
- execute() refines: 16 elements ≤ 8 elements in relative error.
- I-12: bad input fails validation without invoking the binary.
- I-12: unknown case fails with a clear diagnostic.
- Full JSON-RPC round-trip (discover and execute) via stdio.
- Cross-plugin consistency: fea-reference-beam and calculix-adapter
  agree to within 5% on the same problem spec.

## Two-implementation rule (ADR-0022)

The two-implementation rule for `aether.physics.solver` is satisfied
by **two real, mutually-checked implementations**:

1. **`reference_plugins/fea-reference-beam/`** — Euler-Bernoulli
   beam FEM (closed-form element matrices, direct global solve).
   Analytical check on 4 cases.
2. **`reference_plugins/calculix-adapter/`** *(this plugin)* —
   real Timoshenko beam FEM via the ccx_2.23 binary.
   Analytical check on the LE1 case (5% FEM-tolerance).

Plus two protocol shims for the contract-surface half:

3. **`reference_plugins/openfoam-adapter/`** — thin shell-wrapper
   around `cfd-reference-minimal` (the analytical Poiseuille
   solver). Not a real OpenFOAM wrapper; tracks AP-11.
4. **`reference_plugins/cfd-reference-minimal/`** — real
   analytical Poiseuille flow (no CFD, but a real closed-form
   PDE solution).

See `contracts/physics.solver/README.md` for the canonical
two-impl table.
