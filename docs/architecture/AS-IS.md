# Aether — AS-IS Architecture Inventory

**Status:** repository-grounded inventory, 2026-11-24.
**Scope:** what exists in this checkout, not the target architecture.
**Companion documents:** [`GAP-MATRIX.md`](GAP-MATRIX.md),
[`AETHER-3.0.md`](AETHER-3.0.md), [`../verification/STATUS.md`](../verification/STATUS.md).

---

## 1. Workspace

| Item | Reality |
|---|---|
| Language | Rust workspace (edition 2021, MSRV 1.75 declared; built here with 1.99) |
| Crates | 12 workspace members |
| Rust source | ~19,732 lines including tests |
| Build | `cargo build --workspace` |
| Tests | `cargo test --workspace`; Python `pytest` trees under `python/` and `reference_plugins/` |
| CI | `infra/ci/dark-build.sh`, 24 steps, egress-free by policy |
| Fitness | `infra/fitness/run.py` (LOC budget, token scan, schema semver, SBOM, geometry fixture label, CalculiX binary, Rosetta equivalence, contract fuzzing) |

### Crate inventory

| Crate | Role | Approx. Rust LOC |
|---|---|---|
| `aether_kernel` | stdio JSON-RPC transport and process reaping (walking skeleton) | 195 |
| `aether_graph` | Event-sourced Engineering Graph, AQL subset, AS-OF replay, SQLite/Postgres | 2,462 |
| `aether_workflow` | AWL model, DAG compiler, executor, checkpoints | 1,625 |
| `aether_provenance` | Provenance records, Merkle chain, PROV-O export | 857 |
| `aether_policy` | Role catalog, action matrix, policy decision surface | 530 |
| `aether_registry` | Plugin discovery, manifest parsing, capability matching | 613 |
| `aether_exec` | Backend trait, local and fixture backends, handlers, run tokens | 1,933 |
| `aether_compose` | Multiphysics coupling, IQN-ILS-style iteration, conservation checks | 518 |
| `aether_envman` | Environment manifest, strict verification, hashing | 543 |
| `aether_contract_validate` | Rust half of contract validation | 475 |
| `aether_cli` | Canonical CLI, golden path, scientific loop, benchmarks | 7,535 |
| `aether_sdk_ffi` | FFI placeholder | 4 |

---

## 2. Contracts and graph schema

- **24 contract packages** under `contracts/`, each with `manifest.json`,
  schemas, and examples:
  `asm`, `compute.backend`, `evidence`, `field.model`, `frames`, `geometry.kernel`,
  `geometry.recipe`, `graph.model`, `inspection.device`, `io.format`,
  `manufacturing.process`, `mesh.engine`, `model-card`, `optimizer`,
  `physics.composable`, `physics.solver`, `plugin.manifest`, `quantities`,
  `requirement`, `si.governor`, `surrogate`, `test.rig`, `twin.estimator`,
  `uq.method`.
- **Graph catalog:** 61 node types and 32 edge types, loaded at runtime from
  `contracts/graph.model/catalog/`.
- **Node types:** Requirement, System, Part, PhysicalPart, GeometryRevision,
  Mesh, LoadCase, SimulationStudy, SimulationRun, Result, Evidence, Decision,
  OptimizationStudy, SurrogateModel, PhysicsModel, MeasurementDataset,
  InspectionRecord, TestRun, ProcessPlan, ProcessStep, Machine, Tooling,
  Material, Workflow, PluginInstance, Model.
- **Edge types:** satisfies, traces_to, realized_as, simulated_by,
  executed_on, verifies, measured_by, updates, supersedes, decomposes,
  subject_to, optimized_by, planned_by, produced_by, produced, has_geometry,
  has_material, depends_on.

### Graph mechanics that are real

- Append-only event log with SHA-256 canonical-JSON hash chain.
- UUIDv7 event identity; `prev_event_hash` links.
- `NodeUpserted`, `NodeStateChanged`, `NodeTombstoned`, `EdgeAdded`,
  `EdgeRemoved`, `RevisionSuperseded`.
- Immutable revisions; typed lifecycle states.
- AQL subset: type filters and property filters.
- AS-OF replay by event offset.
- SQLite WAL default; Postgres storage implementation.
- 10M-node insert/replay scale binary; replay measured well inside the
  Phase 0 budget, insert time over the implied Phase 1 budget (AP-29).

---

## 3. Plugin and execution substrate

- Plugins are separate processes speaking newline-delimited JSON-RPC 2.0 over
  stdio (`discover`, `capabilities`, `validate`, `execute`).
