# Aether 3.0 — Verification Traceability Matrix

**Status:** initial matrix, 2026-11-24.
**Scope:** Phase 1 and Phase 2 requirements. Expand as phases land.
**Rule:** every requirement has an owner, an implementation reference, a test,
and an evidence artifact. A requirement without a test is not done.

| Requirement | Source | Implementation | Test / evidence | Status |
|---|---|---|---|---|
| A3-REQ-001 Typed quantities | A3-I16 | `crates/aether_quantities` | unit conversion and dimension tests | verified |
| A3-REQ-002 Frame graph | A3-I16 | `crates/aether_quantities::frames` | transform composition, handedness, provenance tests | verified |
| A3-REQ-003 Unit/frame rejection before solver | A3-I04, A3-I16 | CLI/exec boundary validation | mismatch integration test | implemented |
| A3-REQ-004 ASM object lifecycle | A3-I03, A3-I14 | `crates/aether_asm`, graph catalog | create/revise/supersede tests | verified |
| A3-REQ-005 Interface compatibility | A3-I03 | ASM interface schema | incompatible interface rejected | verified |
| A3-REQ-006 Configuration snapshot | A3-I03, A3-I06 | `ConfigurationSnapshot` node/artifact | reconstruct exact revisions; invalidation test | verified |
| A3-REQ-007 Artifact integrity | A3-I06 | `crates/aether_artifacts` | tamper/dedupe/missing tests | verified |
| A3-REQ-008 Evidence gate fail-closed | A3-I02, A3-I04 | `crates/aether_evidence` | missing/stale evidence blocks release | verified |
| A3-REQ-009 Authorized waiver | A3-I05 | evidence + policy | unauthorized waiver rejected; authorized recorded | verified |
| A3-REQ-010 Real policy evaluation | A3-I13 | `aether_policy` | unknown action denied; decision on event | verified |
| A3-REQ-011 CLI schema discovery | A3-I10 | `aether schema list/show` | golden JSON tests | verified |
| A3-REQ-012 Status/policy/provenance explain | A3-I10 | CLI commands | golden tests | verified |
| A3-REQ-013 Metallic wing-box vertical slice | Phase 2 | example + workflow | end-to-end CLI execution | verified |
| A3-REQ-014 Reproducible result | A3-I06 | envman + snapshot + replay | same result within tolerance offline | verified |
| A3-REQ-015 Dependent invalidation | A3-I14 | graph impact query | changed load invalidates result/evidence | verified |
| A3-REQ-016 No certification overclaim | A3-I22 | docs/status/claim scan | claim-scan fitness check | verified |
| A3-REQ-017 Model card lifecycle | A3-I10, A3-I15 | `aether.model-card` | promotion blocked without benchmark | planned |
| A3-REQ-018 Agent cannot mutate released baseline | A3-I21 | `aether.agent-tool` policy | denied-action integration test | planned |
| A3-REQ-019 Manufacturing genealogy | A3-I11 | graph nodes + manufacturing contract | design→batch→inspection trace test | planned |
| A3-REQ-020 Threat model coverage | A3-I13 | `docs/threat-model/THREAT-MODEL.md` | review sign-off | verified |

## Evidence artifact conventions

- Test output: `docs/verification/baseline/<date>/<suite>.txt`
- Benchmark output: `benchmarks/AETHER-BENCH-*.json`
- Gate verdict: `evidence/release/<baseline>/<gate>.json`
- Claim scan: `docs/verification/baseline/<date>/claims.json`

## Maintenance

The traceability matrix is checked by `infra/fitness/traceability.py`. A row
with `planned` is allowed during Phase 0; a row with `implemented` must name a
test that exists. Phase gates require every in-scope row to be `verified`.
