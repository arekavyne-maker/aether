# Contract catalog

Aether has 19 contracts (G0..G7). Each contract declares a method
surface, a set of JSON Schemas, and example documents. The
two-implementation rule (ADR-0022) requires that each contract has
two independent implementations before the contract is stable.

| Contract | Purpose | Phase |
|---|---|---|
| [`aether.plugin.manifest`](../contracts/plugin.manifest/) | Plugin identity, capabilities, isolation, runtime | G0 |
| [`aether.physics.solver`](../contracts/physics.solver/) | CFD/FEA-style solver contract (discover / capabilities / validate / prepare / execute / collect / verify / provenance) | G0 |
| [`aether.compute.backend`](../contracts/compute.backend/) | Resource requirements (memory, cores, accelerator, architecture) | G0 |
| [`aether.io.format`](../contracts/io.format/) | Data format descriptors (STEP AP242, CGNS, HDF5/XDMF, Zarr, ...) | G0 |
| [`aether.graph.model`](../contracts/graph.model/) | Engineering Graph node / edge / event / state schemas | G0 |
| [`aether.geometry.kernel`](../contracts/geometry.kernel/) | CAD-style geometry service (import / export / parameterize / tessellate / boolean / fillet / check) | G3 |
| [`aether.geometry.recipe`](../contracts/geometry.recipe/) | Versioned, tool-agnostic parametric recipe | G3 |
| [`aether.mesh.engine`](../contracts/mesh.engine/) | Mesh generation and adaptation | G3 |
| [`aether.field.model`](../contracts/field.model/) | Neutral field dataset (mesh + fields + metadata) | G3 |
| [`aether.requirement`](../contracts/requirement/) | Requirement package + SysML v2 textual subset | G3 |
| [`aether.physics.composable`](../contracts/physics.composable/) | Multiphysics coupling + IQN-ILS + conservation | G4 |
| [`aether.uq.method`](../contracts/uq.method/) | LHS, MC, PCE, Sobol, GP surrogate methods | G4 |
| [`aether.si.governor`](../contracts/si.governor/) | SI decision records (autonomy A0..A4) | G4 |
| [`aether.optimizer`](../contracts/optimizer/) | Design optimization (NSGA-III, MOEA/D, CMA-ES, gradient NLP) | G5 |
| [`aether.twin.estimator`](../contracts/twin.estimator/) | EKF, UKF, particle filter | G5 |
| [`aether.surrogate.builder`](../contracts/surrogate/) | GP-RBF, POD, deep_ensemble, RBF, polynomial_chaos | G5 |
| [`aether.manufacturing.process`](../contracts/manufacturing.process/) | Plan, simulate, export_program, record_as_run | G6 |
| [`aether.inspection.device`](../contracts/inspection.device/) | CMM, structured-light scan, NDT | G6 |
| [`aether.test.rig`](../contracts/test.rig/) | DAQ, channel mapping, telemetry streaming | G6 |

## How to read a contract directory

```
contracts/<contract-id>/
├── manifest.json          # methods, schemas, vocabularies
├── schema/                # JSON Schema files (draft 2020-12)
│   └── <name>.schema.json
├── examples/              # YAML/JSON example documents
│   └── <name>.yaml
├── README.md              # contract-specific notes
```

## Conformance fixtures

The conformance fixtures (in `python/aether_testing/` and
`crates/aether_contract_validate/`) load every contract manifest,
compile every schema, and validate every example. A new contract
must ship with:

- `manifest.json` declaring methods and schemas
- At least one `schema/<name>.schema.json`
- At least one `examples/<name>.yaml`
- Two independent plugins that pass the conformance test (ADR-0022)

The dark-build runs `cargo run -p aether_contract_validate` which
exits non-zero on any validation failure.
