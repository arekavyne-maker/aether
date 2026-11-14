# Aether


![Aether architecture: 17 platform layers (CLI, microkernel, graph, contracts, plugins, scheduler, provenance, policy, security, packaging, etc.) wrapping a single Engineering Graph — an append-only hash-chained event log feeding the closed loop from Requirement through Geometry, Mesh, Simulation, Result, Optimization, Manufacture, and Test, with measured-data evidence flowing back as model updates. EIC Pathfinder / Horizon Europe. Aether-VPSL-1.0 — vetted partners only.](./Diagram.png)

Aether is an air-gapped, hardware-agnostic, fully modular engineering fabric for emerging aerostructures. It exists to solve one problem: engineering organizations run requirements, CAD, multiphysics simulation, optimization, uncertainty quantification, manufacturing, and test on a dozen disconnected tools, each vendor-locked, each with its own notion of truth. Aether replaces that patchwork with **one Engineering Graph and one bidirectional digital thread** that every discipline reads from and writes to — independent of which CAD kernel, which solver, which compute backend, or which hardware happens to be underneath.

Aether is not a CAD system, a solver, a PLM suite, or a notebook environment. It runs no production physics engine of its own. It is the connective, governing, and reproducibility layer that makes all of those tools interchangeable parts of the same engineering process, rather than the process itself.

## Dual-use framing

Aether is, by design, a **dual-use** engineering platform. The same substrate that supports the certification evidence chain for a civilian UAV wing rib — fully traceable requirement → design → simulation → manufacture → inspection → test → model-update loops, with hash-chained provenance and offline reproducibility — also supports the same kind of traceable, vendor-neutral engineering process for any other advanced airframe or aerostructure application, including those with defence or national-security relevance. That symmetry is intentional: the contract-driven plugin architecture, the signed offline bundles, the deterministic event-sourced graph, and the air-gap-first design are *more* useful, not less, when the engineering environment has to withstand rigorous audit, restricted-toolchain access, and long reproducibility windows. Nothing in the substrate is aerostructure-specific; the *contracts* and the *plugins* are where the application lives.

This dual-use nature is part of the EIC Pathfinder scope: the programme explicitly funds early-stage research whose civilian track record strengthens European strategic autonomy in advanced air mobility, novel propulsion, and high-integrity structural design. Aether's contribution to that scope is a *governing layer* — the open, vetted, audit-friendly substrate on which both civilian and strategic-aerostructure engineering workflows can be assembled without per-vendor lock-in.

## The core idea

Everything domain-specific is a plugin. The kernel knows about identity, content-addressed storage, the graph, workflow execution, eventing, capability discovery, provenance, policy, and plugin lifecycle — and nothing else. No CFD solver, FEA solver, CAD kernel, optimizer, or manufacturing system is allowed to live in or link against the kernel; each exists behind a **contract** — a versioned JSON Schema plus a conformance suite — and the kernel only ever talks to contracts, never to a specific vendor's implementation.

This is enforced, not aspirational. The `aether_kernel` crate carries a hard LOC budget precisely so it cannot quietly absorb domain logic. A prohibited-token scan (`infra/fitness/token_scan.py`) fails the build if anything in the codebase branches on `if solver == X` or an equivalent vendor check. A dependency scan blocks any third-party domain library (a CFD package, a CAD kernel, an ML framework) from being linked into the kernel at all. The result is a platform where swapping a solver, a CAD system, or a compute backend never touches the Engineering Graph's schema or a workflow's definition — the same AWL (Aether Workflow Language) program runs unchanged against whichever plugin satisfies its contract.

Nineteen contracts currently define the domain surface — `physics.solver`, `geometry.kernel`, `geometry.recipe`, `mesh.engine`, `field.model`, `compute.backend`, `io.format`, `graph.model`, `plugin.manifest`, `requirement`, `physics.composable`, `uq.method`, `si.governor`, `optimizer`, `twin.estimator`, `surrogate`, `manufacturing.process`, `inspection.device`, and `test.rig` — each living under `contracts/` as its own versioned schema package with a matching conformance suite. Every contract obeys a **two-implementation rule**: no contract is considered stable until at least two independent implementations exist and are run against each other on every release, which is the actual proof that the contract abstracts something real rather than one plugin's accidental interface.

