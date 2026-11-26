# Aether


![Aether architecture: 17 platform layers (CLI, microkernel, graph, contracts, plugins, scheduler, provenance, policy, security, packaging, etc.) wrapping a single Engineering Graph — an append-only hash-chained event log feeding the closed loop from Requirement through Geometry, Mesh, Simulation, Result, Optimization, Manufacture, and Test, with measured-data evidence flowing back as model updates.](./Diagram.png)

Aether is an air-gapped, hardware-agnostic, fully modular engineering fabric for emerging aerostructures. It exists to solve one problem: engineering organizations run requirements, CAD, multiphysics simulation, optimization, uncertainty quantification, manufacturing, and test on a dozen disconnected tools, each vendor-locked, each with its own notion of truth. Aether replaces that patchwork with **one Engineering Graph and one bidirectional digital thread** that every discipline reads from and writes to — independent of which CAD kernel, which solver, which compute backend, or which hardware happens to be underneath.

Aether is not a CAD system, a solver, a PLM suite, or a notebook environment. It runs no production physics engine of its own. It is the connective, governing, and reproducibility layer that makes all of those tools interchangeable parts of the same engineering process, rather than the process itself.

## Dual-use framing

Aether is, by design, a **dual-use** engineering platform. The same substrate that supports the certification evidence chain for a civilian UAV wing rib — fully traceable requirement → design → simulation → manufacture → inspection → test → model-update loops, with hash-chained provenance and offline reproducibility — also supports the same kind of traceable, vendor-neutral engineering process for any other advanced airframe or aerostructure application, including those with defence or national-security relevance. That symmetry is intentional: the contract-driven plugin architecture, the signed offline bundles, the deterministic event-sourced graph, and the air-gap-first design are *more* useful, not less, when the engineering environment has to withstand rigorous audit, restricted-toolchain access, and long reproducibility windows. Nothing in the substrate is aerostructure-specific; the *contracts* and the *plugins* are where the application lives.

The dual-use posture is architectural, not a funding claim: the same contract-driven, signed, offline-verifiable substrate is useful whenever an engineering environment must withstand audit, restricted toolchains, and long reproducibility windows. Funding and affiliation statements are deliberately kept out of this README until they are confirmed and legally reviewed.

## The core idea

Everything domain-specific is a plugin. The kernel knows about identity, content-addressed storage, the graph, workflow execution, eventing, capability discovery, provenance, policy, and plugin lifecycle — and nothing else. No CFD solver, FEA solver, CAD kernel, optimizer, or manufacturing system is allowed to live in or link against the kernel; each exists behind a **contract** — a versioned JSON Schema plus a conformance suite — and the kernel only ever talks to contracts, never to a specific vendor's implementation.

This is enforced, not aspirational. The `aether_kernel` crate carries a hard LOC budget precisely so it cannot quietly absorb domain logic. A prohibited-token scan (`infra/fitness/token_scan.py`) fails the build if anything in the codebase branches on `if solver == X` or an equivalent vendor check. A dependency scan blocks any third-party domain library (a CFD package, a CAD kernel, an ML framework) from being linked into the kernel at all. The result is a platform where swapping a solver, a CAD system, or a compute backend never touches the Engineering Graph's schema or a workflow's definition — the same AWL (Aether Workflow Language) program runs unchanged against whichever plugin satisfies its contract.

Twenty-four contracts currently define the domain surface — `physics.solver`, `geometry.kernel`, `geometry.recipe`, `mesh.engine`, `field.model`, `compute.backend`, `io.format`, `graph.model`, `plugin.manifest`, `requirement`, `physics.composable`, `uq.method`, `si.governor`, `optimizer`, `twin.estimator`, `surrogate`, `manufacturing.process`, `inspection.device`, `test.rig`, `quantities`, `frames`, `asm`, `evidence`, and `model-card` — each living under `contracts/` as its own versioned schema package with a matching conformance suite. Every contract obeys a **two-implementation rule**: no contract is considered stable until at least two independent implementations exist and are run against each other on every release, which is the actual proof that the contract abstracts something real rather than one plugin's accidental interface.

