# Aether Platform — Architecture

**Status:** reference document · **Date:** 2026-09-12 · **Scope:** the platform as actually built (Phases 0–2 substrate, golden path, scientific loop) — not the full SDP vision.

This document describes the Aether architecture as it exists in this repository: the crates, contracts, plugins, protocols, and enforcement machinery, with honest labels for what is real and what is a fixture. For status claims and their verification commands see [`docs/verification/STATUS.md`](verification/STATUS.md); for open items see [`workflows/paper-run/gaps.md`](../workflows/paper-run/gaps.md); for the normative target architecture see [`docs/architecture/AETHER-3.0.md`](architecture/AETHER-3.0.md).

---

## 1. Purpose and positioning

Aether is an air-gapped, hardware-agnostic engineering fabric for emerging aerostructures. Its one central thesis:

> **Everything domain-specific is a plugin; the only shared truth is one append-only, hash-chained Engineering Graph; and every claim is carried with the evidence that produced it.**

It is *not* a CAD system, a solver, a PLM suite, or a notebook. It runs no production physics engine of its own. It is the connective, governing, reproducibility layer that makes CAD kernels, solvers, optimizers, and manufacturing systems interchangeable parts of the same engineering process.

The design is **dual-use by symmetry**: the substrate that carries a certification evidence chain for a civilian UAV wing carries any aerostructure workflow that needs audit, restricted toolchains, and long reproducibility windows. Nothing in the substrate is aerostructure-specific; the *contracts* and *plugins* are where the application lives.

---

## 2. The Twelve Invariants

The architecture is governed by twelve non-negotiable invariants, each with a named enforcement mechanism (an invariant without enforcement is a wish). Ratified in ADR-0001…0012; canonical text in [`specs/invariants.md`](../specs/invariants.md).

| ID | Invariant | Enforcement |
|---|---|---|
| I-1 | Microkernel minimality (~50k LOC cap) | LOC budget check in CI; ADR required for kernel additions |
| I-2 | Everything else is a plugin | Kernel dependency scan; two-implementation rule; conformance suites |
| I-3 | Zero hard-coupling | Import-linter rules; prohibited-token scan (no `if solver == X`) |
| I-4 | Graph stability under implementation change | MVR-01/02/06/07; schema semver checker |
| I-5 | Air-gap by design | Egress-free 24-step dark-build CI; HTTP wrapper fails closed; MVR-08 |
| I-6 | Hardware-agnostic execution | Resource-descriptor schema forbids hardware fields; MVR-04/05/12 |
| I-7 | Physics-first, fidelity-honest | Result schema requires `fidelity_class` + `uncertainty` |
| I-8 | Provenance completeness | Provenance record mandatory per contract call; hash-chained audit log |
| I-9 | Reproducibility | Environment Manifest lockfile semantics; MVR-09/11 |
| I-10 | CLI is the canonical interface | GUI/CLI parity through one service layer — no parallel path |
| I-11 | Contracts versioned + two-implementation rule | Semver discipline; per-contract conformance suite; 2-impl badge |
| I-12 | Fail-visible, never fail-silent | Typed error taxonomy; contract violations fatal to the run |

I-12 has real teeth: the vector KF's matrix-dimension validation (Wave 4) exists because a silently wrong posterior is worse than a loud error — the test suite forces the loud path.

---

## 3. System overview

