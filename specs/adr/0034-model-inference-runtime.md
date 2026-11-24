# ADR-0034: Model inference runtime boundary

**Status:** Accepted
**Date:** 2026-11-24

## Context

`crates/aether_model_runtime` was added to load a pinned model card, check a
request against its validity envelope, and produce a prediction with an OOD
verdict and a fallback instruction. ADR-0029 governs the registry but does
not cover the inference execution boundary. A3-I01 requires physics-bounded
predictions; A3-I10 requires declared scope; §0.10 requires an ADR before
introducing a model runtime.

## Decision

Model inference is a **bounded advisory service**:

- the model must be in `Approved` state before inference;
- the model artifact hash must match the card;
- inputs must be inside the declared envelope or the prediction is flagged
  `in_domain: false` with a fallback instruction;
- predictions are proposals; they never modify a released baseline;
- the runtime is replaceable behind a trait; the current deterministic
  linear implementation is not a general ML runtime.

## Consequences

- A production model swap requires only a new implementation of the trait
  and a re-approved card.
- Every prediction carries its card version and artifact hash.
- The runtime cannot grant itself authority to approve or release.
- This ADR supersedes the runtime portion left implicit in ADR-0029.
