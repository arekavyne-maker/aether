# aether-sdk

First-class Python SDK for Aether plugin authors (ADR-0014). This is the Phase 0 stub: a thin shim over the contract validation harness. Phase 2 expands it into the full SDK.

## Phase 0 surface

- `aether_sdk.validation` — JSON Schema 2020-12 validation for every Aether contract
- `aether_sdk.contracts` — discoverable path to the contract package roots in this monorepo

## Phase 2+ surface (planned)

- `aether_sdk.plugin` — `@plugin(manifest=...)` decorator and base classes
- `aether_sdk.solvers.PhysicsSolver` — base class implementing the `aether.physics.solver` contract
- `aether_sdk.geometry.GeometryKernel` — base class for `aether.geometry.kernel`
- `aether_sdk.contracts.contract_path(id)` — contract path resolution
- `aether_sdk.cas` — content-addressed store client
- `aether_sdk.provenance` — provenance record construction
- `aether_sdk.events` — event bus client

## Building

```bash
cd python/aether_sdk
pip install -e ".[dev]"
pytest
```

The full kernel integration is in `crates/aether_kernel/`; the SDK is the typed Python surface over its stdio JSON-RPC substrate (ADR-0015).
