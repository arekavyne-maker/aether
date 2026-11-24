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
| A3-REQ-015 Dependent invalidation | A3-I14 | `crates/aether_asm` snapshots + manufacturing impact | changed object/build invalidates dependent records | verified |
| A3-REQ-016 No certification overclaim | A3-I22 | docs/status/claim scan | claim-scan fitness check | verified |
| A3-REQ-017 Model card lifecycle | A3-I10, A3-I15 | `crates/aether_model_registry` | `model verify` gate and `model promote` | verified |
| A3-REQ-018 Agent cannot mutate released baseline | A3-I21 | `aether.agent-tool` policy | denied-action integration test | planned |
| A3-REQ-019 Manufacturing genealogy | A3-I11 | `crates/aether_manufacturing` | `manufacturing verify/as-built/impact` acceptance | verified |
| A3-REQ-020 Threat model coverage | A3-I13 | `docs/threat-model/THREAT-MODEL.md` | review sign-off | verified |

| A3-REQ-021 Manufacturing genealogy completeness | A3-I11, A3-I14 | `crates/aether_manufacturing` | material/lot/inspection/disposition checks | verified |
| A3-REQ-022 As-built snapshot reproducibility | A3-I06, A3-I11 | `crates/aether_manufacturing` | order-independent manifest hash | verified |
| A3-REQ-023 Design-revision impact analysis | A3-I11, A3-I14 | `crates/aether_manufacturing::impact` | affected builds and revalidation list | verified |
| A3-REQ-024 Role-gated model promotion | A3-I05, A3-I13, A3-I21 | `crates/aether_model_registry` | unauthorized role and non-independent validation refused | verified |
| A3-REQ-025 Deterministic fidelity escalation | A3-I06, A3-I07 | `crates/aether_fidelity` | cheapest-level selection and budget/error conflict explanation | verified |
| A3-REQ-026 Offline export/replay integrity | A3-I06, A3-I08, A3-I12 | `aether export` / `aether replay` | tamper detection and deterministic replay acceptance | verified |
| A3-REQ-027 Sandbox and federation | A3-I13, A3-I18, A3-I20 | `crates/aether_exec` + Phase 7 | real sandbox and second-site exercise | planned |
| A3-REQ-028 Solver adapters consume quantities | A3-I04, A3-I16 | `crates/aether_quantities` + adapters | solver-submit rejection test | planned |
| A3-REQ-029 Evidence-native release workflow | A3-I02, A3-I03 | evidence + ASM + registry | signed release package produced by CLI | planned |

| A3-REQ-030 Solver-submission unit/frame check | A3-I04, A3-I16 | `aether_quantities::validate_submission` | `phase1-units-frames.sh` | verified |
| A3-REQ-031 Workflow branch/join/loop control flow | A3-I06, A3-I07 | `aether_workflow` | `phase4-control-flow.sh` | verified |
| A3-REQ-032 Design feasibility and negative evidence | A3-I09, A3-I17 | `crates/aether_design` | `phase5-design-exploration.sh` | verified |
| A3-REQ-033 Advisor-only agent permissions | A3-I05, A3-I13, A3-I21 | `crates/aether_agents` | `phase6-agents.sh` | verified |
| A3-REQ-034 Sandbox policy enforcement at spawn | A3-I13 | `crates/aether_exec::spawn_sandboxed` | `phase7-sandbox.sh` | verified |
| A3-REQ-035 Evidence-native release package | A3-I02, A3-I03, A3-I05 | `aether_evidence::prepare_release` | `phase2-release-package.sh` | verified |
| A3-REQ-036 Signed plugin bundle verification | A3-I13, A3-I19 | `scripts/plugin-sign.py` | sign/verify round trip | verified |
| A3-REQ-037 Kernel-level isolation | A3-I13 | seccomp/namespaces/cgroups adapter | real host hardening exercise | planned |
| A3-REQ-038 Federation trust exchange | A3-I13, A3-I18 | TUF/federation adapter | two-site signed exchange | planned |
| A3-REQ-039 Solver adapters call units check automatically | A3-I04, A3-I16 | `aether_exec` handler integration | adapter rejects bad submission test | planned |
| A3-REQ-040 Physical validation of emerging structures | A3-I01, A3-I15 | test campaign | coupon/element/component evidence | planned |

## Evidence artifact conventions

- Test output: `docs/verification/baseline/<date>/<suite>.txt`
- Benchmark output: `benchmarks/AETHER-BENCH-*.json`
- Gate verdict: `evidence/release/<baseline>/<gate>.json`
- Claim scan: `docs/verification/baseline/<date>/claims.json`

## Maintenance

The traceability matrix is checked by `infra/fitness/traceability.py`. A row
with `planned` is allowed during Phase 0; a row with `implemented` must name a
test that exists. Phase gates require every in-scope row to be `verified`.
