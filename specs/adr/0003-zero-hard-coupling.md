# ADR-0003: I-3 Zero Hard-Coupling

- **Status:** Accepted
- **Date:** 2026-08-30
- **Deciders:** Aether Architecture Review Board
- **Supersedes:** —
- **Superseded by:** —

## Context

Vendor lock-in is a slow, structural failure mode. A single `if solver == "OpenFOAM"` branch anywhere in the platform corrupts the entire modularity claim, because every downstream component can transitively depend on it.

## Decision

Domain modules SHALL NEVER import another domain's implementation. They communicate only through:

1. **Contracts** (JSON Schema + Rust trait + Python protocol)
2. **Engineering Graph objects** (typed nodes/edges with validated properties)

**Prohibited tokens** in the codebase: `solver ==`, `solver !=`, `cad_vendor`, `is_nvidia`, `is_intel`, `if vendor`, `when database_engine`, and similar branch-by-vendor patterns. Detection is automated.

**Prohibited imports:** a physics-solver plugin SHALL NOT import from a specific mesh engine plugin; a manufacturing-process plugin SHALL NOT import from a specific inspection-device plugin; a workflow-step type SHALL NOT import another workflow-step type's implementation.

## Consequences

- Positive: replacing any implementation has no ripple effect on its peers
- Positive: the contract surface becomes the only integration surface — discoverable, versioned, testable
- Negative: extra ceremony when "just calling" between two plugins would be convenient
- Negative: the prohibited-token list must be maintained and expanded as new domain terms appear

## Enforcement

- `import-linter`-style layer rules in CI (`infra/fitness/import_linter.py`)
- Prohibited-token scan over the entire codebase (`infra/fitness/token_scan.py`)
- PRs that introduce a prohibited token or prohibited import are blocked

## References

- SDP §2 I-3
- SDP §5.3 (capability matching, not branching)
- SDP §19.4 (dependency rules)
- SDP §19.5
- ADR-0001 (microkernel minimality)
- ADR-0002 (everything else is a plugin)
