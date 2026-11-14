# ADR Index

This directory contains **Architecture Decision Records** for the Aether platform. ADRs are the unit of architectural memory (§22.1). Each ADR is numbered, immutable once accepted, and uses `superseded-by` links when revised.

| ADR | Title | Status | Section |
|---|---|---|---|
| [ADR-0000](0000-record-format.md) | ADR record format | Accepted | — |
| [ADR-0001](0001-microkernel-minimality.md) | I-1: Microkernel minimality (~50k LOC cap) | Accepted | §2 I-1 |
| [ADR-0002](0002-everything-else-is-a-plugin.md) | I-2: Everything else is a plugin | Accepted | §2 I-2 |
| [ADR-0003](0003-zero-hard-coupling.md) | I-3: Zero hard-coupling | Accepted | §2 I-3 |
| [ADR-0004](0004-graph-stability.md) | I-4: Graph stability under implementation change | Accepted | §2 I-4 |
| [ADR-0005](0005-air-gap-by-design.md) | I-5: Air-gap by design | Accepted | §2 I-5 |
| [ADR-0006](0006-hardware-agnostic-execution.md) | I-6: Hardware-agnostic execution | Accepted | §2 I-6 |
| [ADR-0007](0007-physics-first-fidelity-honest.md) | I-7: Physics-first, fidelity-honest | Accepted | §2 I-7 |
| [ADR-0008](0008-provenance-completeness.md) | I-8: Provenance completeness | Accepted | §2 I-8 |
| [ADR-0009](0009-reproducibility.md) | I-9: Reproducibility | Accepted | §2 I-9 |
| [ADR-0010](0010-cli-canonical-interface.md) | I-10: CLI canonical interface | Accepted | §2 I-10 |
| [ADR-0011](0011-contracts-versioned-tested.md) | I-11: Contracts are versioned, tested, and independently consumable | Accepted | §2 I-11 |
| [ADR-0012](0012-fail-visible.md) | I-12: Fail-visible, never fail-silent | Accepted | §2 I-12 |
| [ADR-0013](0013-kernel-language-rust.md) | Kernel language: Rust | Accepted | §4.2 |
| [ADR-0014](0014-plugin-sdk-python-first.md) | Plugin SDK: Python 3.11+ first-class | Accepted | §4.2 |
| [ADR-0015](0015-ipc-jsonrpc-stdio.md) | IPC: JSON-RPC 2.0 over stdio | Accepted | §4.2 |
| [ADR-0016](0016-contracts-as-data.md) | Contracts as data: JSON Schema 2020-12 | Accepted | §4.2 |
| [ADR-0017](0017-workflow-language-awl.md) | Workflow language: AWL (YAML subset) | Accepted | §4.2 |
| [ADR-0018](0018-databases-sqlite-postgres.md) | Databases: SQLite embedded default, PostgreSQL for teams | Accepted | §4.2 |
| [ADR-0019](0019-eventing-embedded-jetstream-optional.md) | Eventing: embedded durable log; NATS JetStream optional | Accepted | §4.2 |
| [ADR-0020](0020-cryptographic-baseline.md) | Cryptographic baseline: Ed25519, SHA-256/3, X.509, TLS 1.3 | Accepted | §7.1 |
| [ADR-0021](0021-tuf-adapted-offline.md) | TUF-adapted-for-offline artifact distribution | Accepted | §7.2 |
| [ADR-0022](0022-two-implementation-rule.md) | Two-implementation rule for contract stability | Accepted | §5.8 |
| [ADR-0023](0023-air-gap-egress-free-ci.md) | Egress-free "dark build" CI runner | Accepted | §19.5 |
| [ADR-0024](0024-siu-autonomy-classes.md) | SI autonomy classes A0–A4 | Accepted | §8.5 |

## Conventions

- **Format:** see [ADR-0000](0000-record-format.md)
- **Status values:** `Proposed` · `Accepted` · `Superseded` · `Deprecated`
- **Immutability:** once `Accepted`, the file is not edited; create a new ADR that supersedes it
- **Linking:** ADRs cross-reference invariants (§2), sections in [SDP.md](../../../SDP.md), and other ADRs

## Writing a new ADR

1. Copy the template from `0000-record-format.md`
2. Use the next free number with a zero-padded prefix
3. Submit via PR; ARB review (§22.1) approves or rejects
4. On approval, change status to `Accepted` and add to this index
