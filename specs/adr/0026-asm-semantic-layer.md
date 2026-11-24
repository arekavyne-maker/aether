# ADR-0026: Aerospace Systems Model as a versioned semantic layer

**Status:** Proposed
**Date:** 2026-11-24

## Context

The graph catalog is generic and data-driven, but Aether 3.0 needs a canonical
aerospace product model: programs, aircraft configurations, products,
interfaces, variants, baselines, change requests, and effectivity. Encoding
these as ad-hoc properties would make cross-context queries unreliable.

## Decision

Define the Aerospace Systems Model (ASM) as versioned JSON Schemas plus graph
catalog node/edge types, with a thin `aether_asm` crate for IDs, revisions,
lifecycle transitions, interface validation, and configuration snapshots.

ASM is a semantic layer, not a CAD kernel or PLM database. Domain services own
their internal representations and map to ASM at contract boundaries.

## Consequences

- New graph types are added incrementally by vertical slice, not all at once.
- Schema evolution is explicit and versioned.
- The kernel remains small; ASM validation lives in a boundary crate.
- CAD/PLM adapters must declare semantic loss.
