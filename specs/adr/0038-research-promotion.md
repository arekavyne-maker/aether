# ADR-0038: Research-to-production promotion gate

**Status:** Accepted
**Date:** 2026-11-24

## Context

§4 defines research-inspired methods (morphogenesis, evolutionary search,
neural operators, PINNs, SINDy, reservoir computing, active learning,
generative design) and §0.5 requires them quarantined until validated. §1.2
forbids a black-box generative design engine. The user has requested
production-grade implementations of these methods, which increases the need
for a hard boundary rather than removing it.

## Decision

A `aether.research.promotion` contract and gate:

- a method runs in the research sandbox with a `research` label;
- promotion requires: grouped/external benchmark evidence, uncertainty
  characterisation, OOD detector, physical-constraint checks, manufacturing
  feasibility review and named approver;
- a promoted method remains advisory until separately approved for bounded
  automation;
- the production workflow cannot call a research method directly; it calls
  the promotion-managed alias;
- negative results and rejected candidates remain in lineage with reason
  codes.

## Consequences

- Research methods can be production-grade code while still being
  production-*unapproved*.
- No silent contamination of released baselines.
- Promotion is evidence-backed and reversible.
