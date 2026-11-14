# ADR-0005: I-5 Air-Gap by Design

- **Status:** Accepted
- **Date:** 2026-08-30
- **Deciders:** Aether Architecture Review Board
- **Supersedes:** —
- **Superseded by:** —

## Context

Aether's target deployments are air-gapped engineering environments for emerging aerostructures. The platform's correctness claim is meaningless if it depends on internet connectivity, telemetry, or SaaS at runtime.

## Decision

The platform SHALL have:

- **No mandatory cloud, internet, telemetry, or external SaaS dependencies**
- **No silent degradation** into "offline mode"; offline is the default
- **All capabilities** (install, update, run, audit, validate) must function fully offline
- **Egress-free CI** ("dark build") for the platform itself

The HTTP client wrapper used by the kernel (and any optional online feature) **fails closed** when no network is available. The wrapper is mandatory: any direct use of `std::net`, `reqwest`, `urllib`, `requests`, or equivalent is rejected by the fitness pipeline.

**Optional online features** (none currently exist) must degrade silently and cleanly: they fail-closed, return a typed `OfflineError`, and never produce plausible-looking partial output.

## Consequences

- Positive: the platform can be deployed, validated, and certified in fully air-gapped environments
- Positive: eliminates an entire class of supply-chain and exfiltration risks
- Negative: makes some conveniences (auto-update, telemetry, error reporting) impossible without explicit opt-in
- Negative: requires care when depending on third-party crates; transitive network dependencies are forbidden

## Enforcement

- Egress-free "dark build" CI runner (`infra/ci/dark-build.yml`)
- Network call audit via the mandatory HTTP client wrapper; direct network crate use is rejected
- MVR-08 (180-day offline soak) executed before each major release
- All transitive dependencies are vendored; no `cargo install` or `pip install` at runtime

## References

- SDP §2 I-5
- SDP §5.9 (plugin lifecycle air-gap native)
- SDP §7 (air-gapped architecture, security, policy)
- SDP §19.5
- SDP §20 MVR-08
- ADR-0001 (microkernel minimality)
- ADR-0020 (cryptographic baseline)
- ADR-0021 (TUF adapted for offline)
- ADR-0023 (egress-free CI)
