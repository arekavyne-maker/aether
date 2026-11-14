# ADR-0008: I-8 Provenance Completeness

- **Status:** Accepted
- **Date:** 2026-08-30
- **Deciders:** Aether Architecture Review Board
- **Supersedes:** —
- **Superseded by:** —

## Context

Provenance is the engineering fabric's memory. Without it, the digital thread is a slogan, not a structure. Without mandatory completeness, gaps appear precisely where they matter most: at the boundary between trusted and untrusted sources.

## Decision

Every artifact, decision, and execution in the platform SHALL be traceable to:

- **Inputs** (CAS content hashes of geometry, mesh, models, parameters, datasets)
- **Tool versions** (plugin id + version + conformance report hash)
- **Parameters** (canonical JSON)
- **Environment** (env-manifest hash)
- **Placement** (node identity class, scheduler backend)
- **Actor and authorization** (who initiated, what was permitted)
- **Verification outcomes** (the `verify()` result)
- **Signatures** (Ed25519 over the record itself; see ADR-0020)

The **provenance record is a mandatory output** of every contract method that produces artifacts. A plugin that fails to emit a provenance record SHALL fail the run (I-12).

The Engineering Graph enforces **orphan rejection**: an artifact that is not referenced by any provenance chain cannot exist. All cross-domain edges must be declared in edge schema packages (no ad-hoc edges).

The audit log is **hash-chained** (Merkle chain per site per year, with anchor checkpoints exportable and cross-verifiable; SDP §7.6). `aether provenance audit --verify-chain` validates continuity.

Provenance records are content-addressed and **chained** to the audit log. PROV-O export maps records/edges to standard RDF for external PLM/quality interop (SDP §14.2).

## Consequences

- Positive: every claim in the graph is auditable to its origin
- Positive: time-travel and reproduction become mechanical (MVR-09)
- Negative: every plugin author must implement `provenance()` honestly
- Negative: hash-chain storage is append-only; corrections are new events, not edits

## Enforcement

- Provenance record schema (`prov/record.v1`) is a required return for every contract method
- Orphan-artifact rejection enforced by graph engine
- MVR-09/11 (reproducibility from provenance)
- Hash chain verification: `aether provenance audit --verify-chain`

## References

- SDP §2 I-8
- SDP §3.6 (graph invariants)
- SDP §7.6 (immutable audit trail)
- SDP §14 (digital thread and provenance)
- SDP §20 MVR-09/11
- ADR-0007 (physics-first, fidelity-honest)
- ADR-0020 (cryptographic baseline)
