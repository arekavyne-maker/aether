# ADR-0000: ADR Record Format

- **Status:** Accepted
- **Date:** 2026-08-30
- **Deciders:** Aether Architecture Review Board
- **Superseded by:** —
- **Supersedes:** —

## Context

Aether requires a durable, citable, immutable record of architectural decisions (§22.1). Architecture Decision Records (ADRs) are the chosen mechanism because they are lightweight, version-controlled, and discoverable through the same git history as the code they govern.

## Decision

ADRs in this repository use the following format:

### Required sections

1. **Header** — `Status`, `Date`, `Deciders`, `Superseded by`, `Supersedes`
2. **Context** — the forces at play, the problem being solved, and the constraints
3. **Decision** — the choice made, in the active voice ("we will …")
4. **Consequences** — the resulting trade-offs, both positive and negative
5. **Enforcement** — the concrete mechanism that makes the decision hold (§2, §19.5)
6. **References** — SDP sections, other ADRs, external standards

### Optional sections

- **Alternatives considered** — what was rejected and why
- **Notes** — late additions or clarifications

### Filename

- `NNNN-kebab-case-slug.md` (zero-padded 4-digit number)
- Index entry in [README.md](README.md)

### Status values

| Status | Meaning |
|---|---|
| `Proposed` | Under ARB review |
| `Accepted` | Approved and immutable; changes require a new ADR |
| `Superseded` | Replaced by a later ADR (link it) |
| `Deprecated` | No longer relevant; retained for history |

## Consequences

- Every architectural decision is reviewable, citable, and traceable from code to rationale
- ADR log lives in git history — no external system to maintain
- Conformance suites and fitness-pipeline rules can mechanically cross-reference ADRs

## Enforcement

- ARB review required for `Accepted` status (§22.1)
- Fitness pipeline tracks `Accepted` ADR set; semantic breakage of a referenced invariant fails CI
- Index file (`README.md`) updated by the PR that adds an ADR

## References

- SDP §2 (Architectural Invariants)
- SDP §22.1 (Decision Governance)
- Michael Nygard's ADR format
