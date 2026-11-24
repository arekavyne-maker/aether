# ADR-0033: Agent runtime is bounded, proposal-only, and outside the kernel

**Status:** Superseded by ADR-0035
**Date:** 2026-11-24

## Context

Aether may use specialized agents for requirements, geometry, materials,
analysis planning, manufacturing, quality, verification, and orchestration.
Agents are useful advisors but unsafe authorities. Retrieved documents and
tool outputs are untrusted data.

## Decision

Introduce `aether_agent_runtime` only after policy, evidence, and model
registry exist. The runtime:

- receives a scoped task and immutable context snapshot;
- retrieves only authorized artifacts;
- produces structured proposals, never direct graph mutation;
- calls typed tools through policy;
- cites source objects and assumptions;
- reports uncertainty, conflicts, and missing information;
- requests human approval for controlled changes;
- records decisions and provenance.

Agents are denied: released-baseline mutation, self-approval, policy bypass,
arbitrary production shell, machine-parameter changes, and controlled-data
export.

## Consequences

- Agent capability is a permission-matrix problem, not a prompt problem.
- Prompt injection is treated as a data-handling threat.
- The kernel does not depend on an agent framework.
- Agent features can be removed without affecting deterministic workflows.