```
                                ┌─────────────────────────────────────────────┐
                                │              aether CLI (I-10)              │
                                │  init · doctor · demo · run · explain ·     │
                                │  evidence · optimize · experiment · twin ·  │
                                │  benchmark · export/replay · graph · …      │
                                └───────────────────────┬─────────────────────┘
                                                        │ same service layer
                          ┌─────────────────────────────┼──────────────────────────────┐
                          │                             ▼                              │
                          │              ┌──────────────────────────┐                 │
                          │              │   aether_workflow (AWL)  │                 │
                          │              │  YAML → DAG → executor   │                 │
                          │              └───────────┬──────────────┘                 │
                          ▼                          ▼                                ▼
        ┌───────────────────────────────────────────────────────────────────────────────────┐
        │                          DOMAIN SUBSTRATE  (all below the kernel line)            │
        │                                                                                   │
        │  ┌─────────────────┐   ┌──────────────────┐   ┌─────────────────────────────┐     │
        │  │  aether_graph   │   │  aether_exec     │   │ aether_compose              │     │
        │  │  event-sourced  │   │  Backend trait   │   │ IQN-ILS (Broyden) coupling  │     │
        │  │  Engineering    │   │  Local/Slurm/    │   │ conservation checks,        │     │
        │  │  Graph + AQL    │   │  Apptainer/SSH/  │   │ divergence abort            │     │
        │  │  + AS-OF replay │   │  AgentPool       │   │                             │     │
        │  └───────┬─────────┘   └────────┬─────────┘   └─────────────────────────────┘     │
        │          │                      │                                                 │
        │  ┌───────▼─────────┐   ┌────────▼─────────┐   ┌─────────────────────────────┐     │
        │  │ aether_provenance│  │ aether_registry  │   │ aether_policy               │     │
        │  │ hash-chained    │   │ discovery, semver│   │ RBAC catalog (8 roles),     │     │
        │  │ records + PROV-O│   │ Ed25519 signing  │   │ CEL decision surface        │     │
        │  └─────────────────┘   │ TUF-adapted      │   └─────────────────────────────┘     │
        │                        │ offline verify   │   ┌─────────────────────────────┐     │
        │  ┌─────────────────┐   └──────────────────┘   │ aether_envman               │     │
        │  │aether_contract_ │                          │ pinned plugin/env/input     │     │
        │  │validate (Rust)  │   ┌──────────────────┐   │ hashes; re-run = supported  │     │
        │  └─────────────────┘   │ aether_kernel    │   └─────────────────────────────┘     │
        │                        │ JSON-RPC/stdio   │                                       │
        │                        │ spawn/reap (195  │                                       │
        │                        │ LOC substrate)   │                                       │
        │                        └──────────────────┘                                       │
        └───────────────────────────────────────────────────────────────────────────────────┘
                     │ stdio JSON-RPC 2.0 (ADR-0015) — the only boundary to domains
        ┌────────────┼──────────────────────────────────────────────────────────┐
        ▼            ▼                 ▼                    ▼                     ▼
   physics.solver  mesh.engine    twin.estimator        si.governor          optimizer
   geometry.*      uq.method      surrogate             manufacturing.*      inspection.*
        │            │                 │                    │                     │
   [15 reference plugins — each behind exactly one contract — see §9]
```

Everything above the kernel line knows contracts, not vendors. Cross-domain communication happens only through graph objects and contracts (I-3).

---

## 4. The workspace: crates, roles, and sizes

Twenty-one crates, ~27.1k LOC of Rust source total — comfortably inside the kernel-boundary budget discipline (I-1). The kernel/Boundary LOC tracker counts the kernel and boundary crates, not the new domain libraries.

| Crate | LOC | Role |
|---|---|---|
| `aether_cli` | 7,535 | Canonical interface (I-10): command surface + the user-facing spine (demos, run/explain/evidence, optimize, experiment, benchmarks, sovereign export/replay) |
| `aether_graph` | 2,462 | Event-sourced Engineering Graph: event log, engine, AQL subset, AS-OF replay, SQLite storage (Postgres option), schema catalog loader, 10M-node scale binary |
| `aether_exec` | 1,933 | Execution: `Backend` trait + Local/Slurm/Apptainer/SshBatch/AgentPool backends, process supervisor, plugin handlers (`StdioPluginHandler`, `CalculiXAdapterHandler`, …), signed run tokens |
| `aether_workflow` | 1,625 | AWL model (fidelity ladder, composition, evidence, reproducibility declarations), DAG compiler, executor with step handlers and checkpoint stores |
| `aether_provenance` | 857 | Provenance records, Merkle chain, PROV-O export, canonical hashing |
| `aether_registry` | 613 | Plugin discovery, capability matching, semver range filtering, Ed25519 signing/verification |
| `aether_envman` | 543 | Environment Manifest: pinned versions + content hashes for plugins, environments, inputs |
| `aether_policy` | 607 | Fail-closed RBAC role catalog and decision point; unknown roles/actions denied |
| `aether_quantities` | 430 | Typed quantities, SI canonical conversion, dimensional checking, coordinate-frame graph |
| `aether_artifacts` | 250 | Content-addressed artifact store: SHA-256, atomic writes, dedupe, scoped handles |
| `aether_asm` | 430 | ASM lifecycle, revision lineage, interface compatibility, configuration snapshots |
| `aether_evidence` | 430 | Evidence classes, credibility records, release gates, waivers, release packages |
| `aether_manufacturing` | 430 | Manufacturing genealogy, as-built snapshots, disposition and revision-impact checks |
| `aether_model_registry` | 430 | Model cards, role-gated promotion, hash-chained registry audit |
| `aether_fidelity` | 430 | Deterministic multi-fidelity selection and escalation planning |
| `aether_design` | 430 | Design-space feasibility pipeline, candidate lineage, and uncertainty classes |
| `aether_agents` | 430 | Advisor-only agent tool permissions and human-approval gates |
| `aether_compose` | 518 | Multiphysics coupling: participants, exchanges, coupling schemes, Broyden-style IQN-ILS, conservation checks |
| `aether_contract_validate` | 475 | The Rust half of the two-implementation rule for contract validation |
| `aether_kernel` | 195 | The transport substrate: spawn plugin process, exchange newline-delimited JSON-RPC 2.0, reap. Proves the boundary without absorbing domain logic |
| `aether_sdk_ffi` | 4 | FFI placeholder (stubs out Phase 0) |

