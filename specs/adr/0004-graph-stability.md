# ADR-0004: I-4 Graph Stability Under Implementation Change

- **Status:** Accepted
- **Date:** 2026-08-30
- **Deciders:** Aether Architecture Review Board
- **Supersedes:** —
- **Superseded by:** —

## Context

The Engineering Graph (§3) is the single source of engineering truth. If its schema changes when a solver is swapped, the "replace a tool" property is moot — every project must be migrated, every workflow re-validated, every certificate re-issued.

## Decision

Replacing a solver, CAD system, optimizer, mesh engine, or compute backend SHALL NOT require any change to:

- Engineering Graph schema
- Project structure (`.aetherpkg` layout)
- Workflow definitions (`*.awl` files)
- CLI command surface (within a major version)

The mechanism is that the graph references **capabilities and artifacts** (the result of a `SimulationRun`), not tools. A tool swap creates a new `SimulationRun` sibling on the same study; the study, its intent, its inputs, and all analytical relationships are untouched (SDP §3.8).

## Consequences

- Positive: a tool swap is a plugin install, not a project migration
- Positive: historical projects remain runnable indefinitely
- Negative: schema changes that *are* legitimate (e.g., new node types) must be carefully designed for additive evolution
- Negative: the contract surface is wider than if contracts could be more freely specialized

## Enforcement

- MVR-01/02/06/07 run on every release against two independent implementations per contract family (SDP §20)
- Schema semver checker (`infra/fitness/schema_semver.py`) rejects breaking changes within a major version
- Reference test projects ("graph-stability fixtures") are pinned and re-run on every release

## References

- SDP §2 I-4
- SDP §3.8 (why this abstraction survives tool replacement)
- SDP §5.3 (capability matching)
- SDP §20 (modularity validation)
- ADR-0001 (microkernel minimality)
