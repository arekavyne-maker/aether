# Threat Model (v1)

> **Status:** Phase 0 (G0) — preliminary. Refined at G2, G6, G7 security reviews per SDP §18.

This document is the abridged threat model referenced by SDP §7.5. The full threat model is a Phase 2 deliverable; this v1 captures the threats the Phase 0 walking skeletons and fitness pipeline are designed to address.

## Threat classes

| Class | Examples | Phase 0 mitigation |
|---|---|---|
| Supply-chain compromise | Compromised crate, compromised Python wheel, compromised image | Vendored dependencies; dark-build CI (ADR-0023) |
| Signature forgery | Forged plugin signature, forged provenance record | Ed25519 (ADR-0020); TUF chain verification (ADR-0021); hash-chained audit log |
| Insider exfiltration | Audit log tampering, exfil via bundle export | Merkle hash chain; export-control policy; two-person rule (Phase 2) |
| Reproducibility attack | Silently altered inputs | CAS content addressing; periodic integrity sweep (Phase 1) |
| Dependency vulnerability | Vulnerable crate in vendored stack | SBOM (SPDX) per release; vulnerability watch bundle (Phase 2) |
| License / key theft | License tokens on shared HPC | License broker; node attestation (Phase 5) |

## Phase 0 walkthrough

| Walking skeleton | Threats addressed |
|---|---|
| (a) stdio JSON-RPC | Subprocess injection, malformed RPC, untrusted plugin entry points |
| (b) SQLite 10M-node event-sourced graph | Write amplification, replay integrity, hash chain continuity |
| (c) Offline TUF bundle | Forged metadata, replay attacks, role compromise |

The full threat model (`specs/threat-model/full.md`) is authored at the G2 security review per the SDP.

## References

- SDP §7.5 (threat model, abridged)
- SDP §7.6 (immutable audit trail)
- ADR-0005 (air-gap by design)
- ADR-0008 (provenance completeness)
- ADR-0020 (cryptographic baseline)
- ADR-0021 (TUF adapted for offline)
- ADR-0023 (egress-free CI)
