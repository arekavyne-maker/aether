# Aether 3.0 — Implementation Plan

**Status:** executable roadmap, 2026-11-24.
**Target:** physics-grounded, evidence-native, adaptive aerostructures
engineering platform, built incrementally on the existing substrate.
**Normative target:** [`../architecture/AETHER-3.0.md`](../architecture/AETHER-3.0.md).
**Current inventory:** [`../architecture/AS-IS.md`](../architecture/AS-IS.md).
**Gap mapping:** [`../architecture/GAP-MATRIX.md`](../architecture/GAP-MATRIX.md).

---

## Operating rules

1. Vertical slices with acceptance tests, not empty abstractions.
2. No new database, broker, orchestration engine, model runtime, or agent
   framework without an ADR.
3. No capability claim without code, tests, evidence, and reproducible
   execution.
4. Research methods stay behind explicit contracts and are labeled
   `research`/`fixture` until promoted.
5. Every phase ends with a reviewable evidence package and a status update in
   [`../verification/STATUS.md`](../verification/STATUS.md).

---

## Phase 0 — Repository truth and cleanup

**Duration:** 2–3 weeks.

**Deliverables**

- AS-IS inventory, gap matrix, target architecture, research register,
  traceability matrix, threat model, status page.
- Link and claim cleanup; historical records separated.
- Baseline test and fitness evidence.
- Fixed optional-solver benchmark semantics.
- Reproducible Python environment.
- Clean repository hygiene.

**Exit gate**

- All documentation links resolve.
- `cargo test --workspace` passes or explicitly skips optional tools.
- Python tests run from one bootstrap command.
- No stale or contradictory status claims.
- Phase 0 artifacts reviewed before broad refactor.

---

## Phase 1 — Trusted engineering substrate

**Duration:** 4–6 weeks.

**Work packages**

| WP | Deliverable | Acceptance |
|---|---|---|
| 1.1 | `aether_quantities` | Unit mismatch and frame mismatch fail before solver submission |
| 1.2 | `aether_asm` + ASM schemas/catalog | Object create/revise/link/export/verify; immutable revisions |
| 1.3 | `aether_artifacts` | CAS put/get/verify, dedupe, tamper detection |
| 1.4 | `aether_evidence` | Gate fails closed on missing/stale evidence; authorized waiver only |
| 1.5 | Configuration snapshots | Exact revisions reconstructable; dependent invalidation |
| 1.6 | Real policy evaluation | Unknown action denied; decision recorded on event |
| 1.7 | CLI schema/status/policy/provenance | JSON output and golden tests |

**Exit gate**

- An engineering object can be created, revised, linked, exported, and
  verified.
- The exact input snapshot is reconstructable.
- Unit/frame mismatches fail deterministically.
- A release gate cannot pass on execution alone.

---

## Phase 2 — Vertical slice: conventional metallic aerostructure

**Duration:** 6–8 weeks.

**Demonstrator:** metallic stiffened wing-box subassembly.

**Deliverables**

- Product structure and requirements.
- Geometry import and validation.
- Material assignment and load/environment case.
- Mesh and structural solver adapter.
- Result, uncertainty, provenance, evidence package.
- Impact analysis on change.

**Exit gate**

- End-to-end CLI workflow.
- Reproducible within declared tolerances.
- Changed inputs invalidate dependent evidence.
- No certification claim.

---

## Phase 3 — Manufacturing and as-built thread

**Duration:** 4–6 weeks.

**Deliverables:** process plan, material batch, tooling/machine capability,
work instruction, inspection plan, nonconformance, repair disposition,
as-built configuration.

**Exit gate:** design revision traces to process and inspection records;
as-built deviations visible; impact analysis available. No real machine
control.

---

## Phase 4 — Multi-physics, multi-fidelity, uncertainty

**Duration:** 6–8 weeks.

**Deliverables:** second solver domain, workflow loops/branches, compute
federation contract, ROM contract, uncertainty taxonomy, multi-fidelity
planner.

**Exit gate:** planner explains fidelity selection; surrogate outputs bounded;
escalation reproducible.

---

## Phase 5 — Emerging structures

One vertical at a time:

1. Morphing/compliant structures.
2. Architected/metamaterial structures.
3. Multifunctional embedded sensing/actuation.
4. High-temperature atmospheric structures.
5. Process-aware hybrid/additive structures.

**Exit gate per vertical:** domain model, benchmark, manufacturing
constraints, uncertainty treatment, physical validation plan.

---

## Phase 6 — Adaptive intelligence and governance

**Duration:** 6–8 weeks.

**Deliverables:** model registry, model cards, agent tool contracts, active
learning, evolutionary design lab, generative design sandbox, promotion
workflow.

**Exit gate:** agents cannot bypass deterministic validation or permissions;
candidate lineage reproducible; no model promoted without independent
benchmark evidence.

---

## Phase 7 — Production hardening and federation

**Duration:** ongoing.

**Deliverables:** sandbox hardening, recovery drills, offline installation,
signed releases, SBOM/vulnerability process, supplier/federated exchange,
performance SLOs.

**Exit gate:** documented restore test, tested degraded modes, security
review, reproducible release package, operational runbooks.

---

## Dependency graph

```text
P0 truth/cleanup
  └─► P1 quantities/ASM/artifacts/evidence/policy/snapshots
        ├─► P2 metallic wing-box slice
        │     └─► P3 manufacturing/as-built
        ├─► P4 multi-physics/fidelity/UQ
        └─► P5 emerging-structure verticals
              └─► P6 model registry/agents
                    └─► P7 hardening/federation
```

## Versioning

The workspace is held at `3.0.0-rc.1`. Production-grade code and vertical
slices exist across Phases 0–7, but final `3.0.0` is reserved until the P7
gate passes: measured restore/SLO evidence, a real second federation site or
an explicitly labelled harness, an independent security review, and signed
reproducible release evidence. The rc version reflects implemented
engineering capability, not certification. Physical validation and
Demonstrators C/E validation plans are tracked in
[`../verification/STATUS.md`](../verification/STATUS.md).
