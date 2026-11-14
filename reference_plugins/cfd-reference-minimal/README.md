# cfd-reference-minimal — Phase 0 conformance fixture

A deliberately minimal 1D-flow solver used **only** for conformance testing
(SDP §5.6, §21). It is **not** positioned as production physics.

The plugin implements a subset of `aether.physics.solver`:

- `discover()` — returns a static `PluginDescriptor`
- `capabilities()` — returns structured `CapabilitySet`
- `validate(problem)` — returns a `ValidationReport`
- (`prepare`, `execute`, `collect`, `verify`, `provenance` arrive in Phase 2)

The plugin speaks **JSON-RPC 2.0 over stdio** (ADR-0015): one JSON object per
line. It is the **first** reference implementation of the contract and
underwrites the two-implementation rule (ADR-0022) for `physics.solver`.

The second reference is `reference_plugins/fea-reference-beam/`. Together they
form the conformance pair.
