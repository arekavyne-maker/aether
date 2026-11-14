# ADR-0007: I-7 Physics-First, Fidelity-Honest

- **Status:** Accepted
- **Date:** 2026-08-30
- **Deciders:** Aether Architecture Review Board
- **Supersedes:** —
- **Superseded by:** —

## Context

Surrogates and reduced-order models are powerful accelerants, but they silently produce "plausible-looking wrong" answers when used outside their validity envelope. In a certification-relevant context, this is unacceptable.

## Decision

Every quantitative artifact in the Engineering Graph SHALL carry an **explicit fidelity class** and an **uncertainty statement**. The result schema is normative.

**Fidelity classes** (SDP §9.3):

```
measurement | high_fidelity | medium_fidelity | low_fidelity | surrogate | rom | heuristic
```

Each `Result` node also carries:

- `validation_status`: `unvalidated | checked | verified | invalidated`
- `uncertainty`: structured object (per-quantity bands, calibration slopes/intercepts, envelope edges)
- `envelope`: declared region of validity (input-space coordinates, units, ranges)
- `trust_record`: training data hashes, validation protocol, calibration status, known failure modes (mandatory for `surrogate` and `rom`)

**Fidelity honesty is schema-enforced**: the result schema makes these fields **required**. CI rejects artifacts missing any of them (the "fidelity gate").

**No silent replacement**: a `physics.solver` plugin whose `verify()` method cannot certify convergence (residuals below declared tolerance, conservation checks passed) SHALL NOT emit a `Result` with `fidelity_class: high_fidelity`. The plugin may retry, refine, or fail — but it must not lie.

The Scientific Intelligence (`si.governor`) layer's `SurrogateTrustAssessor` is the structural check that prevents surrogate overtrust (SDP §8.2). Trust gates are schema-mandatory, not optional.

## Consequences

- Positive: every claim in the graph is auditable
- Positive: trust envelopes make "AI for engineering" defensible to certification
- Negative: every plugin author must implement `verify()` honestly
- Negative: some quick-and-dirty workflows become harder to express

## Enforcement

- Result schema (`contracts/io.format/`) requires `fidelity_class`, `validation_status`, `uncertainty`
- CI fidelity gate (`infra/fitness/fidelity_gate.py`) rejects non-conforming artifacts
- `verify()` method in solver contract schema is a required return
- MVR-01/02 + AGARD 445.6 benchmark in Phase 4 (§18)

## References

- SDP §2 I-7
- SDP §5.5 (solver contract `validate`/`verify`)
- SDP §8.4 (evidence contracts)
- SDP §8.6 (learning and model management discipline)
- SDP §9.3 (doctrinal rules for adoption)
- ADR-0002 (everything else is a plugin)
