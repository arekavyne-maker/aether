# Aether 3.0 — Capability Gap Matrix

**Status:** proposed target mapping, 2026-11-24.
**Method:** every row is grounded in [`AS-IS.md`](AS-IS.md) and the current
tests/artifacts. Maturity labels are `implemented`, `partial`, `missing`,
`blocked by dependency`, `research prototype`.

| ID | Capability | Current evidence | Maturity | Gap to Aether 3.0 | Phase |
|---|---|---|---|---|---|
| G-01 | Microkernel and bounded contexts | 19 crates; domain logic outside kernel | implemented | Formal service API and charter; do not grow the 195-LOC IPC binary into a monolith | 0 |
| G-02 | Event-sourced graph | 61 nodes, 32 edges, hash chain, AS-OF, SQLite/Postgres | partial | Full AQL, merge semantics, configuration snapshots, access-controlled queries | 1 |
| G-03 | Provenance | Merkle chain, PROV-O, signed records | implemented | Evidence classes, credibility records, gates, waivers, release packages | 1 |
| G-04 | Identity and policy | 8 roles, action matrix, fail-closed RBAC | implemented | Separate safety, security, export-control, quality dimensions; decision recorded on graph events | 1 |
| G-05 | Plugin runtime | stdio JSON-RPC, run tokens, 15 plugins | partial | Sandbox, capability enforcement, quotas, network allowlist, attestation | 7 |
| G-06 | Contracts | 24 packages, 48 schemas, semver, conformance | partial | Two implementations for every stable contract; standards round-trip | 1–4 |
| G-07 | Workflow engine | DAG, run/sweep/gate/noop, checkpoints, retries | partial | Loops, branches/joins, nested workflows, approval gates, compensation, caching | 4 |
| G-08 | Compute federation | Backend trait; Local real; others fixtures | blocked by dependency | Real HPC/cloud/edge exercise; idempotency, resume, quotas, attestation | 7 |
| G-09 | Units and frames | Typed quantities + frame graph library; CLI surfaces | partial | Solver adapters must consume typed quantities end to end | 1 |
| G-10 | Aerospace Systems Model | Lifecycle/revision/interface/snapshot library + contract + graph nodes | implemented | Bind every domain adapter to ASM snapshots | 1 |
| G-11 | Requirements and V&V | Requirement, SysML import, coverage report | partial | VerificationCase, ValidationCase, ComplianceFinding, ReviewDecision, traceability | 1–2 |
| G-12 | Geometry | Recipe + bounding-box fixture | partial | Real B-rep/STEP AP242, conversion-error tracking, semantic-loss reports | 2 |
| G-13 | Materials | Material node and assignments | partial | Cards, allowables, pedigree, process pairs, temperature/rate, statistics | 2–3 |
| G-14 | Loads and environments | LoadCase node | partial | Flight-regime envelope, spectra, thermal/acoustic/icing/corrosion environments | 2 |
| G-15 | Physics | Beam, Poiseuille, CalculiX LE1, lifting line, sizing | partial | Real CFD/FEA/thermal/acoustics/fatigue/impact breadth; validity envelopes | 2–4 |
| G-16 | Multi-fidelity planning | Deterministic cost/error planner + CLI | partial | Uncertainty, consequence, and evidence-aware escalation policy | 4 |
| G-17 | Uncertainty | Sobol/LHS/MC, GP/POD | partial | Separate uncertainty classes, model-form, manufacturing, OOD, reliability | 4 |
| G-18 | Optimization and design exploration | NSGA-II/III, surrogate-assisted, verified optimum | partial | Adjoint/topology, quality-diversity, generative/developmental, feasibility pipeline | 5–6 |
| G-19 | Digital twin | KF scalar/vector, synthetic data | partial | Real rig ingestion, twin maturity levels, service feedback, model updates | 3–5 |
| G-20 | Manufacturing | Genealogy, as-built, disposition, and impact library + CLI; process contract/fixtures | partial | Bind to real machine/PLC records; material allowables and process twins | 3 |
| G-21 | Quality and inspection | Inspection-plan nodes, CMM/UT fixtures, evidence gates | partial | Real QIF/NDT records and acceptance linkage | 3 |
| G-22 | Test and correlation | test.rig contract and fixture | partial | Real DAQ, channel mapping, correlation, uncertainty | 3 |
| G-23 | Evidence and release gates | Evidence classes, credibility, gates, waivers, release packages | implemented | Wire graph nodes to every run and release package | 1 |
| G-24 | Model registry and AI governance | Model-card contract, registry, promotion gates, audit chain | partial | Real model runtime, drift/OOD, rollback | 6 |
| G-25 | Agent runtime | none | missing | Typed tools, permission matrix, proposal-only workflow, prompt-injection defenses | 6 |
| G-26 | Standards and interoperability | io.format descriptors | partial | STEP AP242, QIF, OPC UA, MTConnect, FMI, HDF5/CGNS/VTK adapters | 2–7 |
| G-27 | Security and resilience | TUF/Ed25519, signed tokens, hash chain, DR tests | partial | Real policy, sandbox, key rotation, restore drills, observability | 1/7 |
| G-28 | CLI | schema/status/policy/provenance/validation/plan/config/evidence/manufacturing/model/fidelity surfaces | implemented | `--dry-run`, `--offline`, full `--format yaml` | 1 |
| G-29 | Research and governance docs | Architecture, ADRs, gaps, S-rollups | partial | AS-IS, GAP-MATRIX, research register, traceability, full threat model, status | 0 |
| G-30 | Performance evidence | 10M graph scale, 11 benchmarks | partial | CLI latency, artifact I/O, workflow overhead, plugin startup, restore, agent latency | 7 |