## How the platform is put together

**The kernel (`crates/aether_kernel`)** provides the small set of primitives everything else builds on: identity, a content-addressed artifact store, portable execution primitives, and the plugin lifecycle contract.

**The Engineering Graph (`crates/aether_graph`)** is the single source of engineering truth. It's event-sourced — every change is an appended event, and the current graph state is a replayable projection of that log — with a query language (AQL) over it. This is what lets a requirement, a geometry feature, a mesh, a simulation result, an optimization study, and a manufacturing definition all live as typed nodes in the same structure, cross-referenced instead of scattered across files.

**Workflows (`crates/aether_workflow`)** are written in AWL, compiled to a DAG, and run by the executor in `crates/aether_exec`, which talks to plugins over a stdio JSON-RPC substrate (`python/aether_sdk`) so that plugins can be written in whatever language makes sense for the domain — the reference plugins here are Python, but the protocol doesn't care. The executor's scheduler is hardware-agnostic by construction: workflows declare computational requirements, not hardware, and the same workflow can run on a workstation, an HPC allocation, or an edge cell without modification.

**Provenance (`crates/aether_provenance`)** makes every artifact, decision, and execution traceable to its inputs, tool versions, parameters, and environment through a hash-chained audit log — not an afterthought bolted onto results, but a mandatory output of every plugin contract call. **Policy (`crates/aether_policy`)** layers CEL-based rule evaluation and RBAC over graph and workflow operations. **The registry (`crates/aether_registry`)** handles capability discovery and artifact signing/verification using real Ed25519 signatures (via `ed25519-dalek`) in a TUF-inspired trust model, so an offline bundle can be verified without ever phoning home. **`crates/aether_compose`** is where multiphysics coupling lives — an IQN-ILS–based orchestration layer that lets independently-developed structural and fluid solvers exchange boundary data and converge to a coupled solution without either one knowing the other exists.

**The CLI (`crates/aether_cli`)** is the canonical interface (`aether ...`), backed by the same service layer any future GUI would have to use — there is no parallel implementation path, by design.

**Reproducibility** is structural rather than procedural: an Environment Manifest (`crates/aether_envman`) pins exact versions and content hashes of plugins, environments, and inputs, so re-running a historical package is a supported operation, not a best-effort archaeology exercise.

## What a contract implementation actually looks like

The abstraction is easiest to see in the `physics.solver` contract, which has real, working implementations today rather than just a schema on paper. `reference_plugins/fea-reference-beam` is a genuine Euler-Bernoulli beam finite-element solver — real stiffness-matrix assembly, real linear solve — checked against closed-form deflection formulas. `reference_plugins/cfd-reference-minimal` solves the Poiseuille flow problem analytically and exactly. `reference_plugins/calculix-adapter` is a native-binary adapter: it writes a real CalculiX input deck, invokes the actual `ccx` solver binary, and parses the real `.frd` output, validated against the NAFEMS LE1 cantilever benchmark. Because two of these are independent, real implementations of the same structural problem, they can be — and are — cross-checked against each other directly, which is the two-implementation rule doing its job rather than sitting in a spec document. `reference_plugins/openfoam-adapter` shows the adapter pattern the other direction, wrapping a reference fluid solver behind the same contract surface an eventual real OpenFOAM integration would use.

The `geometry.kernel` and `geometry.recipe` contracts are exercised today by a lighter reference implementation — a feature-tree interpreter and a bounding-box-level geometry engine — deliberately minimal, in the same spirit as the SDP's own guidance that a contract's *first* reference implementation should be the smallest thing that proves the interface, not the most capable thing that could eventually sit behind it. `uq-basic`, `optimizer-reference`, and `surrogates-core` implement real Sobol/Latin-hypercube sampling, a real NSGA-II multi-objective optimizer, and real Gaussian-process/POD surrogate construction, run against analytic benchmark problems. `structures-sizing` implements the wing-box boom-idealization sizing chain (stress, shear buckling, deflection, mass, reserve margins), and `aero-lifting-line` implements Glauert Fourier collocation of the lifting-line equation — the elliptical planform is a fixed point of its discretization, so it verifies against the classical closed form at machine precision. `twin-kf` implements a real Kalman-filter family (linear, extended, unscented, particle) plus a multivariate linear KF (`ekf_vector`) that estimates a spanwise stiffness distribution station by station — a vector state inverted through an observation matrix measured by finite differences of the real beam FEM — rather than a single scalar growth factor.

