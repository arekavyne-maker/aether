# ADR-0023: Egress-Free "Dark Build" CI Runner

- **Status:** Accepted
- **Date:** 2026-08-30
- **Deciders:** Aether Architecture Review Board
- **Supersedes:** —
- **Superseded by:** —

## Context

If the platform's CI fetches dependencies from the internet, then (a) the build is not reproducible without that network, (b) a supply-chain compromise of any dependency directly compromises the platform, and (c) the air-gap invariant (I-5) is violated by the build system itself.

## Decision

The Aether CI SHALL run on **egress-free "dark build" runners**. The runners have no route to the public internet; all dependencies are vendored into the monorepo or into a sealed local cache populated by an out-of-band process.

**Rationale (SDP §2 I-5, §19.5):**

- **Reproducible** — the same inputs produce the same outputs, every time
- **Supply-chain hardened** — a compromised upstream cannot silently inject a new dependency
- **Self-consistent** — the build system honors the air-gap invariant it produces
- **Verifiable** — the runner's network policy is part of the platform's threat model

**Constraints:**

- All Rust crate dependencies are vendored (Cargo vendor); all Python dependencies are vendored (pip wheelhouse)
- Docker base images for Apptainer/SIF are pulled and content-addressed before the runner is sealed
- No CI step may invoke a network endpoint; the runner's firewall rejects all egress
- The vendored cache is refreshed by a controlled, audited process — not by CI
- The platform itself is the test: a release built on the dark runner must be installable on an air-gapped site

**Walking skeleton validation:** the Phase 0 walking skeletons (a), (b), (c) are built and exercised on the dark runner before any release.

## Consequences

- Positive: the build pipeline cannot be a vector for supply-chain compromise
- Positive: the air-gap invariant is exercised end-to-end
- Negative: vendoring is operationally heavier than live fetches
- Negative: dependency upgrades require a controlled, audited process

## Enforcement

- CI runner firewall config: deny-all egress (`infra/ci/dark-build.yml`)
- `cargo vendor` and `pip wheelhouse` produce sealed inputs
- `aether validate` re-checks that release artifacts are reproducible from the vendored inputs

## References

- SDP §2 I-5
- SDP §19.5
- SDP §20 MVR-08
- ADR-0005 (air-gap by design)
- ADR-0021 (TUF adapted for offline)