**Dependency policy is architectural.** The workspace ships only air-gap-friendly crates (no network-dependent dependencies). One example with real consequences: `serde_json` is compiled with `float_roundtrip` because float parsing must be correctly rounded — otherwise an event payload re-read from SQLite can hash differently than at write time (1-ULP drift) and every event carrying a float would fail chain verification. That single feature flag is the difference between an integrity system and a false-alarm generator, and it was found by exercising the golden path on float-bearing data, not by reading specs.

---

## 5. The Engineering Graph — the single source of truth

### 5.1 Event sourcing with a Merkle chain

The graph is not a table; it is a **log**. Every mutation is an `Event` of one of six kinds: `NodeUpserted`, `NodeStateChanged`, `NodeTombstoned`, `EdgeAdded`, `EdgeRemoved`, `RevisionSuperseded`. Current graph state is a replayable projection of the log.

Each event carries: `event_id` (UUIDv7 — time-ordered, index-friendly), `occurred_at`, `kind`, `actor`, `authz_decision` (the policy decision that admitted the event is recorded *with* the event), payload, `prev_event_hash`, and `event_hash`.

The chain is real cryptography over real canonicalization:

- `event_hash` = SHA-256 of the event's **canonical JSON** (keys sorted, no whitespace, `null` fields omitted — a strict subset of RFC 8785/JCS, chosen so the Rust implementation and the Python `json.dumps(..., sort_keys=True, separators=(",", ":"))` path produce byte-identical canonical form; the same canonicalization the TUF verifier uses).
- `prev_event_hash` links to the previous event's canonical hash → the log is a Merkle chain (SDP §7.6, §14.1). `event_hash` itself is excluded from its own hash input (the chain is over unsigned payloads).
- Tampering with any historical event breaks every hash after it; `aether graph chain` verifies continuity and reports `valid: true/false` — this verdict is carried inside every evidence bundle.

### 5.2 The schema catalog is data, not code

61 node types and 32 edge types — including the original 26 nodes (`Requirement`, `System`, `Part`, `GeometryRevision`, `Mesh`, `LoadCase`, `SimulationStudy`, `SimulationRun`, `Result`, `Evidence`, `Decision`, `OptimizationStudy`, `SurrogateModel`, `PhysicsModel`, `MeasurementDataset`, `InspectionRecord`, `TestRun`, `ProcessPlan`, `ProcessStep`, `Machine`, `Tooling`, `Material`, `Workflow`, `PluginInstance`, `Model`, `PhysicalPart`) plus the Aether 3.0 ASM/manufacturing/lifecycle set (`Program`, `AircraftConfiguration`, `Product`, `Assembly`, `Interface`, `Variant`, `ConfigurationBaseline`, `ConfigurationSnapshot`, `ChangeRequest`, `Effectivity`, `CoordinateFrame`, `Datum`, `MaterialAssignment`, `EnvironmentCase`, `VerificationCase`, `ValidationCase`, `ComplianceFinding`, `ReviewDecision`, `MaterialBatch`, `BuildRecord`, `ProcessParameterSet`, `OperatorQualificationReference`, `InspectionPlan`, `Nonconformance`, `RepairDisposition`, `AsBuiltConfiguration`, `Sensor`, `Measurement`, `MaintenanceEvent`, `DamageEvent`, `DigitalTwinState`, `DesignVariable`, `Objective`, `ConstraintFunction`, `CandidateDesign`) — are loaded at runtime from `contracts/graph.model/catalog/{nodes,edges}/*.json`. The engine's mechanics (validation, revisioning, indexing) are generic over any registered type: adding a domain concept is adding a JSON file, not recompiling the platform (I-2, ADR-0016 "contracts as data"). Duplicate type IDs are a `ContractViolation`, not a warning.

The edge vocabulary encodes the engineering closed loop itself: `satisfies`, `traces_to`, `realized_as`, `simulated_by`, `executed_on`, `verifies`, `measured_by`, `updates`, `supersedes`, `decomposes`, `subject_to`, `optimized_by`, `planned_by`, `produced_by`, …