## Verifying the platform holds together

Aether treats its own architectural claims as testable, not just documented. The **fitness pipeline** (`infra/fitness/`) mechanically checks the kernel's LOC budget, scans for prohibited coupling tokens, validates contract semver discipline, checks the SBOM, fuzzes the contract-validation parser against adversarial input, and confirms the CalculiX adapter's binary and validated case are still resolvable — eight checks in total, run as part of a 24-step **dark-build**, an egress-free CI pipeline that proves the whole substrate builds and passes with no network access at all, which is the literal, executable form of the air-gap-by-design invariant rather than a policy statement. Modular Verification Requirements (MVRs) — swap-two-implementations tests, reproducibility tests, cross-execution-context tests — are run as first-class Rust integration tests in `crates/aether_exec/tests/`, directly exercising the plugin substrate rather than asserting the property in prose.

## Where this sits on the roadmap

`SDP.md` lays out the full nine-phase build sequence, from the initial kernel and contract skeleton through multiphysics composition, digital twins, manufacturing integration, HPC/edge federation, and autonomous scientific-intelligence workflows — the kind of scope that takes a full team over a multi-year horizon. What exists today is the foundation — a working kernel, a working graph, a working plugin/contract substrate with real reference physics behind the solver, estimation, and UQ contracts, working provenance and signing, and a CI discipline that keeps the architecture honest as it grows — plus the user-facing spine and the scientific loop built on top of it: the golden path (`doctor`, five demos, `init --profile`, `run`, `explain`, `evidence export`), adaptive-fidelity escalation driven by the `si.governor` contract, surrogate-assisted optimization whose optimum is re-verified against the real model, an executable experiment engine whose verdicts come from measurement, and a defect-aware digital twin in both scalar and spanwise vector-state form. The whole surface is exercised by an eleven-benchmark suite (11/11 passing, every number from actual execution on the development machine) and the test suites behind it (315 tests passing — 114 Rust + 201 Python, as of 2026-09-12). That is the early stretch of the SDP sequence done properly — substrate plus golden path plus scientific loop — not the platform at G8, and the project's own evaluation discipline (`README-EVAL.md`) says exactly that. `workflows/paper-run/gaps.md` is the living record of what's being worked on next — and of what is deliberately left open, with the phase that resolves it — and is worth reading alongside this document for anyone tracking progress in detail; `specs/adr/` holds the reasoning behind every major architectural decision along the way.

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

- [`SDP.md`](./SDP.md) — the complete architecture, contract catalog, and phased build plan; the normative document everything else here summarizes.
- [`specs/invariants.md`](./specs/invariants.md) — the twelve architectural invariants and their enforcement mechanisms in full.
- [`specs/adr/`](./specs/adr/) — the Architecture Decision Records behind every major design choice.
- [`workflows/paper-run/gaps.md`](./workflows/paper-run/gaps.md) — current work in progress and open items.
- [`README-EVAL.md`](./README-EVAL.md) — line-by-line status evaluation against the SDP, with verification commands and a fresh machine-local G8-gate verdict.
- [`ALIGNMENT_CHANGELOG.md`](./ALIGNMENT_CHANGELOG.md) and [`ALIGNMENT_CHANGELOG_2026-09-04.md`](./ALIGNMENT_CHANGELOG_2026-09-04.md) — sequential audit trail of the alignment passes that brought the substrate to its current state.
- `LICENSE` — the **Aether Vetted-Partner Source-Available License v1.0** under which this software is distributed. Use of this code outside the EIC Pathfinder consortium and its EU-vetted partner set is **not permitted**; users and redistributors are responsible for compliance with applicable export-control, sanctions, and end-use law.
