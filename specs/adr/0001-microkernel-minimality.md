# ADR-0001: I-1 Microkernel Minimality

- **Status:** Accepted
- **Date:** 2026-08-30
- **Deciders:** Aether Architecture Review Board
- **Supersedes:** —
- **Superseded by:** —

## Context

Without a budget or charter, microkernel platforms grow by accretion. Aether is a 15-year horizon engineering fabric; the kernel's discipline must be mechanical, not aspirational.

## Decision

The Aether Core (kernel) SHALL contain **only** the eleven subsystems enumerated in SDP §4.1: `Identity`, `ArtifactStore`, `GraphEngine`, `WorkflowEngine`, `EventBus`, `CapabilityRegistry`, `ProvenanceService`, `PolicyService`, `PluginLifecycle`, `ConfigService`, `ExecService`.

The kernel SHALL stay within a hard line-of-code budget of **~50,000 lines** of Rust (excluding generated code, vendored dependencies, and conformance test fixtures).

Any proposed addition to the kernel requires an **ADR proving** the capability cannot exist as a plugin (I-2).

## Consequences

- Positive: prevents scope creep, preserves the "replaceable" property of every other layer
- Positive: makes the kernel auditable as a single unit (Conway's law used deliberately)
- Negative: forces awkward decomposition when something legitimately wants to be a service
- Negative: the budget is a ceiling, not a target — the team may underinvest in kernel hardening

## Enforcement

- CI tracks kernel LOC per release (`infra/fitness/loc_budget.py`)
- PRs that exceed budget are blocked; ARB must approve an ADR exception
- The "kernel" scope is defined by `crates/aether_*` excluding the `aether_cli` adapter and `aether_sdk_ffi` bindings

## References

- SDP §2 I-1
- SDP §4.1
- SDP §4.2
- SDP §19.5 (CI/CD and Fitness Pipeline)
- SDP §22.1 (Decision Governance)
- ADR-0000 (record format)
