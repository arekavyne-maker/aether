# ADR-0014: Plugin SDK — Python 3.11+ First-Class

- **Status:** Accepted
- **Date:** 2026-08-30
- **Deciders:** Aether Architecture Review Board
- **Supersedes:** —
- **Superseded by:** —

## Context

The plugin ecosystem must be accessible to the scientists and engineers who write the actual physics, optimizers, and surrogate methods — not only to the platform team. Most scientific computing in 2026 is Python.

## Decision

The Aether Plugin SDK SHALL have **Python 3.11+ as the first-class** language. Rust and C++ bindings SHALL be **second-class** (available, supported, but not where the SDK's productivity lives).

**Rationale (SDP §4.2):**

- Python is the lingua franca of scientific computing and ML
- Solver wrappers in C++/Fortran are reached through the **native binary adapter pattern** (§5.6), so SDK breadth stays manageable
- Most reference plugins are written in Python; community plugins follow

**Constraints:**

- Plugin authors target Python 3.11+; older versions are not supported
- The Python SDK is a thin layer over the kernel's JSON-RPC interface (see ADR-0015)
- The SDK has no privileged access — every Python call translates to a typed RPC + CAS operation
- All Python dependencies are pinned via the Environment Manifest (ADR-0009)

## Consequences

- Positive: lower barrier to entry for the scientific computing community
- Positive: the SDK is a thin shim, so the contract surface is the integration surface
- Negative: Python startup latency is a real cost; mitigated by process pooling (Phase 2)
- Negative: Python's GIL constrains CPU-bound parallelism within one process; mitigated by multi-process and distributed patterns

## Enforcement

- `python/aether_sdk/` is the canonical Python SDK
- SDK tests run in CI against the contract validation harness (ADR-0011)
- Reference plugins in `reference_plugins/` are Python-first

## References

- SDP §4.2
- SDP §5 (plugin architecture)
- SDP §5.6 (native binary adapter pattern)
- ADR-0002 (everything else is a plugin)
- ADR-0009 (reproducibility)
- ADR-0013 (kernel language Rust)
- ADR-0015 (IPC)
