# Aether — Current Status

**Status:** generated summary, 2026-11-24.
**Rule:** this is the single source of truth for current counts and gate
verdicts. Historical evaluations live under [`../history/`](../history/).

---

## Current capability line

Aether is a **Phase 0–2 substrate + golden path + scientific loop**, extended
with Aether 3.0 Phase 1 primitives and working Phase 2–7 vertical slices. It
is **not** a completed 3.0 platform and does not claim certification or
qualification.

| Area | Status | Evidence |
|---|---|---|
| Rust workspace | builds; 21 crates, ~27.1k Rust LOC | `cargo build --workspace` |
| Rust tests | green baseline | `cargo test --workspace` |
| Python tests | green baseline | `python3 -m pytest -q` |
| Contracts | 24 packages / 48 schemas | `contracts/*/manifest.json` |
| Graph catalog | 61 node types / 32 edge types | `contracts/graph.model/catalog/` |
| Reference physics | beam FEM, Poiseuille, CalculiX LE1, lifting line, sizing | `reference_plugins/` |
| Benchmarks | 11 benchmarks; absent optional tooling is `SKIP`, not `FAIL` | `benchmarks/` |
| Dark-build | 24 steps, egress-free by policy | `infra/ci/dark-build.sh` |
| Fitness | 11 checks; 2 optional-tool skips on this machine | `infra/fitness/run.py` |
| Policy | fail-closed RBAC; unknown role/action denied | `crates/aether_policy/src/policy.rs` |
| Units and frames | typed quantities, frame graph, solver-submission check | `crates/aether_quantities` |
| Artifacts | content-addressed local store, scoped handles | `crates/aether_artifacts` |
| ASM | lifecycle, revisions, interfaces, snapshots | `crates/aether_asm` |
| Evidence | classes, gates, waivers, release packages, `release prepare` | `crates/aether_evidence` |
| Manufacturing | genealogy, as-built snapshots, disposition and impact checks | `crates/aether_manufacturing` |
| Model governance | model cards, promotion gates, auditable registry | `crates/aether_model_registry` |
| Fidelity planning | deterministic cost/error selection and escalation | `crates/aether_fidelity` |
| Design exploration | feasibility pipeline, candidate lineage, uncertainty classes | `crates/aether_design` |
| Agent governance | advisor-only tool permissions and human-approval gates | `crates/aether_agents` |
| Workflow control flow | branch / join / bounded loop | `crates/aether_workflow` |
| Plugin sandbox | policy + enforced spawn (`spawn_sandboxed`) | `crates/aether_exec/src/sandbox.rs` |
| Release tooling | Ed25519 plugin signing + verification | `scripts/plugin-sign.py` |
| Aether 3.0 docs | architecture, runbook, threat model, traceability | `docs/` |

## Honest boundaries

- `openfoam-adapter` is a shim, not real OpenFOAM.
- `occ-geometry` is a bounding-box fixture, not real B-rep.
- Slurm/Apptainer/SSH/AgentPool backends are contract-surface fixtures.
- Manufacturing, inspection, and test are conformance fixtures; the new
  manufacturing crate records and checks evidence, it does not control machines.
- The digital twin uses synthetic data; real rig ingestion is not implemented.
- AGARD 445.6 is not numerically reproduced.
- No real HPC fleet, factory cell, or machine control is exercised.
- The model registry is a governance and audit layer, not a trainer; no model
  runtime or autonomous agent loop is implemented.
- Units/frames are enforced through `aether units check`; solver adapters do
  not yet call it automatically on every path.
- Fidelity planning uses declared errors as claims; it does not measure error
  or prove physical fidelity.
- Evidence gates and release packages are exercised through the CLI; the
  release workflow does not yet sign the package with a site key.
- Plugin sandboxing is enforced at the policy and spawn boundary; kernel-level
  isolation (seccomp, namespaces, cgroups) is an adapter, not yet implemented.
- Federation key exchange and multi-site trust are not implemented.
- Phase 7 acceptance covers chain/export/replay/tamper/DR/sandbox. Signed
  production release, SLOs, and penetration testing remain open.

## Optional-tool status on this machine

| Tool | Status | Effect |
|---|---|---|
| CalculiX `ccx` | absent | BENCH-08 reports `SKIP`; fitness `ccx_binary` reports `SKIP`; no production claim |
| Rosetta `arch -x86_64` | absent | `rosetta_cross_arch` reports `SKIP`; AP-16 stays open |

## Gate verdicts

| Gate | Verdict | Notes |
|---|---|---|
| Phase 0 truth/cleanup | complete | docs, links, baseline, optional-tool semantics |
| Phase 1 substrate | implemented for the declared scope | quantities, artifacts, ASM, evidence, policy, snapshots, CLI, release packages |
| Phase 2 vertical slice | demonstrator complete | `scripts/acceptance/phase2-wingbox.sh`, `phase2-release-package.sh` |
| Phase 3 manufacturing | vertical-slice complete | `phase3-manufacturing.sh`; no machine control |
| Phase 4 multi-physics | fidelity + control-flow demonstrator complete | `phase4-fidelity.sh`, `phase4-control-flow.sh`; no new physics domain |
| Phase 5 emerging structures | design-exploration slice complete | `phase5-design-exploration.sh`; no physical validation |
| Phase 6 adaptive intelligence | governance + agent demonstrator complete | `phase6-model-governance.sh`, `phase6-agents.sh`; no model runtime |
| Phase 7 hardening/federation | partial | `phase7-hardening.sh`, `phase7-sandbox.sh`; federation and kernel isolation open |

## Acceptance commands

```bash
bash scripts/acceptance/phase1-units-frames.sh
bash scripts/acceptance/phase2-wingbox.sh
bash scripts/acceptance/phase2-release-package.sh
bash scripts/acceptance/phase3-manufacturing.sh
bash scripts/acceptance/phase4-control-flow.sh
bash scripts/acceptance/phase4-fidelity.sh
bash scripts/acceptance/phase5-design-exploration.sh
bash scripts/acceptance/phase6-model-governance.sh
bash scripts/acceptance/phase6-agents.sh
bash scripts/acceptance/phase7-hardening.sh
bash scripts/acceptance/phase7-sandbox.sh
```

## Acceptance commands

```bash
bash scripts/acceptance/phase1-units-frames.sh
bash scripts/acceptance/phase2-wingbox.sh
bash scripts/acceptance/phase2-release-package.sh
bash scripts/acceptance/phase3-manufacturing.sh
bash scripts/acceptance/phase4-control-flow.sh
bash scripts/acceptance/phase4-fidelity.sh
bash scripts/acceptance/phase5-design-exploration.sh
bash scripts/acceptance/phase6-model-governance.sh
bash scripts/acceptance/phase6-agents.sh
bash scripts/acceptance/phase7-hardening.sh
bash scripts/acceptance/phase7-sandbox.sh
```

## How to refresh

Run:

```bash
make baseline
```

and commit the generated `docs/verification/baseline/<date>/` artifacts. Do not
hand-edit counts in README or architecture documents; link here instead.