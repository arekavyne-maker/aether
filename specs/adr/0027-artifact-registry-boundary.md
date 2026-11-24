# ADR-0027: Artifact registry as a separate boundary

**Status:** Proposed
**Date:** 2026-11-24

## Context

Content hashing is currently scattered across CLI and plugin code. The graph
must reference large artifacts (CAD, meshes, fields, scans, telemetry) without
becoming a blob store.

## Decision

Introduce `aether_artifacts` with a small `ArtifactStore` contract:

- put/get/exists/verify by content hash;
- atomic local filesystem implementation;
- scoped read/write handles;
- deduplication and tamper detection;
- future adapters for object stores and federated transfer.

The graph stores metadata, hashes, and references only.

## Consequences

- A single integrity path for all large data.
- Plugins receive scoped artifact handles, not raw paths or DB access.
- Transfer resumption and remote stores can be added behind the contract.
