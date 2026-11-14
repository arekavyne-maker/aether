# ADR-0015: IPC — JSON-RPC 2.0 over stdio

- **Status:** Accepted
- **Date:** 2026-08-30
- **Deciders:** Aether Architecture Review Board
- **Supersedes:** —
- **Superseded by:** —

## Context

The kernel and plugins live in separate processes for isolation, supervision, and air-gapped distribution (SDP §4.4, §5.7). The IPC substrate is the platform's most-tested contract; getting it wrong is fatal.

## Decision

The Aether plugin control plane SHALL use **JSON-RPC 2.0 over stdio** (LSP-proven pattern) as the default transport. **Local Unix socket / TCP-localhost gRPC** MAY be used for high-throughput services where justified.

**Rationale (SDP §4.2):**

- stdio works everywhere — HPC login/compute nodes with no daemon privileges, factory PCs, air-gapped workstations
- No port allocation; trivially sandboxable
- Proven by the Language Server Protocol at massive scale
- Bulk data **never** travels through RPC: it goes through the CAS + field-file conventions (§10.3). Control via RPC; bulk via CAS.

**Constraints:**

- Plugin entry points expose `--stdio` mode for kernel-driven invocation
- Messages are JSON, one per line (newline-delimited JSON-RPC)
- Requests have timeouts; heartbeats are exposed in the manifest (`§5.2`)
- All RPC calls are authenticated via the `RunToken` (scoped, signed, short-lived)
- The kernel's IPC layer is the only place that parses RPC; plugins see typed SDK methods

**Walking skeleton (a)** (§18 Phase 0) validates this substrate on x86_64 + aarch64 + a Slurm-allocated node before any further investment.

## Consequences

- Positive: substrate works in every environment the platform targets
- Positive: no port conflicts, no firewall negotiation
- Positive: language-agnostic; any language with stdio + JSON can be a plugin host
- Negative: bulk data must use CAS, not RPC — extra discipline
- Negative: Python plugin startup latency per call; mitigated by process pooling

## Enforcement

- Walking skeleton (a) validates stdio + JSON-RPC across the architecture matrix
- IPC layer is the only parser of RPC; the fitness pipeline enforces this
- No plugin process may open a network socket without explicit manifest declaration and policy approval

## References

- SDP §4.2
- SDP §4.4 (runtime model)
- SDP §5.7 (isolation and supervision)
- SDP §10.3 (field data model)
- SDP §18 Phase 0 walking skeleton (a)
- ADR-0001 (microkernel minimality)
- ADR-0002 (everything else is a plugin)
- ADR-0005 (air-gap by design)
