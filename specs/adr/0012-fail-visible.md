# ADR-0012: I-12 Fail-Visible, Never Fail-Silent

- **Status:** Accepted
- **Date:** 2026-08-30
- **Deciders:** Aether Architecture Review Board
- **Supersedes:** —
- **Superseded by:** —

## Context

A solver that returns success but emits a malformed result record is worse than one that crashes. A policy denial that returns a warning is worse than one that blocks. "Plausible-looking output" is the failure mode of all serious engineering platforms.

## Decision

The platform SHALL halt with explicit diagnostics on:

- **Contract violation** — a plugin that does not honor its contract (`ContractViolation`, fatal)
- **Policy denial** — an action forbidden by the active policy (`PolicyDenial`, fatal)
- **Validation failure** — an input that fails schema validation (`ValidationFailure`, fatal to step, recorded as evidence)
- **Unverifiable provenance** — a run whose provenance chain cannot be verified (`ContractViolation`, fatal)
- **Plugin contract breach** — a plugin that returns success but emits a malformed artifact (`ContractViolation`, fatal)

The kernel **never masks a contract breach**. A `verify()` that fails, a `fidelity_class` that is unsupported, a missing `uncertainty` field, a missing `provenance()` — all are fatal to the run. The failure is itself a **provenance-recorded event** with full context (who, what, when, inputs, plugin version, environment, signatures).

**Typed error taxonomy** (SDP §4.5, normative):

| Variant | Class | Retry? |
|---|---|---|
| `ContractViolation` | fatal | no |
| `PolicyDenial` | fatal | no |
| `ResourceUnavailable` | retryable per policy | yes |
| `DataMismatch` | fatal | no |
| `PluginCrash` | per-workflow retry/abort | configurable |
| `ValidationFailure` | fatal to step | no |

CLI exit codes mirror the taxonomy (§15.1): `0` ok · `10` validation · `20` policy denial · `30` contract violation · `40` resource · `50` internal.

## Consequences

- Positive: a wrong answer is never produced silently
- Positive: failure context is always available for forensic review
- Negative: some workflows that "would have been fine" halt; users must explicitly handle the failure
- Negative: every plugin must handle the error taxonomy correctly

## Enforcement

- Kernel treats contract violations as fatal to the run, recorded as evidence
- Fitness pipeline includes a "fail-loud" regression suite: tests that exercise failure paths
- `aether validate` re-runs the failure-mode checks against historical runs

## References

- SDP §2 I-12
- SDP §4.5 (failure semantics)
- SDP §15.1 (CLI exit codes)
- ADR-0007 (physics-first, fidelity-honest)
- ADR-0008 (provenance completeness)