- `aether_exec` routes by `contract::plugin`; handlers include the generic
  stdio handler, CalculiX adapter, OpenFOAM adapter, and mesh baseline.
- Signed run tokens are issued by the kernel signing key.
- `Backend` trait covers local, Slurm, Apptainer, SSH batch, and agent pool.
  Only `Local` is exercised as a real execution path; the others produce
  contract-shaped fixtures.
- **No sandbox:** `LocalBackend` runs child processes directly; rlimit,
  filesystem isolation, and seccomp/sandbox profiles are explicitly deferred.

---

## 4. Reference plugins

| Plugin | Contract | Classification |
|---|---|---|
| `fea-reference-beam` | physics.solver | Real Euler–Bernoulli beam FEM, closed-form checked |
| `cfd-reference-minimal` | physics.solver | Analytical Poiseuille flow |
| `calculix-adapter` | physics.solver | Real `ccx` binary adapter for NAFEMS LE1 |
| `openfoam-adapter` | physics.solver | Thin shim to the Poiseuille fixture |
| `aero-lifting-line` | physics.solver | Glauert collocation, closed-form verification |
| `structures-sizing` | physics.solver | Wing-box boom-idealization sizing |
| `occ-geometry` | geometry.kernel | Real OpenCASCADE B-rep kernel (STEP/IGES, booleans, fillets, tessellation) |
| `mesh-gmsh` | mesh.engine | Real Gmsh tetrahedral mesher with quality metrics |
| `reference-geometry` | geometry.recipe | Recipe/tessellation reference |
| `mesh-baseline` | mesh.engine | Minimal baseline mesher |
| `uq-basic` | uq.method | Sobol/LHS, analytic benchmarks |
| `optimizer-reference` | optimizer | NSGA-II/III |
| `surrogates-core` | surrogate | GP/POD |
| `si-essentials` | si.governor | Fidelity escalation governor |
| `twin-kf` | twin.estimator | KF family, scalar and vector state |
| `plc-reference` | manufacturing/inspection/test | Conformance fixture |

---

## 5. CLI surface

The CLI is the canonical interface (`aether`). Current top-level commands:

`init`, `doctor`, `demo`, `run`, `explain`, `experiment`, `evidence`,
`export`, `replay`, `benchmark`, `project`, `graph`, `plugin`, `package`,
`workflow`, `geometry`, `mesh`, `requirement`, `simulate`, `uq`, `optimize`,
`twin`, `manufacture`, `inspect`, `test`, `deploy`, `envman`, `verify-tuf`,
`dark-build-check`.

The golden path is `doctor` → `demo` → `init --profile` → `run` → `explain`
→ `evidence export` → `benchmark`. JSON output is selected with `--json`.

---

## 6. Security and reproducibility

- **Real:** Ed25519 signatures in the TUF-adapted offline verifier, signed run
  tokens, SHA-256 content hashing, Merkle provenance chain, hash-chain
  verification, sovereign export/replay.
- **Not real yet:** policy evaluation (the engine always allows), plugin
  sandboxing, key rotation/revocation operations, worker attestation, tested
  RPO/RTO restore drills, observability metrics/traces.
- **Reproducibility:** environment manifests, input hashes, canonical JSON,
  offline bundle replay.

---

## 7. Documentation state

- `docs/ARCHITECTURE.md` describes the built platform and is largely accurate
  but has stale counts (crate LOC, test counts, fitness-check count).
- `specs/invariants.md` is the canonical invariant table (I-1…I-12).
- `specs/adr/` contains ADR-0000…ADR-0024.
- `docs/certification/` contains S-1…S-8 rollups; some counts are stale.
- `specs/threat-model/README.md` is an abridged v1 threat model.
- `workflows/paper-run/gaps.md` is the living open-items log.
- `README-EVAL.md` and the alignment changelogs are historical records, now
  under `docs/history/`.
- `SDP.md` was referenced but absent; a compatibility pointer now exists.

---

## 8. Known baseline defects

1. `cargo test --workspace` fails in `benchmarks_execute_for_real` because
   BENCH-08 hard-fails when `ccx` is absent.
2. `PolicyEngine::evaluate` always allows.
3. Plugin execution is unsandboxed.
4. Python toolchain pins and `requires-python` disagree; `hypothesis` is not
   installed in the current environment.
5. Documentation links and counts are inconsistent.
6. `site/` and the Aether 3.0 source document were untracked; `.DS_Store`
   files were present.
7. Funding/affiliation wording conflicts with the proprietary license and the
   latest commit intent.

See [`GAP-MATRIX.md`](GAP-MATRIX.md) for the capability-level mapping.