## How the platform is put together

**The kernel (`crates/aether_kernel`)** provides the small set of primitives everything else builds on: identity, a content-addressed artifact store, portable execution primitives, and the plugin lifecycle contract.

**The Engineering Graph (`crates/aether_graph`)** is the single source of engineering truth. It's event-sourced — every change is an appended event, and the current graph state is a replayable projection of that log — with a query language (AQL) over it. This is what lets a requirement, a geometry feature, a mesh, a simulation result, an optimization study, and a manufacturing definition all live as typed nodes in the same structure, cross-referenced instead of scattered across files.

**Workflows (`crates/aether_workflow`)** are written in AWL, compiled to a DAG, and run by the executor in `crates/aether_exec`, which talks to plugins over a stdio JSON-RPC substrate (`python/aether_sdk`) so that plugins can be written in whatever language makes sense for the domain — the reference plugins here are Python, but the protocol doesn't care. The executor's scheduler is hardware-agnostic by construction: workflows declare computational requirements, not hardware, and the same workflow can run on a workstation, an HPC allocation, or an edge cell without modification.

**Provenance (`crates/aether_provenance`)** makes every artifact, decision, and execution traceable to its inputs, tool versions, parameters, and environment through a hash-chained audit log — not an afterthought bolted onto results, but a mandatory output of every plugin contract call. **Policy (`crates/aether_policy`)** is a fail-closed RBAC decision point: unknown roles and unknown actions are denied, and denial is recorded on the event it blocks. **The registry (`crates/aether_registry`)** handles capability discovery and artifact signing/verification using real Ed25519 signatures (via `ed25519-dalek`) in a TUF-inspired trust model, so an offline bundle can be verified without ever phoning home. **`crates/aether_compose`** is where multiphysics coupling lives — an IQN-ILS–based orchestration layer that lets independently-developed structural and fluid solvers exchange boundary data and converge to a coupled solution without either one knowing the other exists. Phase 1 adds four small primitives: **`aether_quantities`** (typed units and coordinate frames), **`aether_artifacts`** (content-addressed artifact store), **`aether_asm`** (lifecycle, interfaces, configuration snapshots), and **`aether_evidence`** (evidence classes, gates, waivers, release packages).

**The CLI (`crates/aether_cli`)** is the canonical interface (`aether ...`), backed by the same service layer any future GUI would have to use — there is no parallel implementation path, by design.

**Reproducibility** is structural rather than procedural: an Environment Manifest (`crates/aether_envman`) pins exact versions and content hashes of plugins, environments, and inputs, so re-running a historical package is a supported operation, not a best-effort archaeology exercise.

## What a contract implementation actually looks like

The abstraction is easiest to see in the `physics.solver` contract, which has real, working implementations today rather than just a schema on paper. `reference_plugins/fea-reference-beam` is a genuine Euler-Bernoulli beam finite-element solver — real stiffness-matrix assembly, real linear solve — checked against closed-form deflection formulas. `reference_plugins/cfd-reference-minimal` solves the Poiseuille flow problem analytically and exactly. `reference_plugins/calculix-adapter` is a native-binary adapter: it writes a real CalculiX input deck, invokes the actual `ccx` solver binary, and parses the real `.frd` output, validated against the NAFEMS LE1 cantilever benchmark. Because two of these are independent, real implementations of the same structural problem, they can be — and are — cross-checked against each other directly, which is the two-implementation rule doing its job rather than sitting in a spec document. `reference_plugins/openfoam-adapter` shows the adapter pattern the other direction, wrapping a reference fluid solver behind the same contract surface an eventual real OpenFOAM integration would use.

