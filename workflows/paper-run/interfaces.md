# Paper Run: UAV Wing Rib — Interfaces and Gaps

The composite UAV wing rib (WING-RIB-014) narrative (SDP §3.7) touches
**every major contract** in the platform. Walking this paper run through
the actual contract schemas is the Phase 0 way to surface interface gaps
before any production code depends on them.

## Interfaces touched (per the SDP)

| Interface | Contract | Phase 0 surface | Phase 1+ executable? |
|---|---|---|---|
| Requirements → design | `requirement` (in `graph.model` Requirement) | YAML, schema-validated | yes (G1) |
| System breakdown | `graph.model` System, Assembly, Part, Component | YAML | yes (G1) |
| Geometry | `geometry.kernel`, `geometry.recipe` | placeholder schemas; full suite in G3 | partial (G1+G3) |
| Materials | `materials.model` | data in graph | yes (G3) |
| Load cases | graph model `LoadCase` | data in graph | yes (G3) |
| SimulationStudy / Run / Result | `physics.solver` | JSON-RPC stub in reference plugin | yes (G2) |
| OptimizationStudy | `optimizer` | placeholder | yes (G5) |
| ProcessPlan / ProcessStep | `manufacturing.process` | placeholder | yes (G6) |
| PhysicalPart / genealogy | graph model | data in graph | yes (G6) |
| InspectionRecord | `inspection.device` | placeholder | yes (G6) |
| TestRun / MeasurementDataset | `test.rig` | placeholder | yes (G6) |
| Model update | `twin.estimator` + Decision | data in graph | yes (G5) |
| Evidence dossier | `provenance` | `aether provenance chain` command stub | yes (G6) |

## Gaps surfaced by the paper run

The following items are **in scope for Phase 0 acceptance** or **out of
scope for Phase 0**, with the next phase that addresses each:

1. **AQL query language grammar** (SDP §3.5) — Phase 0 has the example
   query in `dossier.md` but the parser lands in G1. *Resolved in G1.*
2. **AQL graph mutation commands** (typed commands only; no raw SQL) —
   same. *Resolved in G1.*
3. **Event log schema merge semantics** — the paper run doesn't exercise
   branching/merging; that lands in G1 alongside the merge tooling.
4. **Composite node types** (Assembly/Component/Part) — Phase 0 uses a
   single `Part` node-type with a `kind` enum; G3 may split this into
   three types if the recipes diverge enough. *Tracking.*
5. **AP242 PMI round-trip** — the paper run references PMI in the
   geometry revision but the actual PMI serialization is G3. *Resolved in G3.*
6. **OptimizationStudy node type** — placeholder in Phase 0; full schema
   in G5. *Resolved in G5.*
7. **ProcessPlan / ProcessStep node types** — same, G6. *Resolved in G6.*
8. **PhysicalPart / genealogy node type** — G6. *Resolved in G6.*
9. **InspectionRecord / TestRun / MeasurementDataset node types** — G6. *Resolved in G6.*
10. **TUF bundle for the .aetherpkg format** (SDP §17) — full bundle
    format lands in G2; the Phase 0 bundle verifies the security
    substrate. *Resolved in G2.*
11. **Hash-chained audit log** — the chain primitive is in
    `aether_graph::event::Event::compute_hash`; the durable store
    integration lands in G1. *Resolved in G1.*
12. **Dossier rendering** (`aether provenance chain --format dossier`) —
    the data is in the graph; the renderer lands in G6.

## Items that the paper run explicitly tests (Phase 0)

- **Decision node has `autonomy_class`** — verified.
- **SimulationRun has immutable `input_snapshot_hash`** — verified.
- **Result has `fidelity_class`, `validation_status`, `uncertainty`** — I-7 verified.
- **Resource requirement forbids `gpu_model` and `queue_name`** — I-6 verified.
- **Plugin manifest requires `signature`** — I-8 verified.
- **TUF bundle signs root with Ed25519 and verifies** — I-12 verified.

## See also

- `chain.yaml` — the full node+edge document
- `awl/wing-screening.awl.yaml` — the workflow
- `dossier.md` — the narrative
- `gaps.md` — the full list of gaps with phase mapping