### 5.3 Query and time travel

- **AQL subset** (`aql.rs` + engine): type filters + property filters; the query language is deliberately small until G1 grammar work.
- **AS-OF replay**: `aether graph as-of N` replays the log to the Nth event and emits the snapshot — historical states are first-class queries, not backups.

### 5.4 Storage

SQLite (bundled `rusqlite`, WAL) is the embedded default and the event-log store; a Postgres storage backend exists as the second implementation of the storage surface. The scale binary has demonstrated **10M-node insert + replay** on aarch64: replay 6.3 s against the Phase-0 budget of 300 s (47× headroom); insert time is honestly recorded as over the Phase-1 implied budget (AP-29 in `gaps.md`).

---

## 6. Workflow engine — AWL, DAGs, checkpoints

**AWL (Aether Workflow Language, ADR-0017)** is YAML, compiled to a DAG before execution. The model (`awl.rs`) carries the engineering semantics, not just control flow:

- `FidelityLadder` + `LadderTier` — named fidelity rungs a workflow may move between;
- `CompositionDecl` — participants, field exchanges (`CompositionExchange`), coupling scheme (`CompositionScheme`) for multiphysics steps;
- `EvidenceDecl` / `OutputDecl` — what the run must produce to be admissible;
- `ReproducibilityDecl` + `ReproducibilityMode` — the workflow's own reproducibility contract;
- `PlacementHint` (hardware-agnostic resource descriptors — I-6 forbids hardware fields), `ParameterSpace` (DOE declarations), `RetryPolicy`.

**Execution** (`execute.rs`): the `Executor` walks the compiled DAG with a `StepHandler` strategy per step type, producing `ExecutionResult`/`ExecutionStatus`. Steps are checkpointed (`CheckpointStore` trait: in-memory and file-backed stores) so interrupted runs resume. `Checkpoint`/`StepResult` records feed provenance.

