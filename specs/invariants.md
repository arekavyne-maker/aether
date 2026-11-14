# Architectural Invariants

This document is the canonical reference for the twelve non-negotiable invariants (I-1…I-12) that govern the Aether platform. Each invariant is **enforced** by a named mechanism — an invariant without enforcement is a wish.

The invariants are ratified by the Architecture Review Board via the ADR-0001…ADR-0012 set in [../adr/README.md](../adr/README.md).

## The Twelve Invariants

| ID | Invariant | ADR | Enforcement |
|---|---|---|---|
| **I-1** | Microkernel minimality (~50k LOC cap) | [ADR-0001](../adr/0001-microkernel-minimality.md) | LOC budget tracker in CI; ADR required for any kernel addition |
| **I-2** | Everything else is a plugin | [ADR-0002](../adr/0002-everything-else-is-a-plugin.md) | Kernel dependency scan; two-implementation rule; conformance suites |
| **I-3** | Zero hard-coupling | [ADR-0003](../adr/0003-zero-hard-coupling.md) | `import-linter` rules; prohibited-token scan |
| **I-4** | Graph stability under implementation change | [ADR-0004](../adr/0004-graph-stability.md) | MVR-01/02/06/07; schema semver checker |
| **I-5** | Air-gap by design | [ADR-0005](../adr/0005-air-gap-by-design.md) | Egress-free dark-build CI; HTTP client wrapper fails closed; MVR-08 |
| **I-6** | Hardware-agnostic execution | [ADR-0006](../adr/0006-hardware-agnostic-execution.md) | Resource-descriptor schema forbids hardware fields; MVR-04/05/12 |
| **I-7** | Physics-first, fidelity-honest | [ADR-0007](../adr/0007-physics-first-fidelity-honest.md) | Result schema requires `fidelity_class` + `uncertainty`; CI rejects gaps |
| **I-8** | Provenance completeness | [ADR-0008](../adr/0008-provenance-completeness.md) | Per-method provenance record mandatory; hash-chained audit log |
| **I-9** | Reproducibility | [ADR-0009](../adr/0009-reproducibility.md) | Environment Manifest lockfile semantics; MVR-09/11 |
| **I-10** | CLI canonical interface | [ADR-0010](../adr/0010-cli-canonical-interface.md) | GUI/CLI parity tests; same service layer |
| **I-11** | Contracts versioned + 2-impl rule | [ADR-0011](../adr/0011-contracts-versioned-tested.md) | Semver; conformance suite per contract; two-impl badge |
| **I-12** | Fail-visible, never fail-silent | [ADR-0012](../adr/0012-fail-visible.md) | Typed error taxonomy; contract violations fatal to the run |

## Consequence for Decomposition

The 17 layers in the SDP (§2.1) decompose as:

| Layer | Component | Status |
|---|---|---|
| L01 | CLI / Developer Experience | Built (kernel) |
| L02 | Aether Microkernel | Built (~50k LOC cap) |
| L03 | Engineering Graph | Built (kernel subsystem) |
| L04–L12 | Domain contracts + plugins | Contract packages + plugins |
| L13 | Digital Thread / Provenance | Built (kernel subsystem) |
| L14 | Compute Abstraction / Scheduling | Built (kernel SPI) + plugin backends |
| L15 | Plugin / Connector Ecosystem | Built (SDK + registry) + plugin repos |
| L16 | Security / Air-Gap / Policy | Built (kernel policy) + policy bundles |
| L17 | Portable Execution / Packaging | Built (kernel) + format plugins |

Dependency direction is strictly downward: L01 → L02 → L03; L04–L12 rest on L03 + L02; L13–L17 support all. Cross-layer peer communication occurs only through graph objects and contracts (I-3).

## How to read this

- **Architect** — every architectural decision must respect all twelve invariants. If a proposed decision violates an invariant, the response is "no" by default; an exception requires an ADR.
- **Kernel engineer** — your job is to keep the kernel small and to expose the eleven subsystems' interfaces cleanly.
- **Plugin author** — your plugin lives in a contract's conformance envelope. The two-impl rule is your friend; it means you can always be replaced.
- **Operator** — the fitness pipeline in `infra/fitness/` enforces these invariants. Green fitness = invariants hold.
- **Auditor** — every MVR in §20 is a release-blocking test of an invariant. MVR verdicts are signed and are part of the release artifact.

## References

- SDP §2 (Architectural Invariants)
- SDP §2.1 (Consequence for Decomposition)
- SDP §4 (Microkernel)
- SDP §20 (Modularity Validation)
- `specs/adr/` — the ADR set
- `infra/fitness/` — the enforcement pipeline
