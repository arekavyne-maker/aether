# ADR-0031: Standards are adapters, not the internal schema

**Status:** Proposed
**Date:** 2026-11-24

## Context

Aether 3.0 must exchange data with STEP AP242, QIF, OPC UA, MTConnect, FMI,
HDF5/CGNS/VTK, and related standards. Standards are necessary interoperability
boundaries but poor candidates for a universal internal model.

## Decision

Keep ASM and contracts canonical. Implement standards as versioned adapters
behind `aether.io.format` / `aether.standards.exchange`, with:

- import/export fixtures;
- round-trip tests;
- explicit semantic-loss reports;
- version compatibility matrices;
- no conformance claim merely because an adapter exists.

## Consequences

- Proprietary formats are isolated behind contracts.
- Interoperability quality is measurable.
- Adapter gaps remain visible rather than contaminating the core model.
