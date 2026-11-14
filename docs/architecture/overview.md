# Architecture overview

Aether is an air-gapped, hardware-agnostic, fully modular engineering
fabric for emerging aerostructures. It is a microkernel platform on
which 19 contracts (graph, physics, geometry, mesh, optimizer,
surrogate, twin, manufacturing, inspection, test, UQ, SI, policy, ...)
are implemented as independently-installable plugins.

## Twelve invariants (the constitution)

The platform is governed by twelve non-negotiable invariants (SDP §2).
Each is enforced by a named mechanism; an invariant without
enforcement is a wish.

| # | Invariant | Enforcement |
|---|---|---|
| I-1 | Microkernel minimality (~50k LOC cap) | LOC budget tracker in CI |
| I-2 | Everything else is a plugin | Two-implementation rule; dependency scan |
| I-3 | Zero hard-coupling | `import-linter` rules; prohibited-token scan |
| I-4 | Graph stability under implementation change | MVR-04/05/12 conformance |
| I-5 | Air-gap by design | Dark-build CI; AETHER_OFFLINE=1; egress-fail-closed |
| I-6 | Hardware-agnostic execution | Resource-descriptor schema forbids hardware names |
| I-7 | Physics-first, fidelity-honest | Result schema requires `fidelity_class` + `uncertainty` |
| I-8 | Provenance completeness | Hash-chained audit log; per-method record emission |
| I-9 | Reproducibility | envman lockfile + StrictVerifier |
| I-10 | CLI-canonical interface | GUI/CLI parity tests |
| I-11 | Contracts versioned + 2-impl | Conformance test; SemVer discipline |
| I-12 | Fail-visible, never fail-silent | Typed error taxonomy; contract violations fatal |

The full invariant table is in `architecture/invariants.md`.

## The Engineering Graph

The Engineering Graph is the central abstraction: a typed, directed,
attributed multigraph with immutable revisions and an append-only
event log. Files, applications, solvers, and hardware are
implementations *around* the graph, not the other way around.

Every node has a UUIDv7 identity; every artifact is content-addressed
(SHA-256). Replays are deterministic; AS-OF queries return the graph
at any past event offset. The 19 contracts include the
`aether.graph.model` schema (G0) which the kernel enforces on every
graph mutation.

See `architecture/graph.md` for the full catalog.

## The microkernel

The kernel is the only immutable component. It contains eleven
subsystems (`architecture/invariants.md`):

- Identity, ArtifactStore (CAS), GraphEngine, WorkflowEngine,
  EventBus, CapabilityRegistry, ProvenanceService, PolicyService,
  PluginLifecycle, ConfigService, ExecService.

The kernel stays under ~50k lines of Rust; everything else is a
plugin. Plugins speak newline-delimited JSON-RPC 2.0 over stdio
(ADR-0015) and never receive raw DB credentials or graph write
access — they only get scoped, signed `RunToken`s.

## The contract surface (19 contracts)

Every capability in Aether is a contract. Each contract declares
its methods, its example documents, and its conformance fixtures.
The two-implementation rule (ADR-0022) ensures no vendor lock-in:
no contract is stable until two independent plugins pass the
conformance test.

See `contracts/index.md` for the full catalog and `architecture/awl.md`
for the workflow language that orchestrates them.

## Air-gap and reproducibility

Aether's default deployment mode is air-gapped. There is no
mandatory cloud, telemetry, or external SaaS dependency. The
`AETHER_OFFLINE=1` env var forces every network-aware call site to
fail closed. The envman lockfile pins every plugin, container,
and policy bundle to a content hash; strict mode refuses to start
a workflow if any pin is unresolved.

See `operator/airgap.md` and `operator/envman.md`.

## The closed loop

The platform's core idiom is the closed loop: design → manufacture
→ inspect → test → update-model. Every step in the loop is a
contract; the graph is the spine. The closed loop runs end-to-end
in the GA-2 closed-loop AWL conformance test
(`workflows/wing-rib-closed-loop-ga2.awl.yaml`).

## What Aether is not

Aether is not a CAD system, a solver, a PLM suite, or a SaaS product.
It is the connective, governing, and reproducibility layer that
makes those tools interchangeable. See `architecture/invariants.md`
for the full list of what the platform is and is not.
