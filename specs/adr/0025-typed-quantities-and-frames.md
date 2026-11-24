# ADR-0025: Typed quantities and coordinate frames

**Status:** Proposed
**Date:** 2026-11-24
**Supersedes:** none

## Context

Aether currently carries units as strings in result and composition payloads.
A unit or frame mismatch can reach a solver and produce a plausible but wrong
result. Aether 3.0 requires machine-checkable physical dimensions, coordinate
frames, sign conventions, and transform provenance.

## Decision

Introduce `aether_quantities` as a small foundational crate:

- SI canonical storage for all quantities;
- explicit unit conversion with dimension checking;
- `FrameGraph` with named frames, 4×4 transforms, handedness, validity
  intervals, and provenance;
- API-boundary validation that rejects incompatible quantities before solver
  submission.

Domain crates and plugins consume the contract; they do not invent their own
unit strings.

## Alternatives considered

1. Use a third-party units crate. Acceptable if it is pure Rust, air-gap
   friendly, and passes a dependency ADR. Deferred until evaluated.
2. Keep strings and document conventions. Rejected: not machine-checkable.
3. Put units in the graph only. Rejected: the graph is metadata, not a
   dimensional-analysis engine.

## Consequences

- Every solver input/output schema gains a quantity representation.
- Existing string-unit payloads need migration.
- Unit/frame errors become loud contract violations.
- A new dependency or implementation must remain small and auditable.
