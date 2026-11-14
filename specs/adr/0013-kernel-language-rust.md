# ADR-0013: Kernel Language — Rust

- **Status:** Accepted
- **Date:** 2026-08-30
- **Deciders:** Aether Architecture Review Board
- **Supersedes:** —
- **Superseded by:** —

## Context

The kernel is the only immutable component of Aether (SDP §4.1). Its language must support a 15-year horizon, an air-gapped deployment target, and the high-throughput graph/event paths at the platform's core.

## Decision

The Aether kernel SHALL be written in **Rust** (stable toolchain).

**Rationale (SDP §4.2):**

- **Memory safety without GC** — eliminates a class of correctness issues without runtime overhead
- **Single static binary per target** — `x86_64` and `aarch64` builds are portable, simplify air-gapped distribution
- **Robust process supervision** — Rust's strong type system expresses plugin lifecycle states without state explosion
- **Predictable performance** for graph/event hot paths, where allocation pauses from a GC would be visible
- **Strong ecosystem** for serialization, cryptography, and database drivers (all permitted kernel dependencies per ADR-0002)

**Constraints:**

- Rust stable toolchain only; nightly features forbidden
- No `unsafe` outside explicitly-reviewed `safety:`-commented blocks
- Transitive dependencies must be air-gap-deployable (no build-time network fetches); vendored where practical

## Consequences

- Positive: the kernel can be reasoned about formally; `unsafe` is auditable
- Positive: deploys as a single static binary; no dynamic linker surprises on HPC
- Positive: performance is predictable
- Negative: fewer contributors than Python; ramp-up cost for new kernel engineers
- Negative: build times are longer than Go or Python

## Enforcement

- `crates/aether_kernel/Cargo.toml` declares the edition and rust-version
- `#![deny(unsafe_code)]` at crate root, with explicit `#[allow]` for justified blocks (each reviewed)
- CI matrix: stable toolchain, both target architectures

## References

- SDP §4.2
- ADR-0001 (microkernel minimality)
- ADR-0005 (air-gap by design)
- ADR-0014 (plugin SDK Python first)
