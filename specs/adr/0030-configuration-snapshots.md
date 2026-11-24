# ADR-0030: Configuration snapshots bind runs to exact revisions

**Status:** Proposed
**Date:** 2026-11-24

## Context

Input hashes exist, but they do not identify the exact product, geometry,
material, process, model, solver, and environment revisions used by a run.

## Decision

Add `ConfigurationSnapshot` as an immutable graph object and content-addressed
manifest. A snapshot lists every in-scope object revision and artifact hash.
Simulation and manufacturing runs bind to a snapshot ID.

## Consequences

- Historical runs are reconstructable.
- Changing any input invalidates dependent evidence.
- Snapshots can be diffed for impact analysis.
- Snapshot creation is explicit; there is no mutable "current baseline" hidden
  state.
