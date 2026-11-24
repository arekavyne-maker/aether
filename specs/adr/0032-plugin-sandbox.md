# ADR-0032: Plugin sandbox and capability enforcement

**Status:** Proposed
**Date:** 2026-11-24

## Context

Plugins are separate processes, but they currently inherit the host
environment, filesystem, and network. Signed manifests and run tokens establish
identity but not isolation.

## Decision

Add a plugin sandbox with:

- declared capabilities and least-privilege scopes;
- explicit filesystem mounts;
- network deny-all by default, allowlist where required;
- CPU/memory/time quotas;
- scoped artifact handles;
- signed package identity and version pinning;
- audit logs for capability use;
- reproducible plugin environments.

A plugin cannot gain broader access because an LLM or workflow requests it.

## Consequences

- Some plugins will need manifest and packaging changes.
- Local development needs a permissive-but-explicit profile.
- Sandboxing is a security boundary, not an engineering-correctness guarantee.
