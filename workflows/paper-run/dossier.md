# Paper Run: UAV Wing Rib — Dossier

The narrative from **requirement R-STR-011** to **model update**, in
prose form, with concrete references to the contract schemas and node
types that the Engineering Graph uses for each link.

## 1. The requirement

`Requirement R-STR-011` is captured in the graph as a `Requirement`
node with `text`, `criticality`, `verification_method`, and `status`
properties (per `contracts/graph.model/catalog/nodes/requirement.node.json`).
For the wing rib, R-STR-011 reads:

> The wing rib shall sustain the limit load factor of 1.5 with no
> failure, across the maneuver spectrum −1g to +4g.

`criticality: catastrophic`, `verification_method: test`, `status: open`.

## 2. The system breakdown

The system is captured as a tree of `System` → `Assembly` → `Component`
nodes, connected by `decomposes` edges. The wing rib is `Part: WING-RIB-014`
under `Assembly: WING-ASSEMBLY` under `System: UAV-WING`.

## 3. The geometry

`Part: WING-RIB-014` is connected to `GeometryRevision #7` via
`has_geometry`. The geometry revision carries:

- `cas_hash` — SHA-256 of the AP242 file in the content-addressed store
- `format: step-ap242` — STEP AP242 edition 2 (SDP §16)
- `parametric_recipe_ref` — pointer to a Python recipe that regenerates
  the geometry from parameters; this is what makes CAD replacement
  non-destructive (MVR-06)
- `frame`, `units`, `tolerance_baseline` — explicit, not implicit
- `pmi_refs` — GD&T in the same artifact, accessible to manufacturing

## 4. The material

`Part: WING-RIB-014` is connected to `Material: M21E-tape` via
`has_material`. The material is a `Material` node with `class`,
`allowables_basis`, `certification_status`. The material is **not** a
file in the graph — it is a typed node referenced by the same CAS store
machinery.

## 5. The load case

`Part: WING-RIB-014` is connected to `LoadCase LC-04` via `subject_to`.
The load case is a `LoadCase` node with `spectrum`, `limits`, `sources`.
LC-04 covers the maneuver spectrum −1g→+4g.

## 6. The simulation

`Part: WING-RIB-014` is connected to `SimulationStudy SS-112` via
`simulated_by`. SS-112 is an aeroelastic study, coupled (the composition
is in `AWL v1.2`'s `compose:` block; Phase 4 deliverable).

SS-112 produces `SimulationRun #2031`, which:

- references `input_snapshot_hash 9f3a…` — frozen hashes of geometry,
  mesh, model, parameters, plugin versions, environment (I-8)
- declares the plugin (`com.acme.cfd.opensfoam@2.4.1`) and the
  contract versions it speaks
- records its `placement` (which backend, which node class)
- records its `determinism_class`

`SimulationRun #2031` produces `Result` nodes, each carrying:

- `fidelity_class` — high_fidelity for the converged RANS result
- `validation_status` — verified (the solver's `verify()` method
  reported convergence, conservation, and benchmark gates pass)
- `uncertainty` — bands per quantity, method = convergence_residual
- `envelope` — input-space coordinates where the result is valid

## 7. The optimization

`Part: WING-RIB-014` is connected to `OptimizationStudy DS-018` via
`optimized_by`. DS-018 is an NSGA-II study with a GP surrogate. The
study is itself a `OptimizationStudy` node referencing its
`DesignVariable` and `Objective` nodes; the Pareto front is
`produced_by` `SimulationRun`s with the surrogate.

## 8. The manufacturing plan

`Part: WING-RIB-014` is connected to `ProcessPlan PP-201` via
`planned_by`. PP-201 is a `ProcessPlan` node: ordered steps, parameters,
resources, acceptance criteria. Its steps are `ProcessStep` nodes
connected to `Machine` (AFP-CELL-2) via `executed_on`.

The plan is **not an export** from the design system; it is a graph
object referenced by the same identity scheme. A stale-geometry plan is
a graph-validated error, not a habit.

## 9. The physical part

`Part: WING-RIB-014` is connected to `PhysicalPart WING-RIB-014-003`
via `realized_as`. The physical part carries:

- `serial` — unique identifier
- `as_built_refs` — references to as-built geometry and material
- `genealogy` — material batch, sub-parts, machine/operator config

## 10. The inspection and test

`PhysicalPart WING-RIB-014-003` is connected to `InspectionRecord
IR-9987` (laser scan, 98.7% coverage) and `TestRun TR-114` (buckling
test to 120% limit load) via `measured_by`.

`TestRun TR-114` produces `MeasurementDataset` (120 channels, 2.1 MSa)
which `updates` the `Model: buckling ROM rev 3`. The model update is
itself a new revision with an `updates` edge from the dataset that
justified it (SDP §8.6).

## 11. The evidence and the decision

`Part: WING-RIB-014` is connected to `Evidence EV-441` (compliance
claim, chain of 14 artifacts) via `traces_to`. The evidence dossier
is `aether provenance chain --part WING-RIB-014-003 --format dossier`
— the certification persona's daily artifact.

At each significant choice (which solver, which surrogate, which
optimization, when to test), the Scientific Intelligence layer
recorded a `Decision` node with its `autonomy_class`. A4 is forbidden.
A3 (surrogate-as-sufficient for verification-relevant claim) is gated
on named human sign-off; the signoff is a `Decision.review_signoff`
property.

## 12. The close

The arrow from `MeasurementDataset` back to `Model` is a single typed
edge. The arrow from `Model` back to `Requirement` is `verifies` — the
model update satisfied the requirement. The arrow from `Requirement`
to `System` is `satisfies` — the design meets the requirement. The
closed loop is **graph-recorded**, not narrated.

## See also

- `chain.yaml` — the same narrative as a typed node+edge document
- `interfaces.md` — which contracts the paper run touches
- `gaps.md` — open items from the paper run
- SDP §3.7 (Engineering Graph example projection)