The `geometry.kernel` and `geometry.recipe` contracts are exercised today by a lighter reference implementation — a feature-tree interpreter and a bounding-box-level geometry engine — deliberately minimal, in the same spirit as the SDP's own guidance that a contract's *first* reference implementation should be the smallest thing that proves the interface, not the most capable thing that could eventually sit behind it. `uq-basic`, `optimizer-reference`, and `surrogates-core` implement real Sobol/Latin-hypercube sampling, a real NSGA-II multi-objective optimizer, and real Gaussian-process/POD surrogate construction, run against analytic benchmark problems. `structures-sizing` implements the wing-box boom-idealization sizing chain (stress, shear buckling, deflection, mass, reserve margins), and `aero-lifting-line` implements Glauert Fourier collocation of the lifting-line equation — the elliptical planform is a fixed point of its discretization, so it verifies against the classical closed form at machine precision. `twin-kf` implements a real Kalman-filter family (linear, extended, unscented, particle) plus a multivariate linear KF (`ekf_vector`) that estimates a spanwise stiffness distribution station by station — a vector state inverted through an observation matrix measured by finite differences of the real beam FEM — rather than a single scalar growth factor.

## Verifying the platform holds together

Aether treats its own architectural claims as testable, not just documented. The **fitness pipeline** (`infra/fitness/`) mechanically checks the kernel's LOC budget, scans for prohibited coupling tokens, validates contract semver discipline, checks the SBOM, fuzzes the contract-validation parser against adversarial input, verifies documentation links, validates the traceability matrix, scans for stale claims, and probes optional external tools (CalculiX, Rosetta) without turning their absence into a false platform failure — eleven checks in total, run as part of a 24-step **dark-build**, an egress-free CI pipeline that proves the whole substrate builds and passes with no network access at all, which is the literal, executable form of the air-gap-by-design invariant rather than a policy statement. Modular Verification Requirements (MVRs) — swap-two-implementations tests, reproducibility tests, cross-execution-context tests — are run as first-class Rust integration tests in `crates/aether_exec/tests/`, directly exercising the plugin substrate rather than asserting the property in prose.

## Where this sits on the roadmap

The target architecture is now [`docs/architecture/AETHER-3.0.md`](docs/architecture/AETHER-3.0.md). It describes the full adaptive-aerostructures programme — ASM, typed units and frames, evidence gates, manufacturing genealogy, multi-fidelity planning, model governance, and bounded agents — as a target, not as shipped capability. The repository-grounded inventory is [`docs/architecture/AS-IS.md`](docs/architecture/AS-IS.md), the capability gaps are in [`docs/architecture/GAP-MATRIX.md`](docs/architecture/GAP-MATRIX.md), and the phase plan is [`docs/roadmap/IMPLEMENTATION-PLAN.md`](docs/roadmap/IMPLEMENTATION-PLAN.md).

What exists today is the foundation: a working kernel, graph, plugin/contract substrate, real reference physics, provenance and signing, and an honest CI discipline — plus the user-facing spine and scientific loop (golden path, adaptive-fidelity escalation, surrogate-assisted optimization with real-model verification, executable experiments, and scalar/vector-state digital twins). Current counts and gate verdicts live in [`docs/verification/STATUS.md`](docs/verification/STATUS.md); historical evaluations are under [`docs/history/`](docs/history/). `workflows/paper-run/gaps.md` remains the living open-items log, and `specs/adr/` holds the decision record set.

## Repository layout

| Path | Purpose |
|---|---|
| `crates/` | Canonical CLI and the Rust engineering substrate, including the local web server |
| `contracts/` | Versioned JSON Schema contracts and conformance suites |
| `experimental/web/` | Marketing preview and the embedded zero-build browser console |
| `experimental/desktop/` | Shared native shell used by the macOS and Windows/Linux apps |
| `experimental/macos/`, `experimental/windows/`, `experimental/linux/` | Platform packaging, icons and installers |
| `scripts/acceptance/` | End-to-end gates for every phase and every application surface |
| `docs/`, `specs/adr/`, `audit/` | Operator docs, decisions and the independent audit |
| `infra/fitness/`, `infra/ci/` | Architecture fitness checks and the egress-free dark build |

The CLI remains canonical. Application folders only present and drive it; they
never reimplement engineering logic.

## Quick start (the golden path)

