# Aether — Current Status

**Status:** generated summary, 2026-11-24.
**Rule:** this is the single source of truth for current counts and gate
verdicts. Historical evaluations live under [`../history/`](../history/).

---

## Current capability line

Aether is a **Phase 0–2 substrate + golden path + scientific loop**, extended
with Aether 3.0 Phase 1 primitives and targeted Phase 3–7 vertical-slice
artifacts. It is **not** a completed 3.0 platform and does not claim
certification or qualification.

| Area | Status | Evidence |
|---|---|---|
| Rust workspace | builds; 19 crates, ~24.6k Rust LOC | `cargo build --workspace` |
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
| Manufacturing | genealogy, as-built snapshots, disposition and impact checks | `crates/aether_manufacturing` |
| Model governance | model cards, promotion gates, auditable registry | `crates/aether_model_registry` |
| Fidelity planning | deterministic cost/error selection and escalation | `crates/aether_fidelity` |
| Plugin sandbox | **not implemented** until Phase 7 | `crates/aether_exec/src/local.rs` |
| Agent runtime | **not implemented** until Phase 6 | — |
| Aether 3.0 docs | Phase 0 set complete | `docs/architecture/`, `docs/roadmap/` |

## Honest boundaries

- `openfoam-adapter` is a shim, not real OpenFOAM.
- `occ-geometry` is a bounding-box fixture, not real B-rep.
- Slurm/Apptainer/SSH/AgentPool backends are contract-surface fixtures.
- Manufacturing, inspection, and test are conformance fixtures; the new
  manufacturing crate records and checks evidence, it does not control machines.
- The digital twin uses synthetic data; real rig ingestion is not implemented.
- AGARD 445.6 is not numerically reproduced.
- No real HPC fleet, factory cell, or machine control is exercised.
- AI agents and a model runtime are not implemented; the model registry is a
  governance and audit layer, not a trainer.
- Units/frames are enforced in the new library and CLI boundaries; solver
  adapters do not yet consume typed quantities end to end.
- Fidelity planning uses declared errors as claims; it does not measure error
  or prove physical fidelity.
- Evidence gates are exercised through `aether evidence gate`; release
  packages are a library type, not yet a full release workflow.
- Phase 7 is only partially demonstrated by chain/export/replay/tamper/DR-drill
  acceptance. Sandboxing, federation, signed production release, and SLOs
  remain open.

## Optional-tool status on this machine

| Tool | Status | Effect |
|---|---|---|
| CalculiX `ccx` | absent | BENCH-08 reports `SKIP`; fitness `ccx_binary` reports `SKIP`; no production claim |
| Rosetta `arch -x86_64` | absent | `rosetta_cross_arch` reports `SKIP`; AP-16 stays open |

## Gate verdicts

| Gate | Verdict | Notes |
|---|---|---|
| Phase 0 truth/cleanup | complete | docs, links, baseline, optional-tool semantics |
| Phase 1 substrate | implemented for the declared scope | quantities, artifacts, ASM, evidence, policy, snapshots, CLI |
| Phase 2 vertical slice | demonstrator complete | `scripts/acceptance/phase2-wingbox.sh` |
| Phase 3 manufacturing | vertical-slice complete | `scripts/acceptance/phase3-manufacturing.sh`; no machine control |
| Phase 4 multi-physics | fidelity planner demonstrator complete | `scripts/acceptance/phase4-fidelity.sh`; no new physics domain |
| Phase 5 emerging structures | not started | one vertical at a time |
| Phase 6 adaptive intelligence | governance demonstrator complete | `scripts/acceptance/phase6-model-governance.sh`; no model runtime/agents |
| Phase 7 hardening/federation | partial | `scripts/acceptance/phase7-hardening.sh`; sandbox/federation open |

## Acceptance commands

```bash
bash scripts/acceptance/phase2-wingbox.sh
bash scripts/acceptance/phase3-manufacturing.sh
bash scripts/acceptance/phase4-fidelity.sh
bash scripts/acceptance/phase6-model-governance.sh
bash scripts/acceptance/phase7-hardening.sh
```

## How to refresh

Run:

```bash
make baseline
```

and commit the generated `docs/verification/baseline/<date>/` artifacts. Do not
hand-edit counts in README or architecture documents; link here instead.