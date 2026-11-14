# `aether.physics.solver` Contract v0

Physics solver contract — the archetype for long-running, tool-wrapping contracts (SDP §5.5). Defines the discover → capabilities → validate → prepare → execute → collect → verify → provenance lifecycle.

## Versioning

- **Contract id:** `aether.physics.solver`
- **Schema version:** `0.1.0` (Phase 0 v0; v1.0 freeze at Phase 2 G2)
- **Method count:** 8 lifecycle methods

## Methods

| Method | Description |
|---|---|
| `discover()` | Cheap, no side effects. Returns `PluginDescriptor`. |
| `capabilities()` | Structured facts (incl. validated-cases registry). |
| `validate(problem)` | Feasibility, license, resources. Returns `ValidationReport`. |
| `prepare(problem, ctx)` | Inputs staged to CAS. Returns `PreparedRun`. |
| `execute(run, placement)` | Returns `JobHandle` (non-blocking). |
| `collect(job)` | Returns `ResultBundle` (typed outputs). |
| `verify(bundle)` | Solver self-checks (residuals, convergence, conservation). Returns `VerificationReport`. |
| `provenance(job)` | Returns `ProvenanceRecord` (I-8). |

**Fidelity honesty is operational here**: a plugin that cannot certify convergence (verify() fails) SHALL NOT emit a Result with `fidelity_class: high_fidelity` (I-7).

## Layout

```
physics.solver/
├── manifest.json
├── schema/
│   ├── methods.schema.json
│   ├── problem.schema.json
│   ├── result-bundle.schema.json
│   └── verification-report.schema.json
├── examples/
│   └── wing-rans.problem.yaml
└── README.md
```

Note: the structural-solver conformance problem for
`fea-reference-beam` is shipped with the plugin itself
(`reference_plugins/fea-reference-beam/tests/test_beam_conformance.py`
contains four `ProblemSpec` instances that exercise the four
validated cases). The contract's `problem.schema.json` is the
*minimum* spec for the `validate` method; plugin-specific
problem extensions (e.g. beam length, modulus) live in the
plugin's documented `validate`/`execute` contract. The two
are kept in lockstep by the conformance suite: a beam
problem-spec change requires updating the plugin's `VALIDATED_CASES`
and the test file together, and the dark-build runs both.

## Two-implementation rule (ADR-0022)

- **Reference impl 1 (Phase 0–2, real physics):** `reference_plugins/fea-reference-beam/`
  — a deliberately minimal Euler-Bernoulli beam FEM (2-noded beam
  elements, direct global solve) used only for conformance testing
  (SDP §5.6, §21). The `execute` method runs the FEA solve and
  reports `verification.overall_status: pass` only when the
  numerical tip/midspan deflection matches the closed-form
  analytical formula to within the declared tolerance. Four
  validated cases: cantilever with point load, cantilever with UDL,
  simply-supported with point load, simply-supported with UDL. The
  relative error vs the analytical formula is < 1e-6 with 8
  elements and < 1e-8 with 16 elements.
- **Reference impl 2 (Phase 0–2, real physics):** `reference_plugins/cfd-reference-minimal/`
  — a steady, fully-developed plane Poiseuille-flow solver that
  samples the analytical parabolic velocity profile
  `u(y) = (G/(2 mu)) y (h - y)` on a uniform y-grid and reports
  the analytical `u_max`, `u_mean`, and `pressure_drop`. The
  closed-form comparison is exact (the profile is the analytical
  solution everywhere in the channel), so the
  `verification.overall_status: pass` line is a numerical claim
  about the sampling, not a discretisation claim.
- **Protocol shim (Phase 0–2, not real physics):** `reference_plugins/openfoam-adapter/`
  — a thin shell-wrapper that shells out to the
  `cfd-reference-minimal` plugin and stamps the wrapper identity
  on the relayed result. The plugin manifest `com.acme.cfd.opensfoam`
  is the *contract-facing* id; the internal id is
  `org.aether.reference.fe.opencfd`. This shim exists to satisfy
  the two-impl manifest pair (one real, one protocol-only) without
  claiming OpenFOAM is invoked; the production OpenFOAM adapter is
  a G2/G3 deliverable.
- **Reference impl 3 (Phase 2, real physics, closed-form-checked):**
  `reference_plugins/calculix-adapter/` — a real native-binary
  adapter that drives the `ccx_2.23` binary on a single validated
  case: the NAFEMS LE1 cantilever beam. The adapter writes a real
  `cantilever.inp` deck (B31 elements, `*BEAM SECTION SECTION=RECT`,
  isotropic linear-elastic material, encastre root, tip point
  load), shells out to `ccx_2.23` in a per-call tmpdir, parses the
  real `.frd` output, and reports the tip deflection against the
  closed-form analytical formula `P L³ / (3 E I)`. With 8 B31
  elements the FEA gives ~1.99 m vs the analytical 2.0 m (0.5%
  relative error; the 5% tolerance is the engineering standard
  NAFEMS acceptance). The adapter is **narrow on purpose**: only
  the LE1 case is closed-form-checked; the wider CalculiX
  capability surface (modal, nonlinear, plate/shell, 3D brick)
  is out of scope and is declared as such in `discover()` and
  `capabilities()`. A fitness check (`infra/fitness/ccx_binary.py`)
  enforces that the binary is on PATH in environments that
  declare the adapter production-grade. A second, mutually-checked
  test (`test_cross_plugin_consistency.py`) runs the SAME
  problem spec through both `fea-reference-beam` and
  `calculix-adapter` and asserts the two numerical tip deflections
  agree to within 5% — this is the explicit "two real,
  mutually-checked implementations" claim for the structural-
  solver side of the two-impl rule.
- Production-grade OpenFOAM adapter (and the wider CalculiX surface
  beyond LE1) land at Phase 2/3 and are the path to G2 acceptance.

## See also

- SDP §5.4 (contract catalog)
- SDP §5.5 (solver contract in depth)
- SDP §5.6 (native binary adapter pattern)
- ADR-0007 (physics-first, fidelity-honest)
- ADR-0008 (provenance completeness)
- ADR-0022 (two-implementation rule)
