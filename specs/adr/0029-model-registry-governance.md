# ADR-0029: Model registry and model-card governance

**Status:** Accepted
**Date:** 2026-11-24

## Context

Surrogates and AI models can influence engineering decisions. A score or a
successful run is not evidence of validity. Aether needs a lifecycle that
separates research, benchmarked, independently validated, and approved models.

## Decision

Add an `aether.model-card` contract and a model registry boundary:

- model identity and semantic version;
- intended and prohibited use;
- input/output schema and validity envelope;
- training/validation data lineage and rights;
- code and weight digests;
- metrics, uncertainty method, OOD method, fallback solver;
- approval state and rollback version;
- drift monitoring hooks.

Promotion is gated by benchmark evidence and independent review. An agent or
model cannot approve its own release.

## Consequences

- Surrogate results must reference a model card.
- Out-of-envelope use triggers fallback or refusal.
- Rollback is immediate and auditable.
- Registry implementation can start as contract + metadata before a runtime.
