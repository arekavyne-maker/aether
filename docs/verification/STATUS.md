# Aether — Current Status

**Status:** generated summary, 2026-11-24.
**Rule:** this is the single source of truth for current counts and gate
verdicts. Historical evaluations live under [`../history/`](../history/).

---

## Current capability line

Aether is held at **`3.0.0-rc.1`** — a production-grade, evidence-native
engineering platform with working vertical slices across Phases 0–7 and
production-grade implementations of the research methods behind ADR-0038's
promotion gate. Final `3.0.0` is reserved until the P7 evidence gate passes.
It does **not** claim certification, airworthiness, or production
qualification.

| Area | Status | Evidence |
|---|---|---|
| Rust workspace | builds; 38 crates, ~33.9k Rust LOC | `cargo build --workspace` |
| Rust tests | green baseline (100 test binaries) | `cargo test --workspace` |
| Python tests | green baseline | `python3 -m pytest -q` |
| Contracts | 32 packages / 58 schemas | `contracts/*/manifest.json` |
| Graph catalog | 78 node types / 38 edge types | `contracts/graph.model/catalog/` |
| Geometry | **real OpenCASCADE B-rep** (STEP/IGES, booleans, fillets, tessellation) | `reference_plugins/occ-geometry` |
| Meshing | **real Gmsh** tetrahedral mesher with quality metrics | `reference_plugins/mesh-gmsh` |
| Physics | beam, thermal, buckling, composite CLT, lattice — all analytics-verified; CalculiX real | `reference_plugins/` |
| Materials | cards, statistical allowables, temperature envelopes, pedigree | `crates/aether_materials` |
| Loads | flight-regime envelopes as structured data | `crates/aether_loads` |
| V&V | requirements, verification/validation cases, coverage, review decisions | `crates/aether_vv` |
| Units and frames | typed quantities + solver-submission rejection | `crates/aether_quantities` |
| Artifacts | content-addressed local store, scoped handles | `crates/aether_artifacts` |
| ASM | lifecycle, revisions, interfaces, snapshots | `crates/aether_asm` |
| Evidence | classes, credibility, gates, waivers, release packages | `crates/aether_evidence` |
| Manufacturing | genealogy, as-built, disposition, impact | `crates/aether_manufacturing` |
| Fidelity | deterministic cost/error selection | `crates/aether_fidelity` |
| Design | feasibility pipeline, lineage, uncertainty classes | `crates/aether_design` |
| Model governance | cards, promotion gates, auditable registry | `crates/aether_model_registry` |
| Model runtime | bounded inference with envelope enforcement | `crates/aether_model_runtime` |
| Agents | advisor-only, permission-gated execution loop | `crates/aether_agents`, `aether_agent_runtime` |
| Workflow | DAG, branch/join/loop, approval gates, nested runs | `crates/aether_workflow` |
| Compute | Local real; Slurm real when binaries exist, honest error otherwise | `crates/aether_exec` |
| Sandbox | policy + macOS sandbox-exec profile / Linux bwrap detection | `crates/aether_exec::sandbox` |
| Classification | data handling policy, unclassified denied | `crates/aether_classification` |
| Identity | short-lived signed tokens, expiry, revocation | `crates/aether_identity` |
| Observability | local metrics with no-leak labels | `crates/aether_observability` |
| Local console | AIXOR-inspired console with locally bundled Urbanist/Arapey, embedded argv CLI, Atlas, live jobs with cancel, project switching, 124-command surface, CLI untouched | `crates/aether_web`, `site/app`, `scripts/acceptance/web-ui.sh` |
| Desktop shell | Native macOS `wry` app with custom titlebar, in-process loopback server, native menus, recents, window state and bundled Python | `apps/desktop`, `scripts/package`, `scripts/acceptance/desktop.sh` |
| Audit | Independent findings register and 82-row task coverage matrix | `audit/REPORT.md`, `audit/task-coverage.md` |
| Interop | STEP/IGES/STL/HDF5/CSV/JSON + QIF, declared semantic loss | `python/aether_standards` |
| Benchmarks | 11 benchmarks; absent optional tooling is `SKIP`, not `FAIL` | `benchmarks/` |
| Dark-build | 24 steps, egress-free by policy | `infra/ci/dark-build.sh` |
| Fitness | 18 checks; 1 optional-tool skip on this machine | `infra/fitness/run.py` |

## Honest boundaries

- The digital twin uses synthetic data; real rig ingestion is not implemented.
- AGARD 445.6 is not numerically reproduced.
- No real HPC fleet, factory cell, or machine control is exercised.
- Composite and lattice homogenisation are estimates, not certification results.
- Plugin sandboxing is enforced at the policy and spawn boundary plus an OS
  profile; it is not a substitute for a full container/VM isolation layer.
- Federation key exchange between organisations is not implemented.
- Physical validation campaigns and certification evidence are out of scope.
- AI agents advise; authorized humans and deterministic checks decide.

## Optional-tool status on this machine

| Tool | Status | Effect |
|---|---|---|
| CalculiX `ccx` | present (vendored 2.23) | BENCH-08 and `ccx_binary` PASS |
| OpenCASCADE (OCP) | present | real B-rep active |
| Gmsh | present | real meshing active |
| Rosetta `arch -x86_64` | absent | `rosetta_cross_arch` reports `SKIP` |

## Gate verdicts

| Gate | Verdict | Notes |
|---|---|---|
| Phase 0 truth/cleanup | complete | docs, links, baseline, optional-tool semantics |
| Phase 1 substrate | complete | quantities, artifacts, ASM, evidence, policy, snapshots, V&V, release |
| Phase 2 vertical slice | complete | `phase2-wingbox.sh`, `phase2-release-package.sh` |
| Phase 3 manufacturing | complete (slice) | `phase3-manufacturing.sh`; no machine control |
| Phase 4 multi-physics | complete (slice) | `phase4-fidelity.sh`, `phase4-control-flow.sh`, `phase4-federation.sh` |
| Phase 5 emerging structures | demonstrators A/B/C/D/E | synthetic-fixture validation plans for C and E |
| Phase 6 adaptive intelligence | complete | model/agent governance plus production-grade research methods behind ADR-0038 |
| Phase 7 hardening/federation | engineering complete; evidence gate open | encryption, key rotation, signed release, restore/SLO runbook pass; real second site and independent security review required for `3.0.0` |

## Acceptance commands

```bash
for s in scripts/acceptance/*.sh; do bash "$s"; done
```

## How to refresh

Run:

```bash
make baseline
```

and commit the generated `docs/verification/baseline/<date>/` artifacts. Do not
hand-edit counts in README or architecture documents; link here instead.