# ADR-0041: Contract compatibility and migration

**Status:** Accepted
**Date:** 2026-11-24

## Context

A3-I19 requires explicit schema evolution, tested backward compatibility and
reversible migrations where feasible. The new §20 contracts and completed ASM
families will change schemas. ADR-0011 covers semver discipline but not
migration execution.

## Decision

Each contract package carries a `migrations/` directory with ordered,
deterministic migrations and fixtures for the previous and current versions.
A breaking change requires a major version and a tested migration path.
Readers accept the current and previous minor version; writers emit the
current version. The `schema_semver` fitness check is extended to run
compatibility fixtures.

## Consequences

- Old artifacts remain readable during a transition window.
- Migration failure is loud and reversible.
- Contract versions are part of provenance.
