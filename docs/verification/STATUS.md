# Aether — Current Status

**Status:** generated summary, 2026-11-24.
**Rule:** this is the single source of truth for current counts and gate
verdicts. Historical evaluations live under [`../history/`](../history/).

---

## Current capability line

Aether is a **Phase 0–2 substrate + golden path + scientific loop**, now
extended with the first Aether 3.0 Phase 1 primitives. It is **not** a
completed 3.0 platform and does not claim certification or qualification.

| Area | Status | Evidence |
|---|---|---|
| Rust workspace | builds; 16 crates; ~22.8k Rust LOC | `cargo build --workspace` |
| Rust tests | green baseline | `cargo test --workspace` |
| Python tests | green baseline | `python3 -m pytest -q` |
| Contracts | 24 packages / 48 schemas | `contracts/*/manifest.json` |
| Graph catalog | 61 node types / 32 edge types | `contracts/graph.model/catalog/` |
| Reference physics | beam FEM, Poiseuille, CalculiX LE1, lifting line, sizing | `reference_plugins/` |
| Benchmarks | 11 benchmarks; absent optional tooling is `SKIP`, not `FAIL` | `benchmarks/` |
| Dark-build | 24 steps, egress-free by policy | `infra/ci/dark-build.sh` |
| Fitness | 11 checks; 2 optional-tool skips on this machine | `infra/fitness/run.py` |
| Policy | fail-closed RBAC; unknown role/action denied | `crates/aether_policy/src/policy.rs` |
| Units and frames | typed quantities + frame graph | `crates/aether_quantities` |
| Artifacts | content-addressed local store, scoped handles | `crates/aether_artifacts` |
| ASM | lifecycle, revisions, interfaces, snapshots | `crates/aether_asm` |
| Evidence | classes, gates, waivers, release packages | `crates/aether_evidence` |
| Model cards | contract only; registry/runtime not implemented | `contracts/model-card/` |
| Plugin sandbox | **not implemented** until Phase 7 | `crates/aether_exec/src/local.rs` |
| Agent runtime | **not implemented** until Phase 6 | — |
| Aether 3.0 docs | Phase 0 set complete | `docs/architecture/`, `docs/roadmap/` |

## Honest boundaries

- `openfoam-adapter` is a shim, not real OpenFOAM.
- `occ-geometry` is a bounding-box fixture, not real B-rep.
- Slurm/Apptainer/SSH/AgentPool backends are contract-surface fixtures.
- Manufacturing, inspection, and test are conformance fixtures.
- The digital twin uses synthetic data; real rig ingestion is not implemented.
- AGARD 445.6 is not numerically reproduced.
- No real HPC fleet, factory cell, or machine control is exercised.
- AI agents and the model registry runtime are not implemented.
- Units/frames are enforced in the new library and CLI boundaries; solver
  adapters do not yet consume typed quantities end to end.
- Evidence gates are exercised through `aether evidence gate`; release
  packages are a library type, not yet a full release workflow.

## Optional-tool status on this machine

| Tool | Status | Effect |
|---|---|---|
| CalculiX `ccx` | absent | BENCH-08 reports `SKIP`; fitness `ccx_binary` reports `SKIP`; no production claim |
| Rosetta `arch -x86_64` | absent | `rosetta_cross_arch` reports `SKIP`; AP-16 stays open |

## Gate verdicts

| Gate | Verdict | Notes |
|---|---|---|
| Phase 0 truth/cleanup | complete | docs, links, baseline, optional-tool semantics |
| Phase 1 substrate | partial | quantities, artifacts, ASM, evidence, policy, snapshots, CLI |
| Phase 2 vertical slice | demonstrator complete | `scripts/acceptance/phase2-wingbox.sh` |
| Phase 3 manufacturing | not started | genealogy/as-built nodes and contracts staged |
| Phase 4 multi-physics | not started | fidelity/UQ |
| Phase 5 emerging structures | not started | one vertical at a time |
| Phase 6 adaptive intelligence | not started | model-card contract only |
| Phase 7 hardening/federation | not started | sandbox/restore/federation |

## How to refresh

Run:

```bash
make baseline
```

and commit the generated `docs/verification/baseline/<date>/` artifacts. Do not
hand-edit counts in README or architecture documents; link here instead.
