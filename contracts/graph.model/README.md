# `aether.graph.model` Contract v0

The Engineering Graph schema (SDP §3). The contract describes node types, edge types, identity scheme, revisions, and event semantics. The kernel owns the generic mechanics (validation, revisioning, indexing, AQL); the catalog itself is data, so new domains can add types without kernel changes (I-2).

## Versioning

- **Contract id:** `aether.graph.model`
- **Schema version:** `0.1.0` (Phase 0 v0; v1.0 freeze at Phase 1 G1)

## Components

- **Node-type catalog** (SDP §3.2) — registered via schema packages
- **Edge-type catalog** (SDP §3.3) — declared in edge schema packages with allowed source/target types and cardinalities
- **Identity** (SDP §3.4) — UUIDv7 URN, content-addressed artifacts (SHA-256)
- **Revisions** (SDP §3.1) — immutable; corrections create new revisions with `supersedes` edges
- **Events** (SDP §3.1) — append-only event log; graph state is a materialized view
- **Invariants** (SDP §3.6) — engine-enforced

## Layout

```
graph.model/
├── manifest.json
├── schema/
│   ├── node-type.schema.json
│   ├── edge-type.schema.json
│   ├── revision.schema.json
│   └── event-envelope.schema.json
├── catalog/
│   ├── nodes/
│   │   ├── requirement.node.json
│   │   ├── system.node.json
│   │   ├── part.node.json
│   │   ├── geometry-revision.node.json
│   │   ├── simulation-run.node.json
│   │   ├── result.node.json
│   │   ├── evidence.node.json
│   │   ├── decision.node.json
│   │   └── ... (initial catalog per SDP §3.2)
│   └── edges/
│       ├── satisfies.edge.json
│       ├── verifies.edge.json
│       ├── decomposes.edge.json
│       ├── has_geometry.edge.json
│       ├── produced_by.edge.json
│       ├── supersedes.edge.json
│       └── ... (initial catalog per SDP §3.3)
└── README.md
```

## See also

- SDP §3 (Engineering Graph)
- SDP §3.2 (node-type catalog)
- SDP §3.3 (edge-type catalog)
- SDP §3.4 (identity, addressing, versioning)
- SDP §3.6 (graph invariants)
- SDP §14 (digital thread = provenance)
- ADR-0001 (microkernel minimality — kernel knows only generic mechanics)
- ADR-0004 (graph stability under implementation change)
- ADR-0008 (provenance completeness)
