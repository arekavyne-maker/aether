# ADR-0009: I-9 Reproducibility

- **Status:** Accepted
- **Date:** 2026-08-30
- **Deciders:** Aether Architecture Review Board
- **Supersedes:** —
- **Superseded by:** —

## Context

"Reproducible" in scientific computing often means "I think I can re-run it." Aether's claim is stronger: bit-identical inputs, documented-equivalent outputs, and a verifiable path to re-execution.

## Decision

An engineering package (`.aetherpkg`) pins:

- **Exact versions** of every plugin (id + semver + content hash)
- **Exact versions** of the kernel
- **Exact environment** (Python toolchain, solver builds, container image digests) via the Environment Manifest (`envman`, SDP §17.4)
- **Exact content** of every input (CAS hashes)
- **Authorization** and policy bundle version
- **Determinism class** of every executed step (see below)

**Determinism classes** (SDP §17.6, normative):

| Class | Meaning |
|---|---|
| `bit-reproducible` | Pinned toolchain + deterministic algorithm — strict mode can bit-compare |
| `statistically-reproducible` | Declared seeds + tolerance bands — equivalence mode |
| `documented-equivalent` | Nondeterministic backend; equivalence via declared tolerances on declared quantities |

Consumers (SI governors, verification logic) declare the **minimum determinism class** they accept. The policy engine can forbid verification-relevant claims from runs below `bit-reproducible` unless compensating evidence exists (e.g., replicated runs with agreement analysis).

Historical re-execution is a **supported mode**, not a best effort. The CLI command `aether provenance reproduce <run-id>` resolves the run's immutable input snapshot, pinned plugin versions, and env manifest; verifies CAS integrity; and re-executes in strict or equivalence mode, producing a reproduction report attached as evidence to the original run (MVR-09).

## Consequences

- Positive: historical runs are re-executable indefinitely, regardless of plugin churn
- Positive: verification claims carry their reproducibility basis with them
- Negative: some workflows must explicitly opt into a determinism class; GPU nondeterminism is acknowledged rather than hidden
- Negative: env-manifest pinning requires care during plugin upgrades

## Enforcement

- Environment Manifest (`envman`) lockfile semantics
- MVR-09/11 executed per release
- Strict mode refuses execution on unresolvable pin
- Equivalence mode records the delta and attaches to the original run

## References

- SDP §2 I-9
- SDP §17.4 (Environment Manifest)
- SDP §17.6 (Determinism Policy)
- SDP §20 MVR-09/11/12
- ADR-0007 (physics-first, fidelity-honest)
- ADR-0008 (provenance completeness)
