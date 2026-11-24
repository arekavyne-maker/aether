# The Engineering Graph

The Engineering Graph is the platform's central abstraction (SDP §3).
It is a typed, directed, attributed multigraph with immutable
revisions and an append-only event log. Files, applications,
solvers, and hardware are implementations *around* the graph, not
the other way around.

## Properties

- **Typed**: every node and edge has a type declared in a registered
  schema package.
- **Directed**: edges have a source and a target.
- **Attributed**: nodes and edges carry typed properties validated
  against the type's JSON Schema.
- **Multigraph**: multiple edges between the same pair are allowed.
- **Immutable revisions**: revisions are append-only; corrections
  create new revisions with `supersedes` edges.
- **Append-only event log**: every mutation is an event; the current
  state is a materialized view of the log.

## Identity

- **Logical identity**: every node carries a permanent
  `aether://<project>/<uuid>` URN.
- **Content identity**: every artifact is content-addressed
  (SHA-256).
- **Revisions**: a revision is immutable; `current_revision` on the
  node points to the head of the revision chain.

## Replays and AS-OF queries

The graph's event log is hash-chained (SDP §7.6). AS-OF queries
return the graph at any past event offset. The kernel's `replay_to_offset`
materializes state from the log without mutation; the production
`graph query as-of <offset>` CLI uses this.

## Catalogs (the schema package surface)

- **Node-type catalog**: 20+ node types (Requirement, System, Part,
  GeometryRevision, Mesh, SimulationStudy, SimulationRun, Result,
  Evidence, Decision, Material, ...). Defined in
  `contracts/graph.model/catalog/nodes/*.json`.
- **Edge-type catalog**: 20+ edge types (satisfies, decomposes,
  has_geometry, simulated_by, produced_by, optimizes_by, verified_by,
  updates, traces_to, ...). Defined in
  `contracts/graph.model/catalog/edges/*.json`.

Each edge-type entry declares its source / target node types and
its cardinality constraints (max_out_per_source, etc.).

## Schemas (the 24 contracts)

The graph itself is `aether.graph.model` (G0). Every other contract
(physics.solver, geometry.kernel, mesh.engine, ...) operates on
graph objects via the engine's typed commands. The contracts declare
their own schemas; the graph is the spine that holds them together.

## Conformance

- `cargo test -p aether_graph --test engine_conformance` — 11 tests
  covering typed nodes, edges, cardinality, immutability, AS-OF
  replay, and the verification-coverage invariant.
- The SDP §3.6 invariant 7 (verification coverage) is enforced:
  a `System` cannot transition to `Released` unless every
  `satisfies`-edge target `Requirement` has a `verifies`-edge path.

## What the graph is not

- It is not a database. The graph's storage SPI is the interface;
  the SQLite / PostgreSQL / in-memory implementations are swappable.
- It is not a version-control system. Plugins persist their own
  revision chains via the graph's `supersedes` edge.
- It is not a workflow engine. Workflows are AWL documents; the
  executor runs them and writes the result into the graph.
