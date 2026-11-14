# ADR-0016: Contracts as Data — JSON Schema 2020-12

- **Status:** Accepted
- **Date:** 2026-08-30
- **Deciders:** Aether Architecture Review Board
- **Supersedes:** —
- **Superseded by:** —

## Context

Aether's contract surface is the integration surface. If the schema language is proprietary, every consumer reimplements validation. If it is text-only without formal validation, every consumer reimplements parsing.

## Decision

All Aether message and payload schemas SHALL be expressed in **JSON Schema draft 2020-12**. Schemas SHALL live in contract packages under `contracts/` and SHALL be embedded at build time (codegen for Rust + Python).

**Rationale (SDP §4.2):**

- **Language-neutral** — Rust, Python, and any other future SDK can validate against the same source
- **Testable** — schema validators are themselves conformance fixtures
- **Air-gap friendly** — schemas are pure data; no external service to fetch them
- **Conformance tooling** — the fitness pipeline runs every plugin's payloads through the schema
- **Versioning** — `$id` and `version` keywords in every schema, with strict semver discipline (ADR-0011)

**Constraints:**

- The JSON Schema 2020-12 meta-schema is **pinned**; newer drafts are not adopted without an ADR
- Every contract package has a top-level `manifest.json` declaring its contract id, version, capabilities, and the JSON Schemas it contains
- The contract validation harness (§18 Phase 0) has **two independent parser implementations** (Rust + Python) — this is the foundation of the two-impl rule (ADR-0022)

## Consequences

- Positive: the contract surface is the integration surface, in a widely understood format
- Positive: a single schema change propagates mechanically to Rust and Python types
- Negative: codegen must be maintained; type drift between generated code and schemas is a real failure mode
- Negative: JSON Schema's expressiveness has edges; some constraints must be expressed at the SDK layer

## Enforcement

- Schema pinning in `contracts/*/manifest.json`
- Codegen as a CI step; drift between generated code and checked-in fixtures fails the build
- Two-impl validation harness runs every contract schema through both parsers

## References

- SDP §4.2
- SDP §5.4 (contract catalog)
- SDP §16 (standards strategy)
- SDP §18 Phase 0
- ADR-0011 (contracts are versioned, tested, independently consumable)
- ADR-0022 (two-implementation rule)
