# ADR-0036: Job envelope and federated compute

**Status:** Accepted
**Date:** 2026-11-24

## Context

§7.1 defines a common execution envelope and §7.3 requires portable job
submission across local, cluster, cloud, edge, lab and air-gapped targets.
The current `JobSpec` covers job id, plugin, run token, entry path, workdir,
staged paths and placement, but not idempotency keys, configuration snapshot,
environment digest, data classification, licensing, expected outputs or
validation rules. Existing non-local backends are honest fixtures.

## Decision

Introduce a versioned `aether.job-envelope` contract. Every backend consumes
it. A backend that cannot reach its scheduler returns an error; it never
fabricates success. Federation adds signed worker identity, capability
negotiation, checksummed resumable transfer, quotas, cost accounting and
data-locality policy. A loopback two-home harness is permitted but must be
labelled as a harness until a real second site is exercised.

## Consequences

- Backends are interchangeable without changing workflow steps.
- Local mode remains fully usable offline.
- Audit records can bind every result to an exact environment and snapshot.
- Real federation evidence requires a second site; a harness does not count.
