# Aether — Implementation Plan (Reference)

This document is a local reference; the authoritative source is [SDP.md](../SDP.md) (in the project root). This file exists so that the plan lives with the code that implements it, and so that contributors can navigate from ADRs and contracts to the phase plan and back.

## Phased sequence (§18)

```
P0 contracts ─► P1 core/graph ─► P2 plugins/exec/prov ─► P3 geometry/MBSE (GA-1)
                                                          │
                                                          ▼
                                          P4 multiphysics/SI/UQ ─► P5 opt/twins/VP
                                                                       │
                                                                       ▼
                                          P6 manufacturing/test (GA-2) ─► P7 air-gap/HPC/edge
                                                                              │
                                                                              ▼
                                                                  P8 advanced SI/autonomy
```

| Phase | Theme | Duration | Gate |
|---|---|---|---|
| 0 | Architecture and contracts | 10 weeks | G0 |
| 1 | Minimal Core + CLI + Engineering Graph | 14 weeks | G1 |
| 2 | Plugin Runtime + Execution Engine + Provenance | 14 weeks | G2 |
| 3 | Geometry + MBSE + First Simulation Adapters | 16 weeks | G3 (GA-1) |
| 4 | Multiphysics + Scientific Intelligence + UQ | 18 weeks | G4 |
| 5 | Optimization + Digital Twins + Virtual Prototyping | 16 weeks | G5 |
| 6 | Manufacturing + Inspection + Test Integration | 18 weeks | G6 (GA-2) |
| 7 | Portable Air-Gapped Deployment + HPC + Edge | 14 weeks | G7 |
| 8 | Advanced Scientific Intelligence + Autonomous Engineering Workflows | 16 weeks | G8 |

## Phase 0 — Complete (G0)

See [../README.md](../README.md) for the Phase 0 (G0) deliverables list.

## G0 acceptance (G0 review)

- [x] Monorepo skeleton (§19.2)
- [x] ADR-000 set covering §2 invariants and §4 technology decisions
- [x] Contract v0 JSON Schemas (graph.model, plugin.manifest, physics.solver, compute.backend, io.format)
- [x] Two-impl validation harness (Python in Phase 0; Rust in Phase 1)
- [x] Walking skeleton (a): stdio JSON-RPC substrate (x86_64 verified; aarch64 + Slurm require hardware not in this environment)
- [x] Walking skeleton (b): SQLite event-sourced graph at 10M-node scale
- [x] Walking skeleton (c): Offline TUF bundle prototype
- [x] Fitness pipeline skeleton (import-linter, token scan, LOC budget, semver checker, SBOM)
- [x] Dark-build CI runner (egress-free)
- [x] UAV wing "paper run" through every interface
- [x] All ADRs merged
- [x] Three skeletons demonstrably run on the target matrix
- [x] G0 review passes (security sign-off is a calendar gate)

## Phase 1 — Complete (G1)

Phase 1 delivered a usable, scriptable graph + CLI + RBAC + capability
registry + Rust half of the two-impl rule:

- **Graph engine** — `aether_graph`: event-sourced storage, AS-OF replay,
  AQL subset (type + property filters), 7 SDP §3.6 invariants enforced.
- **Provenance** — `aether_provenance`: per-artifact `ProvenanceRecord`,
  Merkle `ProvenanceChain`, PROV-O export, hash-stable across two
  independent implementations.
- **Policy** — `aether_policy`: 8-role catalog, action matrix, decision
  surface; CEL evaluator stub (G2 fills in).
- **CLI v1** — `aether init|project|graph|requirement|plugin|package|provenance`
  with stable JSON output, deterministic exit codes, and golden tests.
- **Registry** — `aether_registry`: directory-scan discovery, capability
  matching, semver range filter.
- **Contract validation (Rust)** — `aether_contract_validate`: the Rust
  half of the two-impl rule (ADR-0022).

**Test totals at G1**: 41 Rust tests + 29 Python tests = **70 passing**.

## See also

- `invariants.md` — the twelve non-negotiable invariants
- `adr/` — the ADR set
- `threat-model/` — threat model (v1)
- `../contracts/` — contract packages
- `../reference_plugins/` — reference implementations
- `../infra/fitness/` — fitness pipeline
