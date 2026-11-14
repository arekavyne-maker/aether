# ADR-0020: Cryptographic Baseline — Ed25519, SHA-256/3, X.509, TLS 1.3

- **Status:** Accepted
- **Date:** 2026-08-30
- **Deciders:** Aether Architecture Review Board
- **Supersedes:** —
- **Superseded by:** —

## Context

Aether's air-gapped deployments operate over a 15-year horizon. Cryptographic agility is required: today's algorithms may be deprecated, broken, or regulated out of existence within that span.

## Decision

The platform's cryptographic baseline SHALL be:

| Purpose | Algorithm |
|---|---|
| Signatures | **Ed25519** (fast, compact) |
| Content hashes | **SHA-256** (default), **SHA-3** (where mandated) |
| Identity (people, services, keys) | **X.509 PKI** with an **offline root CA** and **online issuing CAs per site** |
| Local network transport | **TLS 1.3** |
| Exported bundle encryption | **age** / libhybrid-pattern envelope encryption |

The **offline root key** is held in an HSM or smartcard ceremony; it signs the site issuing keys and the **release signing keys** used for platform/plugin artifacts.

**Cryptographic agility is mandatory.** Algorithm choices are abstracted behind a `crypto` SPI; rotating SHA-256 to SHA-3 (or Ed25519 to a post-quantum successor) is a configuration + migration change, not a code rewrite.

**Constraints:**

- Private keys never leave their protected store (HSM, smartcard, or `secretref://` resolver)
- The audit log is signed per record, hash-chained across records
- The `RunToken` is Ed25519-signed; includes scope, expiry, and CAS-path allowlist
- `secretref://` references in config are resolved at execution time; the value never enters the graph, logs, or provenance payloads (SDP §4.6)

## Consequences

- Positive: the platform survives a 15-year horizon
- Positive: standard algorithms reduce reviewer overhead
- Negative: the `crypto` SPI is one more thing to maintain
- Negative: HSM/smartcard ceremony is operationally heavy (mitigated by good runbooks)

## Enforcement

- All kernel code uses the `crypto` SPI; direct algorithm use is rejected by the fitness pipeline
- `aether provenance audit --verify-chain` validates signature continuity
- Key rotation runbooks in `docs/operator/`

## References

- SDP §2 (invariants touching security)
- SDP §4.6 (configuration and secrets)
- SDP §7.1 (trust model)
- SDP §7.6 (immutable audit trail)
- SDP §20 MVR-08
- ADR-0005 (air-gap by design)
- ADR-0008 (provenance completeness)
- ADR-0021 (TUF adapted for offline)
