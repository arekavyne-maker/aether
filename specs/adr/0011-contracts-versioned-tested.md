# ADR-0011: I-11 Contracts Are Versioned, Tested, and Independently Consumable

- **Status:** Accepted
- **Date:** 2026-08-30
- **Deciders:** Aether Architecture Review Board
- **Supersedes:** —
- **Superseded by:** —

## Context

A contract without a conformance suite is wishful. A contract without two independent implementations is locked-in by another name.

## Decision

Every contract SHALL have:

- A **SemVer** version, `MAJOR.MINOR.PATCH`
- A **published JSON Schema** for every message and payload
- A **published vocabulary** for capability declarations (closed enumerations, extensible via registered namespaces)
- A **lifecycle semantics** document (state machine, error codes, retryability)
- A **conformance suite** as an executable artifact, runnable offline
- **Two independent implementations** that pass the conformance suite before the contract is considered stable (the "two-implementation rule"; see ADR-0022)

**Semver discipline:**

- Breaking changes bump `MAJOR` and require a migration guide
- A **dual-support period** of 6 months applies to all `MAJOR` changes after Phase 3 GA-1
- Workflows pin contract versions; the registry resolves the pinned set
- The platform refuses to install plugins whose required contract range cannot be satisfied by the installed contracts

**Contract freeze windows:**

- Phases 1–2: contracts may break freely with notice
- After Phase 3 GA-1: `MAJOR` changes require an ADR + 6-month dual support

**Capability vocabularies are namespaces.** Closed within a namespace; extensible by registering a new namespace (e.g., `physics.morphing.aeroelastic`). This prevents vocabulary monopolies.

## Consequences

- Positive: a contract is never released without proof it works
- Positive: a contract is never stable with only one implementation
- Negative: dual-support policy means the platform carries more surface for 6 months after a breaking change
- Negative: conformance suite authorship is a first-class engineering task, not an afterthought

## Enforcement

- Contract packages versioned independently in `contracts/`
- No contract released without its conformance suite
- Two-impl badge awarded by Contract Certification Board (§22.1)
- Schema semver checker in CI

## References

- SDP §2 I-11
- SDP §5.4 (contract catalog)
- SDP §5.8 (contract governance rules)
- SDP §19.5
- ADR-0002 (everything else is a plugin)
- ADR-0022 (two-implementation rule)
