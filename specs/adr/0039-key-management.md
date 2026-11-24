# ADR-0039: Key management and data classification

**Status:** Accepted
**Date:** 2026-11-24

## Context

§15.1 requires short-lived credentials, key rotation/revocation, encryption
at rest and in transit, and signed releases. §15.3 requires project-configurable
classification and a refusal to claim regulatory compliance without a separate
assessment. `aether_identity` issues signed tokens but has no rotation;
`aether_classification` enforces handling but has no key lifecycle.

## Decision

Introduce a site keyring with versioned key ids, rotation and revocation lists.
Tokens carry `kid`, expire, and are checked against the active key set.
Artifacts are encrypted at rest with keys in the keyring and signed manifests
are verified on import/export. Transport encryption is required for any
federated or remote operation. Classification labels gate read/export/federate
regardless of encryption state. No export-control or cybersecurity compliance
is claimed by the presence of these mechanisms.

## Consequences

- Rotation is testable; old tokens fail after the revocation point.
- Lost keys require a documented recovery ceremony, not a silent fallback.
- Federation peers exchange public keys only through signed trust bundles.
