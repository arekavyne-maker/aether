# ADR-0021: TUF-Adapted-for-Offline Artifact Distribution

- **Status:** Accepted
- **Date:** 2026-08-30
- **Deciders:** Aether Architecture Review Board
- **Supersedes:** —
- **Superseded by:** —

## Context

Aether's offline-media bundle distribution must be tamper-evident and role-separated, but it must also work without a live server: a USB stick carried into an air-gapped site is the realistic channel.

## Decision

Plugin installations, updates, policy distribution, and model/dataset imports SHALL use **TUF (The Update Framework)** metadata **adapted for offline media**. The adaptation is the inclusion of **validity windows** and **site-acknowledged sequence numbers** on the metadata itself, so media carries both content and freshness.

**Rationale (SDP §7.2):**

- **Role separation** means compromise of a packaging key cannot silently rewrite the registry
- **`targets` manifest** lists artifacts with hashes and signatures
- **`snapshot`/`timestamp` roles** bind freshness even when no server exists
- **Offline validity window** lets a site reject bundles that are too old or from the wrong sequence
- **Kernel refuses** installation of any artifact whose chain does not verify against the site's trusted root (I-12)

**TUF role mapping (Aether-adapted):**

| TUF role | Aether manifestation |
|---|---|
| Root | Offline root key (HSM/smartcard); signs targets + snapshot + timestamp metadata keys |
| Targets | Plugin manifests + content hashes for a release |
| Snapshot | Per-release metadata; sequence number |
| Timestamp | Per-bundle metadata; validity window |

**Bundle types** (SDP §7.3): `plugin-bundle`, `update-bundle`, `policy-bundle`, `data-bundle`, `project-package`, `audit-export`. Every bundle carries a TUF-style metadata envelope.

## Consequences

- Positive: tamper-evident offline distribution is mechanically enforced
- Positive: role separation prevents single-key compromise from rewriting the registry
- Negative: the metadata format is non-trivial; tools are required to author and verify bundles
- Negative: media-borne updates have higher operational overhead than network updates

## Enforcement

- Kernel refuses installation of any artifact whose chain does not verify (I-12)
- Bundle authoring tools are versioned; their output is what the kernel accepts
- Walking skeleton (c) (Phase 0) validates the substrate: a signed bundle, a verifier, and a refusal path

## References

- SDP §7.2 (TUF adapted for offline)
- SDP §7.3 (AEP)
- SDP §18 Phase 0 walking skeleton (c)
- ADR-0005 (air-gap by design)
- ADR-0012 (fail-visible, never fail-silent)
- ADR-0020 (cryptographic baseline)
- ADR-0023 (egress-free CI)
