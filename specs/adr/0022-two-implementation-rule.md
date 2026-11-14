# ADR-0022: Two-Implementation Rule for Contract Stability

- **Status:** Accepted
- **Date:** 2026-08-30
- **Deciders:** Aether Architecture Review Board
- **Supersedes:** —
- **Superseded by:** —

## Context

A contract with one implementation is a single-vendor contract by another name. Aether's modularity claim is enforceable only when every contract has at least two independent implementations that pass the same conformance suite.

## Decision

A contract SHALL NOT be considered stable until **at least two independent implementations** pass its conformance suite. The "two-implementation badge" is awarded by the Contract Certification Board (§22.1).

**Rationale (SDP §5.8):**

- Prevents accidental single-vendor lock-in inside the platform
- Forces the contract surface to be the only integration surface (no implementation-specific leakage)
- Makes the reference plugin set (`reference_plugins/`) a **first-class deliverable**, not sample code

**Constraints:**

- "Independent" means: different teams, different code bases, different vendor relationships (where applicable)
- Conformance suite is the same suite used for certification; pass = badge eligibility
- The badge is recorded in the registry and surfaced in `aether plugin list`
- MVR-01/02/06/07/13/15 explicitly require two implementations per contract family (SDP §20)

**The reference plugin set is therefore a first-class engineering deliverable.** The platform team owns conformance suites and reference implementations for every contract.

## Consequences

- Positive: contract stability is mechanical, not political
- Positive: replaces a class of vendor-lock-in incidents
- Negative: doubles initial implementation effort per contract
- Negative: conformance suite authorship is a real engineering task

## Enforcement

- `aether plugin certify` runs the conformance suite and emits a signed certification report
- Contract Certification Board reviews and awards the two-impl badge
- MVR-01/02/06/07 require two certified implementations

## References

- SDP §2 I-11
- SDP §5.8 (contract governance rules)
- SDP §20 (MVR-01/02/06/07/13/15)
- SDP §22.1 (Contract Certification Board)
- ADR-0002 (everything else is a plugin)
- ADR-0011 (contracts are versioned, tested, independently consumable)
