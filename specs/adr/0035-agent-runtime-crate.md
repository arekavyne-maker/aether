# ADR-0035: Agent runtime crate and bounded tool execution

**Status:** Accepted
**Date:** 2026-11-24

## Context

`crates/aether_agents` defines the permission matrix and `crates/aether_agent_runtime`
executes proposals through registered deterministic tools. ADR-0033 specifies
the agent boundary at the architecture level but was written before the crate
existed and remains Proposed. §0.10 requires an ADR before introducing an
agent framework; §16.3 requires a permission matrix; §4.7 requires
proposal-only operation.

## Decision

The agent runtime is a **proposal evaluator and tool dispatcher**, not an
autonomous actor:

- an agent receives a scoped task and an immutable context snapshot;
- it may only call tools in its role's allow-list;
- mutating tools require a named human approver; the agent cannot self-approve;
- tools are deterministic and registered; there is no arbitrary shell access;
- every proposal and execution is hashed and appended to an audit log;
- retrieved documents are data, never instructions (prompt-injection defence).

This ADR supersedes ADR-0033, which is marked superseded below.

## Consequences

- New agent roles or tools require an update to the permission matrix and a
  conformance test proving the denied cases still fail.
- Agent output cannot become a released engineering artifact without a
  separate approval chain.
- Running an agent never grants graph or machine write authority.
