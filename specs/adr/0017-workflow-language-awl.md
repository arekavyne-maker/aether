# ADR-0017: Workflow Language — AWL (YAML Subset)

- **Status:** Accepted
- **Date:** 2026-08-30
- **Deciders:** Aether Architecture Review Board
- **Supersedes:** —
- **Superseded by:** —

## Context

Workflows are the platform's primary orchestration artifact. They must be human-writable, diffable, validatable, and reviewable. They must not be a programming language — Aether's value is that methodology changes do not require code changes.

## Decision

The Aether Workflow Language (AWL) SHALL be a **YAML subset** with a versioned schema. AWL is declarative, not Turing-complete within a single document; control flow is expressed via composition of declared steps.

**Rationale (SDP §4.2):**

- **Human-writable** — engineers write AWL directly; no compiler expertise required
- **Diffable** — git-friendly, code-reviewable
- **Validatable** — JSON Schema (ADR-0016) describes every AWL document
- **Avoids inventing a programming language** — methodology changes are schema additions, not new syntax

**Constraints:**

- AWL documents declare a `awl` version field at the top
- Steps are typed (`workflow.step` contract); custom step types are plugins
- Parameter spaces are first-class (DOE, sweeps, parameter sets)
- Checkpointing, retries, and step-level placement are declarative

**Initial AWL idioms (Phase 2+):**

- DAG composition
- Parameter sweeps
- `optimize:` loops (Phase 5)
- `compose:` multiphysics blocks (Phase 4)
- `si_hooks` for Scientific Intelligence integration
- Fidelity ladders (Phase 4)
- Closed-loop manufacturing/test (Phase 6)

## Consequences

- Positive: workflows survive methodology changes
- Positive: review process is the same as for code
- Negative: complex control flow requires composition; some users will want a programming language
- Negative: AWL schema must evolve carefully; breaking changes affect every project

## Enforcement

- AWL schema is versioned; semver discipline (ADR-0011)
- `aether workflow validate` runs schema validation in CI
- Reference AWL documents in `workflows/` are conformance fixtures

## References

- SDP §4.2
- SDP §4.3 (WorkflowEngine)
- SDP §6.1 (computational requirements in steps)
- SDP §10.1 (multiphysics composition)
- SDP §18 (phased development)
- ADR-0009 (reproducibility — env manifest pins AWL schema version)
- ADR-0016 (contracts as data)
