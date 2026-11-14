# ADR-0019: Eventing — Embedded Durable Log; NATS JetStream Optional

- **Status:** Accepted
- **Date:** 2026-08-30
- **Deciders:** Aether Architecture Review Board
- **Supersedes:** —
- **Superseded by:** —

## Context

The event log is the substrate of the digital thread (SDP §14). It must be durable, queryable, and operationally simple in the single-user default.

## Decision

The default event transport SHALL be an **embedded durable log** backed by SQLite (workstation) or PostgreSQL (team). **NATS JetStream** MAY be used for multi-node deployments when justified.

**Rationale (SDP §4.2):**

- Keeps the single-binary default simple
- Scales to multi-node only when the deployment actually requires it
- The event bus is the kernel's internal pub/sub layer; the durable log is its history

**Constraints:**

- Event topics are **versioned event schemas** (per the contract catalog)
- Events are **append-only**; corrections are new events, not edits
- The event log is the substrate of `aether graph as-of` and `aether provenance reproduce` (SDP §14.3)

## Consequences

- Positive: zero operational setup in the single-user default
- Positive: scales by adding JetStream only when needed
- Negative: multi-node deployments without JetStream have weaker delivery guarantees; documented limitation

## Enforcement

- Event schemas are versioned; semver discipline (ADR-0011)
- Event log retention policy is project-configurable; defaults documented
- `aether provenance audit --verify-chain` exercises the log's continuity

## References

- SDP §3.1 (formal model — events are first)
- SDP §4.2
- SDP §7.6 (immutable audit trail)
- SDP §14.1 (event log = thread)
- ADR-0008 (provenance completeness)
- ADR-0018 (databases)
