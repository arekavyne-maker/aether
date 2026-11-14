# ADR-0002: I-2 Everything Else is a Plugin

- **Status:** Accepted
- **Date:** 2026-08-30
- **Deciders:** Aether Architecture Review Board
- **Supersedes:** —
- **Superseded by:** —

## Context

Aether exists to make solver, CAD, optimizer, mesh, database, AI framework, manufacturing, and scientific algorithm choices *replaceable*. If any of these ship in the kernel, the modularity claim collapses to a marketing claim.

## Decision

The following are **forbidden in the kernel** (SDP §4.1): any solver, CAD kernel, mesh algorithm, database engine (beyond the embedded store driver), AI framework, GUI, network daemon (except the optional local API socket), vendor branches, or domain logic.

The kernel SHALL NOT link against third-party domain libraries. Permitted kernel dependencies are limited to: cryptography, compression, storage drivers, RPC/serialization, and the CLI framework (SDP §2 I-2).

Each contract (§5.4) SHALL have **at least two independent implementations** pass its conformance suite before the contract is considered stable (the "two-implementation rule"; see ADR-0022).

## Consequences

- Positive: a third-party implementation can replace a reference one with zero kernel changes
- Positive: forces the contract surface to be the integration surface
- Negative: increases initial implementation effort (every contract needs ≥2 impls)
- Negative: requires a robust conformance suite infrastructure

## Enforcement

- Kernel dependency scan in CI rejects third-party domain libraries (`infra/fitness/dep_audit.py`)
- Conformance suite execution per contract (`infra/fitness/conformance_runner.py`)
- Two-impl badge awarded by Contract Certification Board (§22.1)

## References

- SDP §2 I-2
- SDP §4.1
- SDP §5 (Contract-Driven Plugin Architecture)
- SDP §5.8 (Contract Governance Rules)
- SDP §19.5
- ADR-0001 (microkernel minimality)
- ADR-0022 (two-implementation rule)