The executor wraps step inputs as `{"step_id", "inputs"}` envelopes and routes by `contract::plugin` keys — one workflow may use multiple physics plugins. (An early golden-path bug where plugins silently executed with empty inputs was fixed by fixing the envelope handling in the plugins' JSON-RPC layer — the envelope is now round-trip-verified end-to-end.)

---

## 7. Plugin substrate — one protocol to rule every domain

### 7.1 The wire protocol (ADR-0015)

Plugins are **separate processes** speaking newline-delimited **JSON-RPC 2.0 over stdio**. The lifecycle is `discover` / `capabilities` / `validate` / `execute`; the kernel (and any handler) spawns the process, exchanges messages, and reaps it. The protocol is language-blind: reference plugins are Python, but nothing in the substrate cares.

This choice is the air-gap and hardware story in miniature: no sockets, no ports, no daemon, no shared memory — a plugin is a child process whose entire world is stdin/stdout, which is exactly what survives on a locked-down HPC node or an air-gapped workstation.

### 7.2 Executor-side routing

`aether_exec::handlers` holds the handler set: the generic `StdioPluginHandler` (spawn + JSON-RPC by contract id), plus specialized handlers where the plugin is a native binary or a wrapper (`CalculiXAdapterHandler` writes `.inp` decks and parses `.frd`; `OpenFOAMAdapterHandler`; `MeshBaselineHandler`), and a `SharedState` for run-scoped coordination. `run_token.rs` issues **signed run tokens** (kernel signing key, scoped claims) so an execution can prove it was authorized to run what it ran.

### 7.3 The Python SDK

`python/aether_sdk` implements the plugin side of the protocol; `aether_testing` provides the conformance harness; `aether_geometry` and `aether_requirements` are the Python half of geometry/requirement contract handling (the two-implementation rule in action).

---

## 8. Contracts — nineteen packages, two-implementation rule

Under `contracts/`, each domain is a versioned package with JSON Schemas (37 schemas total) and a conformance suite:

| Family | Contracts |
|---|---|
| Graph & platform | `graph.model`, `plugin.manifest`, `io.format`, `compute.backend` |
| Geometry & mesh | `geometry.kernel`, `geometry.recipe`, `mesh.engine` |
| Physics | `physics.solver`, `physics.composable`, `field.model` |
| Science | `uq.method`, `optimizer`, `surrogate`, `si.governor`, `twin.estimator` |
| Manufacturing & test | `manufacturing.process`, `inspection.device`, `test.rig` |
| Requirements | `requirement` |

**The two-implementation rule (ADR-0022)** is the epistemological core: *no contract is considered stable until at least two independent implementations exist and are run against each other on every release.* One implementation can only prove that a contract matches its author's accident; two prove it abstracts something real. Where a second real implementation does not yet exist, the gap is labeled (see §9) and tracked — it is never papered over. Contract semver discipline is machine-checked (`schema_semver` fitness check), and the Rust `aether_contract_validate` crate is itself the second implementation of the validation surface.

---

## 9. Reference plugins — the honest inventory

Fifteen reference plugins implement the contract surface. Their honesty classification is carried on the artifacts themselves (I-7, I-12), not asserted in prose:

| Plugin | Contract | What it actually is | Class |
|---|---|---|---|
| `fea-reference-beam` | physics.solver | Real Euler-Bernoulli beam FEM (stiffness assembly, linear solve); per-element `ei_scales`; spanwise deflection/slope profiles; closed-form-checked to ~1e-13 | **REFERENCE** |
| `cfd-reference-minimal` | physics.solver | Analytical steady Poiseuille flow; exact parabolic profile sampling | **REFERENCE** (closed-form) |
| `calculix-adapter` | physics.solver | Real native-binary adapter: writes NAFEMS LE1 `.inp` deck, runs real `ccx` binary, parses real `.frd`; 0.44% vs closed form; cross-checked against the beam FEM (two real implementations agreeing) | **REFERENCE** (real binary) |
| `openfoam-adapter` | physics.solver | Thin shell-wrapper with own PLUGIN_ID; relays to the Poiseuille fixture; the contract-surface half for fluids | **SHIM** (real OpenFOAM = AP-11) |
| `aero-lifting-line` | physics.solver | Glauert Fourier collocation of the lifting-line equation; elliptical planform is a fixed point → machine-precision closed-form verification + span-efficiency identity check | **REFERENCE** |
| `structures-sizing` | physics.solver | Wing-box boom-idealization sizing: stress, shear buckling, deflection, mass, reserve margins; batch mode | **REFERENCE** (assumptions declared) |
| `occ-geometry` | geometry.kernel | Bounding-box fixture; machine-enforceable `is_real_brep: false` flag + "not real B-rep" note enforced by the `geom_fixture_label` fitness check | **FIXTURE** (real B-rep = AP-13) |
| `reference-geometry` | geometry.recipe | Tessellation matching the OCC-shaped path; exists so MVR-06 (CAD connector replacement) has its two implementations | **REFERENCE** |
| `mesh-baseline` | mesh.engine | Baseline mesher; contract surface for a commercial mesher later | **REFERENCE** (minimal) |
| `uq-basic` | uq.method | Real Sobol/LHS (Saltelli A/B matrices, per-column stratified permutation — a real bug found here by BENCH-06), Ishigami/ZDT1/DTLZ2 | **REFERENCE** |
| `optimizer-reference` | optimizer | Real NSGA-II/III | **REFERENCE** |
| `surrogates-core` | surrogate | Real GP/POD construction and prediction | **REFERENCE** |
| `si-essentials` | si.governor | `FidelityEscalationGovernor` + SI method routing; deterministic escalate/stay decisions | **REFERENCE** (cost table illustrative) |
| `twin-kf` | twin.estimator | KF family (linear/EKF/UKF/EnKF/particle) + `ekf_vector`: multivariate linear KF with full covariance algebra and loud dimension validation (I-12) | **REFERENCE** (linear-Gaussian exact) |
| `plc-reference` | manufacturing.process / inspection.device / test.rig | One conformance fixture implementing all three G6 contracts, routed by method name | **FIXTURE** (real factory = AP-20) |

The pattern is deliberate (and mirrors the SDP's own guidance): a contract's *first* reference implementation should be the smallest thing that proves the interface, not the most capable thing that could eventually sit behind it — and the smallest thing must be **labeled** as such by a machine check, not a docstring alone.

---

## 10. Provenance, registry, security

**Provenance (I-8)** — every contract call emits a `ProvenanceRecord`: actor + `AuthorizationDecision`, `ToolRef` (tool identity/version), `ArtifactRef`s (inputs/outputs, content-hashed), `Placement`, `RecordSignature`. Records chain into a `ProvenanceChain` (Merkle-linked), and `export.rs` renders records as **PROV-O** documents for interoperability with the wider provenance ecosystem. `canonical_hash` is the same canonicalization discipline as the event chain — one canonical form, reused everywhere, which is why hashes are stable across the Rust and Python implementations.

**Registry** — directory-scan discovery, capability matching against contract ids and semver ranges, and **real Ed25519 signatures** (`ed25519-dalek`) in a **TUF-adapted trust model** (ADR-0021): roles/keys/thresholds modeled on TUF, but verification is designed for **offline** bundles — `aether verify-tuf <bundle> <root>` validates without phoning home. The authoring side (`scripts/author_tuf_bundle.py`, `pynacl`) and the Rust verification side are exercised against each other by walking-skeleton-c — again, two implementations.

**Policy** — an 8-role `ROLE_CATALOG` with an action matrix (`Action` enum over graph/workflow/plugin/package operations), custom-role extension points, and a CEL-based decision surface (`PolicyEngine`, `Decision`). The decisive design point: policy decisions are **recorded on the events they admit** (`authz_decision` is a field of every graph event), so the audit trail answers "who was allowed to do this" and "did they actually do it" from the same log.

---

## 11. Compute abstraction (I-6)

`aether_exec::backend` defines the `Backend` trait over `JobSpec` / `JobHandle` / `JobState` / `Placement` / `NodeCapabilities` / `ExitRecord`. Five implementations ship:

| Backend | Reality status |
|---|---|
| `Local` | Real (this is what the golden path runs on) |
| `Slurm` | Contract-surface parity; produces real job scripts, but no real cluster is exercised in this environment (AP-15) |
| `Apptainer` | Same status as Slurm |
| `SshBatch` | Same status as Slurm |
| `AgentPool` | Same status as Slurm |

Workflows declare **computational requirements, not hardware**; the resource-descriptor schema *forbids* hardware fields, which is what makes the same AWL program runnable on a workstation, an HPC allocation, or an edge cell without modification. The conformance test runs every backend against the same contract surface — and its docstring honestly states it is a single-host equivalence exercise, with the strong cross-architecture claim reserved for G7 (MVR-04/05/12; AP-31).

---

## 12. Composition engine (multiphysics)

`aether_compose` orchestrates independently developed participants (each just a plugin behind `physics.solver` / `physics.composable`) that exchange boundary data without knowing each other exists:

- `Participant` + `FieldExchange` — who exchanges what with whom;
- `CouplingScheme` — how the loop iterates;
- **IQN-ILS (Broyden-style) acceleration** — a textbook Broyden update over coupling residuals; honest label: substrate-grade, production IQN-ILS is G4.5 (AP-14);
- **Conservation checks** — declared conservation laws (e.g. force resultant) evaluated per iteration with tolerances;
- **Divergence abort** — residual past a hard threshold kills the coupled run (I-12: a diverged coupling must be a loud failure, never a quietly wrong answer).

Two coupling cases exercise the machinery: the AGARD 445.6 flutter substrate (which *does not* claim to reproduce the published flutter speed — the test docstring says so in those words) and a 1-DOF pitch-flutter case with an analytical flutter speed, converging through the same coupling loop.

---

## 13. Environment, reproducibility, sovereign bundles

- **Environment Manifest** (`aether_envman`, I-9): pins exact versions and content hashes of plugins, environments, and inputs. Re-running a historical package is a supported operation (`envman verify/install`), not archaeology.
- **`aether export` / `aether replay`**: a project exports as a **self-verifying sovereign bundle** — manifest (sha256), graph census, workflows, and the event-chain verdict. `aether replay <bundle>` verifies the bundle and **re-executes its workflows offline**, proving the recorded results reproduce. Reproducibility here is structural (hashes + manifests + deterministic replay), not procedural ("best effort to rerun").

---

## 14. The CLI surface (I-10)

The CLI is the canonical interface — any future GUI must go through the same service layer. The command tree mirrors the phase gates:

| Group | Commands | Gate origin |
|---|---|---|
| Core | `init --profile {core,research,aerospace,developer}`, `project status`, `graph {query,as-of,chain}`, `requirement …`, `plugin list`, `package verify`, `verify-tuf`, `dark-build-check` | G0/G1 |
| Golden path | `doctor`, `demo {aircraft,structure,twin,twin-span,reproducibility}`, `run`, `explain`, `evidence export` | Wave 1 |
| Science | `uq {propagate,sobol}`, `simulate compose`, `optimize`, `twin estimate`, `experiment run`, `benchmark` | G4/G5 + Waves 2–4 |
| Domain | `geometry …`, `mesh generate`, `requirement import-sysml/report`, `manufacture plan`, `inspect acquire`, `test run` | G3/G6 |
| Deployment | `deploy …`, `envman {verify,install}` | G7 |
| Portability | `export`, `replay` | H3 |

Every command emits stable, versioned JSON with `--json`; exit codes are deterministic. `aether doctor` **probes** instead of claiming: structural analysis (beam FEM vs closed form), UQ, governor, and CalculiX/OpenFOAM are each probed, with optional probes reported optional-absent and exit 1 reserved for core failures.

The user-facing spine is deliberate: `run` records the full engineering chain (LoadCase → SimulationRun → Result → Evidence) into the project graph; `explain` returns the latest claim with value, units, analytical reference, relative error, fidelity class, validation status, and producing plugin identity; `evidence export` writes a hash-addressed bundle including the graph-chain verdict. The interface answers the engineer's actual question — *"why should I trust this number?"* — from recorded data, not narrative.

---

## 15. The scientific loop

Wave 2–4 turned the substrate into a closed scientific process, all through contracts:

1. **Adaptive fidelity.** The `FidelityEscalationGovernor` (si-essentials, behind `si.governor`) makes a deterministic escalate/stay decision from uncertainty, margin proximity, and decision sensitivity. `aether run` closes the loop: if the governor escalates after a sizing step, the run **re-executes** the quantity of interest at the escalated ladder rung (beam FEM as the high-fidelity model of the sizing deflection), records the escalated SimulationRun (marked `fidelity_escalation_of`), and stores the **measured** model-form difference between levels. Every decision is a graph `Decision` node.
2. **Surrogate-assisted optimization with verification.** `aether optimize`: DOE → GP (real training) → NSGA-III on the surrogate → **verification re-run of the real model at the optimum** with trust-envelope screening. The optimum reported is the real-model value; surrogate values are labeled as such.
3. **Experiment engine.** `aether experiment <spec>`: baseline and treatment workflows are *executed*, metrics are *measured*, and the verdict is *decided by declared gates* over measured data — never asserted.
4. **Defect-aware digital twin.** Scalar path (`aether demo twin`): synthetic as-built deviation → EKF over noisy deflection-growth measurements → corrected prediction (measured improvement ×2693 in the demonstration). Vector path (Wave 4): `twin_spanwise_loop` estimates the **spanwise EI distribution** (8 stations) over the real beam FEM. The observation matrix H is *measured, not asserted* — one extra real FEM execution per state column gives finite-difference sensitivities `d profile_j / d s_i` at nominal. The updated model is re-executed on the real FEM, so reported correction quality never comes from the linearization. The comparator is the strongest honest baseline: the best *scalar* twin, found by bisection through the real model. Result: a localized root-region stiffness deficit is recovered to 2.4/3.8% while healthy stations return to ~1.0, and max per-station error is 3.4× smaller than any uniform scale factor could achieve. Demonstration data is labeled SYNTHETIC on every record; a real rig sensor suite is Phase 5 (AP-32).

The loop's epistemology is uniform: **classification (REFERENCE/SYNTHETIC) and validation status ride on the artifacts**, so downstream consumers can filter claims by how they were produced.

---

## 16. Enforcement machinery — how the architecture stays true

### 16.1 Fitness pipeline (11 checks)

| Check | Enforces |
|---|---|
| `loc_budget` | I-1 — kernel/boundary LOC within budget |
| `token_scan` | I-3 — no prohibited coupling tokens (`if solver == X` and friends) |
| `schema_semver` | I-11 — 24 contracts / 48 schemas, all valid, semver discipline |
| `sbom` | Supply chain — SPDX 2.3 SBOM emitted per run |
| `geom_fixture_label` | I-12 — the geometry fixture *must* self-label `is_real_brep: false` or the build fails |
| `ccx_binary` | The CalculiX adapter must resolve the real binary and reproduce LE1 within tolerance; absent optional tooling is reported as `SKIP`, never a silent pass |
| `rosetta_cross_arch` | MVR-04 equivalence substrate — aarch64 native ≡ x86_64 Rosetta cas_hash; `SKIP` when Rosetta is unavailable |
| `fuzz_contracts` | I-12 — hypothesis fuzzing of the JSON contract-validation parser (200 × 24 adversarial inputs, zero unhandled exceptions) |
| `docs_links` | Documentation truth — every relative Markdown link resolves |
| `traceability` | Every traceability row has an owner, test/evidence reference, and valid status |
| `claim_scan` | No stale or unsupported capability claim in current-facing docs |

### 16.2 Dark-build (24 steps, egress-free)

`infra/ci/dark-build.sh` is the executable form of air-gap-by-design (I-5): builds the workspace, runs every test tree (Rust + Python conformance suites), the fitness pipeline, release hygiene (machine-enforced absence of scratch artifacts), and the G8-gate self-check — with **no network access**, in tens of seconds. A historical recursion bug (the gate invoking the dark-build invoking the gate) was found and fixed; the build is a DAG now, not a loop.

### 16.3 Benchmarks (11, all executed)

AETHER-BENCH-01…11 cover structural reference, aerodynamic reference, aerostructural coupling, verified optimization, surrogate substitution, uncertainty propagation, adaptive fidelity, cross-implementation reproducibility, twin update (scalar), the closed loop design→test→update, and the spanwise vector-state twin. Every benchmark executes real code and gates on declared tolerances; the suite has already caught one real bug (LHS stratification collapse in BENCH-06), which is the point of executing rather than asserting.

### 16.4 MVRs

Modular Verification Requirements (swap-two-implementations, reproducibility, cross-context) run as first-class Rust integration tests in `crates/aether_exec/tests/` — modularity claims executed as tests, with single-host scope stated honestly in the test docstrings where the strong claim awaits hardware (G7).

---

## 17. A result's lifecycle (the architecture in motion)

What actually happens on `aether run` in a project directory:

1. `init` has scaffolded the project; state lives in `<project>/.aether/state/graph.db` (project-local; `AETHER_STATE_DIR` overrides; a project is self-contained and portable).
2. The AWL workflow compiles to a DAG. The executor walks it, wrapping each step in the `{step_id, inputs}` envelope and routing by `contract::plugin` to a handler.
3. Each plugin call spawns a child process over stdio JSON-RPC (`discover → capabilities → validate → execute`) under a signed run token; the process is reaped after the response.
4. Each execution appends `NodeUpserted`/`EdgeAdded` events to the project graph — every event carrying its `authz_decision` and chained hash. The engineering chain assembles itself: LoadCase → SimulationRun → Result (with `fidelity_class`, `uncertainty`) → Evidence, cross-linked by typed edges (`simulated_by`, `verifies`, `traces_to`).
5. Provenance records (actor, tool, artifact hashes, placement) chain alongside; if the governor escalates, the escalated SimulationRun and the measured model-form difference join the chain.
6. `aether explain` reads the chain and answers the trust question; `aether evidence export` seals it into a hash-addressed bundle with the graph-chain verdict; `aether export`/`replay` can move the whole thing to an air-gapped machine and reproduce it offline.

No step of this path involves trusting a claim that was not measured and recorded.

---

## 18. Honest boundaries (read this before citing the architecture)

The strongest defensible status is **Phase 0–2 substrate + golden path + scientific loop**. Not G8, not a full platform. The following are deliberately open and labeled (full map in `workflows/paper-run/gaps.md`):

- Real OpenFOAM (AP-11) and real B-rep/OCC geometry (AP-13) — the adapters/fixtures are honest shims;
- AGARD 445.6 numerical reproduction — the compose engine is substrate-grade IQN-ILS (AP-14);
- Real HPC fleet exercise (AP-15) and the strong cross-architecture claim (AP-31) — hardware-bound, G7;
- License broker (AP-18), real rig-data twin ingestion (AP-32), real manufacturing integration (AP-20) — Phases 5–6;
- SI benchmark campaigns and the calibrated cost table — Phase 8 / G8;
- Second implementations of `aether.twin.estimator` (AP-34) and the SDP's remaining two-impl badges;
- `cargo-fuzz` Rust-side fuzzing and AP242/CGNS parser fuzz harnesses (AP-30) — the parsers themselves are G3 deliverables.

Every one of these is visible in the artifacts themselves (`is_real_brep: false`, `data_class: SYNTHETIC`, "does not reproduce the published flutter speed") — the architecture treats its own honesty as a machine-checked property, which is the only kind that survives contact with time.

---

## 19. Document map

- [`docs/architecture/AETHER-3.0.md`](architecture/AETHER-3.0.md) — target architecture and implementation brief
- [`docs/architecture/AS-IS.md`](architecture/AS-IS.md) — repository-grounded inventory
- [`docs/architecture/GAP-MATRIX.md`](architecture/GAP-MATRIX.md) — capability maturity mapping
- [`docs/roadmap/IMPLEMENTATION-PLAN.md`](roadmap/IMPLEMENTATION-PLAN.md) — phased plan and acceptance gates
- [`docs/verification/STATUS.md`](verification/STATUS.md) — current counts and gate verdicts
- [`specs/invariants.md`](../specs/invariants.md) — the twelve invariants and their enforcement
- [`specs/adr/`](../specs/adr/) — ADR-0000…0033, the reasoning behind every major decision
- [`docs/history/`](history/) — historical evaluations and alignment changelogs
- [`workflows/paper-run/gaps.md`](../workflows/paper-run/gaps.md) — the living gap log with phase mapping
- [`infra/fitness/`](../infra/fitness/) — the enforcement pipeline itself
- [`crates/`](../crates/), [`contracts/`](../contracts/), [`reference_plugins/`](../reference_plugins/) — the code this document describes