The shortest honest path from nothing to a reproducible engineering result:

```bash
./install.sh                 # builds the CLI, installs a launcher, runs aether doctor
aether doctor                # what works on this machine — probed, not claimed
aether demo aircraft         # deterministic offline demos — aircraft | structure |
                             #   twin | twin-span | reproducibility
aether init wing-study --profile aerospace
cd wing-study
aether run                   # executes the project workflow, records the graph chain
aether explain               # why the result should be trusted: model, fidelity, error
aether evidence export       # machine-readable evidence bundle + manifest
aether benchmark             # AETHER-BENCH-01..11, from actual execution
```

Beyond the golden path, the scientific loop is exposed directly: `aether optimize` runs DOE → surrogate → multi-objective search with the optimum re-verified on the real model, and `aether experiment <spec.yaml>` executes baseline-vs-treatment arms and decides the verdict from declared gates over measured metrics.

`aether doctor` degrades gracefully: missing optional solvers (OpenFOAM, CalculiX) are reported as optional-absent, never as platform failures. The core path above runs entirely offline on the bundled reference implementations; every number it prints comes from a computation it just performed. Project state (graph, evidence) lives inside the project directory (`.aether/`), so a project is self-contained and portable.

## Building it

From source, the same thing the installer does, by hand:

```bash
# Rust workspace — kernel, graph, workflow engine, CLI, exec service
cargo build --workspace --release
cargo test --workspace

# Python plugin SDK and reference plugins
python3 -m pip install -r requirements-dev.txt   # or --no-index --find-links vendor/wheels for the air-gapped path
pip install -e python/aether_sdk -e python/aether_geometry -e python/aether_requirements -e python/aether_testing
pytest python/*/tests/ reference_plugins/*/tests/

# Architecture fitness pipeline
python3 infra/fitness/run.py

# Full offline dark-build
bash infra/ci/dark-build.sh
```

To exercise the real CalculiX adapter, install CalculiX first (`apt-get install calculix-ccx` on Debian/Ubuntu, `brew install calculix` on macOS) and point the adapter at the resulting binary with `CCX_BIN`.

## Reading further

- [`docs/architecture/AETHER-3.0.md`](docs/architecture/AETHER-3.0.md) — the target architecture and implementation brief.
- [`docs/architecture/AS-IS.md`](docs/architecture/AS-IS.md) — repository-grounded inventory.
- [`docs/architecture/GAP-MATRIX.md`](docs/architecture/GAP-MATRIX.md) — capability maturity and risk mapping.
- [`docs/roadmap/IMPLEMENTATION-PLAN.md`](docs/roadmap/IMPLEMENTATION-PLAN.md) — dependency-ordered phases and acceptance gates.
- [`docs/verification/STATUS.md`](docs/verification/STATUS.md) — current counts and gate verdicts.
- [`docs/verification/TRACEABILITY-MATRIX.md`](docs/verification/TRACEABILITY-MATRIX.md) — requirements to tests and evidence.
- [`docs/threat-model/THREAT-MODEL.md`](docs/threat-model/THREAT-MODEL.md) — trust boundaries, attacks, mitigations, residual risk.
- [`docs/research/RESEARCH-REGISTER.md`](docs/research/RESEARCH-REGISTER.md) — sources, evidence class, applicability, limitations.
- [`specs/invariants.md`](specs/invariants.md) — the twelve architectural invariants and their enforcement mechanisms.
- [`specs/adr/`](specs/adr/) — the Architecture Decision Records behind every major design choice.
- [`workflows/paper-run/gaps.md`](workflows/paper-run/gaps.md) — current work in progress and open items.
- [`docs/history/`](docs/history/) — historical status evaluations and alignment changelogs.
- `LICENSE` — the **Aether Proprietary License (Crocell-Proprietary)** under which this software is distributed. The Software is made available to **authorised recipients only**; use, redistribution, or public hosting by anyone without a written agreement with the Licensor is **not permitted**. Users and redistributors are responsible for compliance with applicable export-control, sanctions, and end-use law.
