# ADR-0028: Evidence ledger and release gates

**Status:** Proposed
**Date:** 2026-11-24

## Context

Aether records results and provenance, but successful execution does not
establish engineering authority. Aether 3.0 requires explicit evidence
classes, credibility records, gate evaluation, waivers, and human approvals.

## Decision

Introduce `aether_evidence` and an `aether.evidence` contract:

- evidence classes (analytical, verification, validation, coupon, element,
  component, full-scale, process qualification, inspection, operational,
  independent review);
- credibility records for models and analyses;
- deterministic gate definitions and evaluation over graph revisions and
  artifact hashes;
- waiver and approval records bound to principal, role, and evidence set;
- release packages that fail closed on missing, stale, or contradictory
  evidence.

Evidence is evaluated independently of any solver or agent.

## Consequences

- Release is a policy/evidence operation, not a CLI assertion.
- Missing evidence blocks release.
- Historical evidence remains traceable even when invalidated.
- The gate engine must be deterministic and testable.