## Invariant coverage

| A3 invariant | Current coverage | Gap |
|---|---|---|
| A3-I01 physics before prediction | I-7 result labels | Surrogate validity envelope and fallback policy |
| A3-I02 evidence before authority | Evidence node | Gate engine and approval records |
| A3-I03 configuration explicit | Input hashes | Full configuration snapshots |
| A3-I04 fail closed | Typed errors | Policy/evidence fail-closed paths |
| A3-I05 human authority | RBAC roles | ReviewDecision/approval objects |
| A3-I06 reproducibility | Envman, replay | Snapshot-bound runs |
| A3-I07 core small | Crate boundaries | Keep new primitives thin |
| A3-I08 data portable | Export/replay | Standards round-trip |
| A3-I09 uncertainty | Free-form result field | Structured uncertainty components |
| A3-I10 model scope | None | Model cards and validity envelopes |
| A3-I11 manufacturing in design | Manufacturing contracts | Genealogy and process feedback |
| A3-I12 resilience | Checkpoints, DR tests | Tested restore and degraded modes |
| A3-I13 zero trust | Signed tokens/TUF | Real policy and sandbox |
| A3-I14 no silent mutation | Revisions | Lifecycle enforcement across new nodes |
| A3-I15 research quarantined | Labels | Promotion workflow |
| A3-I16 units and frames | Typed quantities/frame graph | Adapters consume quantities end to end |
| A3-I17 negative results | Gaps log | Machine-readable rejection reasons |
| A3-I18 compute replaceable | Backend trait | Real multi-backend exercise |
| A3-I19 interfaces versioned | Semver | Compatibility matrix |
| A3-I20 safety/security separate | Single policy dimension | Separate policy dimensions |
| A3-I21 AI advisor | No agents | Agent runtime with denied authority |
| A3-I22 no fabricated maturity | Honesty discipline | Generated status and claim checks |

## Priority

1. **Blocking:** solver adapters consuming typed quantities; bind evidence
   gates to graph events; evidence-native release workflow.
2. **Next:** real B-rep/STEP, materials/loads breadth, verification/validation
   nodes across the wing-box slice.
3. **Then:** manufacturing/test fixture replacement, fidelity/UQ depth.
4. **Later:** model runtime/agents, plugin sandbox/federation/hardening.
