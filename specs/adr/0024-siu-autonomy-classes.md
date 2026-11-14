# ADR-0024: Scientific Intelligence Autonomy Classes A0–A4

- **Status:** Accepted
- **Date:** 2026-08-30
- **Deciders:** Aether Architecture Review Board
- **Supersedes:** —
- **Superseded by:** —

## Context

"AI in engineering" is the place where architectural clarity matters most. Without explicit autonomy classes, the same decision is sometimes made by software and sometimes by humans, and the boundary drifts. In a certification context, this drift is unacceptable.

## Decision

The Scientific Intelligence (SI) layer SHALL operate under five **autonomy classes** (SDP §8.5). Every SI output is a `Decision` node tagged with its autonomy class; the policy engine enforces the class.

| Class | Meaning | Default policy |
|---|---|---|
| A0 | Annotate only | always allowed |
| A1 | Adjust sampling/fidelity inside a running study, within declared budget | allowed with notification |
| A2 | Select tools/models for non-released studies | allowed, logged |
| A3 | Gate a surrogate as sufficient for a **verification-relevant** claim | **requires named human sign-off** |
| A4 | Trigger physical manufacturing/test | **forbidden** (SI proposes, humans dispose) |

**A4 is structurally impossible**: no policy bundle can grant A4 authority. SI proposes; humans dispose.

**Constraints:**

- Every `Decision` node carries its autonomy class as a required property
- Class changes are policy-bundle events: signed, audited, versioned
- The autonomy class of a `Decision` determines the policy check that gates it
- A3 sign-off is a policy-controlled graph transition with dual authorization (two-person rule for verification-relevant claims)
- A `Decision` is reversible if its declared `reversal_conditions` are met; reversal itself is a `Decision` node

**MVR-15 (governor replacement)** requires that swapping a governor does not change the autonomy class of its decisions — the class is a property of the policy, not the implementation.

## Consequences

- Positive: the certification-relevant decision boundary is mechanically enforced
- Positive: every SI output is auditable
- Negative: A3 workflows require explicit human-in-the-loop plumbing
- Negative: SI value depends on policy quality; bad policy can starve the system of useful recommendations

## Enforcement

- Decision schema requires `autonomy_class`
- Policy engine enforces class-gated transitions
- ARB approves autonomy policy changes; class changes are auditable
- Red-team tests in Phase 8 attempt to exceed class; success is a release blocker

## References

- SDP §2 I-7 (physics-first, fidelity-honest)
- SDP §8.1 (mandate and non-mandate)
- SDP §8.4 (evidence contracts)
- SDP §8.5 (autonomy classes)
- SDP §8.6 (learning discipline)
- SDP §20 MVR-15
- ADR-0007 (physics-first, fidelity-honest)
- ADR-0012 (fail-visible, never fail-silent)
