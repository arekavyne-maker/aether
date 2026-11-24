# ADR-0037: Graph access control

**Status:** Accepted
**Date:** 2026-11-24

## Context

§6.3 requires graph queries with access control. `aether_graph` currently
records `actor` and `authz_decision` on events but does not evaluate policy;
all callers pass the literal string `"allow"`. A3-I13 requires independent
authorisation of every operation. The recorded strings are evidence of a
decision, not a decision.

## Decision

All graph reads and writes require a `Subject` and are evaluated by
`aether_policy::evaluate` before execution. Denials are appended to the event
chain with a reason code. The CLI and SDK must pass an authenticated subject;
there is no path that defaults to allow. Classification policy additionally
gates reads and exports of classified objects.

## Consequences

- Existing call sites must supply real subjects; this is a breaking API
  change handled in one migration.
- Read-only roles cannot mutate; unauthorized readers cannot see restricted
  objects.
- Every decision is auditable on the chain.
