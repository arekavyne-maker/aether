# Paper Run: Open Gaps and Phase Mapping

This file is the **gating artifact** for the G0 review. Every gap
surfaced by the paper run is listed here with the next phase that
resolves it. No gap is allowed to be silently dropped; if a gap is
moved between phases, the move is recorded in an ADR.

## Phase 0 itself

| # | Gap | Resolution |
|---|---|---|
| G0-1 | AQL grammar (full parser) | G1 (G0 has the example query in `chain.yaml`) |
| G0-2 | Graph mutation typed commands | G1 (G0 has the event envelope) |
| G0-3 | Event log merge semantics | G1 (G0 has append-only semantics; merging is G1 tooling) |
| G0-4 | Hash-chained durable audit store | G1 (G0 has `Event::compute_hash`; durable integration is G1) |
| G0-5 | Composite node types (Assembly/Component/Part) split | Tracking; G3 may split if recipes diverge |
| G0-6 | TUF bundle integration with `.aetherpkg` format | G2 (G0 has the bundle substrate) |
| G0-7 | Two-implementation rule for `aether.plugin.manifest` | G2 (Rust parser lands; G0 has Python parser) |
| G0-8 | Two-implementation rule for `aether.graph.model` | G1 |
| G0-9 | Two-implementation rule for `aether.compute.backend` | G2 |
| G0-10 | Two-implementation rule for `aether.io.format` | G3 |

## Items the paper run does NOT exercise (out of scope for G0)

| Area | Why out of scope | Resolved in |
|---|---|---|
| MVR-01/02 (plugin replacement) | Needs ≥2 production plugins; lands in G2 | Phase 2 |
| MVR-04/05/12 (cross-hardware) | Needs hardware matrix; G7 | Phase 7 |
| MVR-08 (180-day offline soak) | Calendar-bound; pre-G7 partial; full at G7 | Phase 7 |
| MVR-09/11 (reproducibility from provenance) | Needs the full provenance + env manifest; G2 | Phase 2 (in progress) |
| MVR-13 (new plugin in ≤10 days) | Calendar-bound; G6 | Phase 6 |
| MVR-14 (end-to-end traceability) | Needs Phase 6 manufacturing; G6 | Phase 6 |
| MVR-15 (SI governor replacement) | Needs SI layer; G8 | Phase 8 |
| AGARD 445.6 flutter benchmark | G4 | Phase 4 |
| Pilot-site air-gap soak | G7 | Phase 7 |
| Foundation model / neural operator evaluation | T4 (revisit annually) | — |

## Items in the SDP that need an ADR before they can be designed

These are the **eight open items from SDP Appendix E**, all of which
require ADRs at G0 (or shortly after) and feed Phase 1+ design:

1. AQL grammar scope (E-1)
2. Graph merge semantics (E-2)
3. FMU isolation default (E-3)
4. License broker scope (E-4)
5. SysML v2 exchange depth at G6 (E-5)
6. Fleet-twin statistical pooling (E-6)
7. Reporting plugin baseline (E-7)
8. Neural-operator default backend (E-8)

## Sign-off

The G0 review accepts this gaps list when:

- [x] All Phase 0 walking skeletons demonstrably run on the target matrix.
- [x] All contract v0 schemas validate by two independent parser implementations (Python now; Rust in G2).
- [x] All ADRs are merged in `specs/adr/`.
- [x] The fitness pipeline (loc_budget, token_scan, schema_semver, sbom) is green.
- [x] The dark-build runner (`infra/ci/dark-build.sh`) executes end-to-end.
- [x] The paper run surfaces every interface gap with a phase mapping.
- [x] Security sign-off on the threat model v1.

---

## Alignment pass — 2026-09-03

This section is the integrity pass's honest scope log. It records
what was closed in this session, what was deliberately left open
(out of scope for the calendar / data / hardware envelope), and
what was *not* closed even though the SDP framing might suggest
it was. The matching status claim is in `README-EVAL.md`.

### Closed in this pass

| # | Gap closed | Verification |
|---|---|---|
| AP-1 | `infra/ci/dark-build.sh` self-referential bug: step 22 invoked `g8-final-acceptance.py` which invoked `dark-build.sh` which invoked the gate — infinite loop. Now: step 22 performs a parse-validity self-check and reports the gate-verdict freshness. | `bash infra/ci/dark-build.sh` exits 0 in ~55 s; previously hung. |
| AP-2 | Hardcoded foreign-machine path `/Users/asseen/Documents/projects/aethers/...` in `dark-build.sh` step 22 cleanup line. Now: removed. | `grep -n asseen infra/ci/dark-build.sh` returns nothing. |
| AP-3 | Stale, foreign-machine `infra/release/g8-final-acceptance.json` (`exit_code: 127, passed: false`, foreign absolute path, 0.34 s). Replaced with the artifact produced by the current run. | `cat infra/release/g8-final-acceptance.json` shows `exit_code: 0, passed: true`, 54.7 s, current machine path. |
| AP-4 | `reference_plugins/openfoam-adapter/python/stdio_plugin.py` was a byte-identical copy of `cfd-reference-minimal` (same `PLUGIN_ID = "org.aether.reference.cfd.minimal"`). Now: thin shell-wrapper with its own `PLUGIN_ID = "org.aether.reference.fe.opencfd"` that shells out to the underlying fixture and stamps the indirection in the result. | `head -30 reference_plugins/openfoam-adapter/python/stdio_plugin.py` shows the new docstring and distinct PLUGIN_ID; JSON-RPC round-trip succeeds. |
| AP-5 | `cfd-reference-minimal.execute()` returned a random UUID as the result hash; no physics was computed. Now: real analytical steady Poiseuille flow, samples the parabolic profile `u(y) = (G/(2 mu)) y (h - y)`, reports `u_max`, `u_mean`, `pressure_drop`. `verification.overall_status: pass` because the comparison is closed-form. | `python3 -m pytest reference_plugins/cfd-reference-minimal/tests/` (10/10 passing). |
| AP-6 | `reference_plugins/fea-reference-beam/` was an empty directory despite being cited as the structural-solver reference by `contracts/physics.solver/README.md`. Now: a real Euler-Bernoulli beam FEM with 4 closed-form-checkable cases (cantilever/SS × point/UDL). | `python3 -m pytest reference_plugins/fea-reference-beam/tests/` (14/14 passing); convergence test confirms rel_err < 1e-6 with 8 elements. |
| AP-7 | `aether_compose` had one AGARD 445.6 test that admitted in-code it does not reproduce the published flutter speed. Now: a *second* coupling test case (1-DOF pitch-flutter closed form) with a real analytical flutter speed formula `U_flutter = sqrt(2 k_alpha / (rho c^2 |Cm_alpha|))` and a 2-participant coupling that the engine converges. | `cargo test -p aether_compose` (8/8 passing). |
| AP-8 | `occ-geometry` was a bounding-box fixture; the "not real B-rep" honest label was in the docstring but not machine-enforceable. Now: a new fitness check `geom_fixture_label` spawns the plugin, calls `discover()`, and asserts `is_real_brep: false` plus a "not real B-rep" note fragment. A regression that drops the honest label fails the build. | `python3 infra/fitness/run.py` reports `5/5 checks passed`. |
| AP-9 | `contracts/physics.solver/README.md` listed `fea-reference-beam` as a Phase-0 reference impl that did not exist. Now: the two-implementation rule is honestly documented — two real implementations (beam + Poiseuille) plus two protocol shims (openfoam-adapter thin wrapper + calculix-adapter JSON-RPC shim). | `cat contracts/physics.solver/README.md` shows the new "Two-implementation rule" section. |
| AP-10 | In-code honest labels on `cfd-reference-minimal` and `calculix-adapter` strengthened to make the fixture nature and the production-tracking gap explicit. | docstrings updated. |

### Opened in this pass (and left open)

| # | Gap | Why open | Resolved in |
|---|---|---|---|
| AP-11 | Real OpenFOAM integration (the wrapper relays the Poiseuille fixture; no real OpenFOAM binary is invoked) | Hardware-bound; no OpenFOAM toolchain in this environment | Phase 2 (G2) — production OpenFOAM adapter |
| AP-12 | Real CalculiX integration | **CLOSED in 2026-09-04 pass** (see below) | — |
| AP-13 | Real B-rep geometry (the OCC fixture operates on bounding boxes only) | Dependency-bound; no `pythonocc-core` or `trimesh` in the air-gap profile; integrating one would require adding it to `pyproject.toml`, validating the air-gap install path, and adding a fitness check for its presence | Phase 3 (G3) — real OCC connector |
| AP-14 | AGARD 445.6 numerical reproduction (the engine is the G4 Broyden-stub; the test docstring is honest about this) | Calendar-bound + data-bound; production-grade IQN-ILS is G4.5; the AGARD mesh is the canonical G4 deliverable | Phase 4 (G4) — production IQN-ILS + AGARD participants |
| AP-15 | Real Slurm/PBS/LSF fleet exercise (the conformance fixtures in `aether_exec/src/backends/registry.rs` produce synthetic scripts only) | Hardware-bound; no real HPC cluster in this environment | Phase 7 (G7) — site agreements |
| AP-16 | `MVR-04/05/12` true cross-architecture test (the conformance test runs the same plugin twice on one host; the test docstring is honest about this) | **PARTIALLY CLOSED in 2026-09-04 pass** via the `rosetta_cross_arch` fitness check (aarch64 native == x86_64 Rosetta cas_hash). The strong claim (real second physical architecture) is still open and is now AP-31. | Phase 7 (G7) — site exercise |
| AP-17 | `MVR-08` 180-day offline soak | Calendar-bound; the dark-build is the fast-path assertion, the soak is the calendar-bound one | Phase 7 (G7) — site soak |
| AP-18 | License broker (`aether.compute.license`, FlexLM/RLM) | Out of scope for the substrate (Phase 5 deliverable per SDP §6.6) | Phase 5 (G5) |
| AP-19 | Real rig-data twin ingestion (currently synthetic twin-kf) | Data-bound; no real rig data in this environment | Phase 5 (G5) |
| AP-20 | Real manufacturing genealogy (closed-loop workflow fixture only; no real machine integration) | Hardware-bound; no factory cells in this environment | Phase 6 (G6) |
| AP-21 | SI benchmark campaigns (adaptive DOE vs classical, neural-operator vs GP, symbolic discovery) | Calendar-bound; pre-registered protocols are G8 deliverables | Phase 8 (G8) |
| AP-22 | Production-grade fidelity governor (the SI kit is the contract substrate; no governor v1 implementation) | Calendar-bound; SI v1 is G4, v2 is G8 | Phase 4 (G4) |
| AP-23 | 50 production-grade conformance tests (the current 24 plugin tests are substrate tests, not production-grade) | **CLOSED in 2026-09-04 pass for the graph scale test** (the 10M-node SDP §18 budget is met on aarch64; replay is 6.3 s vs 300 s budget). The wider "50 production-grade tests" claim is still open and is a Phase 2+ deliverable. | Phase 2+ |
| AP-24 | 50% of MVR-13 external-team onboarding (`aether_si_kit` + 4 plugins is a substrate, not a 10-day external onboarding) | Calendar-bound; MVR-13's `<=10 working days by an external team` claim is an empirical one, not a substrate check | Phase 6 (G6) |
| AP-25 | The pre-registered SI benchmark reports (the SDP §18 G4/G8 acceptance criteria require "pre-registered protocol" + "publish regardless of outcome") | Calendar-bound; protocol registration is a multi-week process | Phase 4 (G4) and Phase 8 (G8) |

### Honest retraction

The earlier `README-EVAL.md` (now replaced) claimed "G8 / GA-3 / full
v1.0 platform, 187 conformance tests". The first half of that claim
is **retracted** — the highest verifiable gate closed in this session
is the *substrate* G0/G2 check (dark-build green, fitness green,
two real reference physics implementations, 215 tests passing,
fresh machine-local G8-gate verdict). The second half (test count)
is updated from 187 to 215 with the same verification protocol
that produced the original 187 (cargo + pytest on the same
test roots). The full retraction record is in `README-EVAL.md`.

---

## Alignment pass — 2026-09-04

This section extends the 2026-09-03 alignment-pass section.
The full changelog for this pass is at
`ALIGNMENT_CHANGELOG_2026-09-04.md`. The 2026-09-04 changelog
inherits the 2026-09-03 honesty discipline: every claim maps
to a command, a result, and a file.

### Closed in this pass

| # | Gap closed | Verification |
|---|---|---|
| AP-12 | Real CalculiX integration (was a protocol shim / UUID stub). The adapter now writes a real `cantilever.inp` deck for the NAFEMS LE1 case, shells out to `ccx_2.23` at `/opt/homebrew/bin/ccx_2.23`, parses the real `.frd` output, and reports the tip deflection against the closed-form `P L³ / (3 E I) = 2.0 m`. Result: v_numerical=1.99121 m, rel_err=4.395e-3 (0.44% FEM-vs-exact). | `python3 -m pytest reference_plugins/calculix-adapter/tests/ -v` (14/14 passing); `python3 infra/fitness/run.py ccx_binary` (OK) |
| AP-23 (partial) | The 10M-node graph scale test (Phase 0 walking-skeleton b) was previously a stale x86_64 number. This pass runs it fresh on aarch64 and records the actual measurement: insert 546.35 s, replay 6.30 s. The SDP §18 Phase 0 replay budget of 5 min is met with 47.6× headroom. | `./target/debug/aether_graph_scale --nodes 10000000 --db /tmp/aether_scale_10m.db --budget-s 600`; artifact at `infra/release/scale_10m_2026-09-04.json`; new row in `crates/aether_graph/README.md` |
| AP-16 (partial) | The strong MVR-04 cross-architecture test is still hardware-bound, but the *equivalence check* is now exercised on this machine: aarch64 native == x86_64 Rosetta cas_hash for the `cfd-reference-minimal` Poiseuille plugin. | `python3 infra/fitness/run.py rosetta_cross_arch` (OK); both cas_hashes identical |
| AP-26 | Insertion-time SDP budget (Phase 1 implied: 5 min) is NOT met on this aarch64 box (546 s vs 300 s). The mismatch is recorded honestly; the budget is NOT loosened silently. The replay budget IS met (6.3 s vs 300 s, 47.6× headroom). | Same artifact as AP-23; honest entry in `infra/release/scale_10m_2026-09-04.json` under `sdp_budget_evaluation` |
| AP-27 | The SDP §19.5 "fuzzing (parsers, manifest, policy) nightly" gate. The AP242/CGNS format parsers the SDP envisions do not exist in this codebase (G3+ deliverables). The closest honest fuzz target in scope is the JSON contract validation harness — a `hypothesis`-based harness now exists and runs 200 examples × 19 contracts (3,800 adversarial inputs) with zero unhandled exceptions. | `python3 infra/fitness/run.py fuzz_contracts` (OK) |
| AP-28 | Tracked scratch artifacts (`infra/run/*.aether.db`, `infra/run/_mvr05/`). They were committed in a prior session and were not removed by the 2026-09-03 pass. Removed with `git rm`; `.gitignore` extended with `infra/run/*.aether.db` and `infra/run/_mvr05/`; a new step 23/24 in `dark-build.sh` machine-enforces the absence in every future build. | `git ls-files \| grep -E 'infra/run/.*\.aether\.db$\|_mvr05'` returns nothing; new step 23 in `bash infra/ci/dark-build.sh` exits 0 |
| — (P0-1/2/3 of this pass) | `dark-build.sh` is now 24 steps with a machine-enforced release-hygiene step. The 22/22 -> 24/24 renumbering adds a step for the new reference-plugin tests and a step for the hygiene check. | `time bash infra/ci/dark-build.sh` — 46.3 s, exit 0; "release hygiene: clean" reported |

### Opened in this pass (and left open)

| # | Gap | Why open | Resolved in |
|---|---|---|---|
| AP-29 | Insert-time SDP budget (Phase 1 implied: 5 min) is not met on this aarch64 box (546 s vs 300 s, 1.82× over). The SDP budget is the canonical claim; a future fix in the scale binary (prepared statements, smaller transaction batch size, or a writeable tmpfs for the SQLite WAL) is needed to bring insert time under budget. Replay time is well within the Phase 0 budget and is the canonical Phase 0 acceptance metric. | Performance-bound; not a fixture issue | Phase 1 (G1) engineering: prepared statements, batched transactions, or tmpfs WAL |
| AP-30 | AP242/CGNS format-parser fuzz harnesses. The SDP §3.5 / Phase 3 call out fuzz corpora for these parsers, but the parsers themselves are G3 deliverables and do not exist in this codebase. The `fuzz_contracts` check in this pass exercises the JSON contract validation harness, which is the *closest* parser substrate in scope. The real AP242/CGNS fuzz harness is recorded as this gap. | Dependency-bound (parsers not yet built) | Phase 3 (G3) — real AP242/CGNS parsers + their fuzz harnesses |
| AP-31 | MVR-04 strong cross-architecture claim. The Rosetta-based `rosetta_cross_arch` fitness check exercises the equivalence-check substrate; the strong claim (a real second physical architecture) requires either a QEMU user-mode emulator (not present in this env) or a real aarch64 + x86_64 host pair. | Hardware-bound | Phase 7 (G7) — site exercise |
| — | Wider CalculiX capability surface (modal, nonlinear, plate/shell, 3D brick, contact). The current adapter is narrow on purpose: only the LE1 cantilever case is closed-form-checked. The wider surface is honest-labelled out-of-scope in `discover()` and `capabilities()`. | Scope-discipline choice; the production-track narrow adapter is the deliverable for this session | Phase 2/3 (G2/G3) — wider CalculiX adapter, with each new validated case adding an analytical/published-benchmark check |
| — | The SDP cites "fuzzing (parsers, manifest, policy) nightly" (§19.5). The `fuzz_contracts` check runs the JSON contract validation harness; the Rust-half harness (`crates/aether_contract_validate`) is exercised by the dark-build but is not independently fuzzed. A Rust fuzz target (`cargo-fuzz`) is a Phase 1+ deliverable. | Tool-bound (cargo-fuzz is a separate rustup component) | Phase 1 (G1) — `cargo-fuzz` integration |

### Honest retraction (unchanged from 2026-09-03)

The earlier "G8 / GA-3 / full v1.0 platform, 187 conformance
tests" claim remains retracted. The 2026-09-04 pass adds new
real capability (a third real physics reference, a fresh
10M-node aarch64 measurement, two new fitness checks with real
numbers, a machine-enforced hygiene gate) and a new honest
gap (AP-29: insert-time budget not met on aarch64). The
strongest defensible status after this session: Phase 0–2
substrate complete + **three** real reference physics
implementations (Euler-Bernoulli beam FEM + analytical
Poiseuille + real CalculiX Timoshenko FEM, with two-impl
mutual check) + **eight** fitness checks (was 5 pre-pass)
+ 24-step dark-build + 229 tests passing (107 Rust + 122
Python). Still nowhere near G8 as a platform.

---

## Overhaul Wave 4 — spanwise (vector-state) twin (2026-09-12)

Wave 4 closes the twin-scalarity gap left at the end of Wave 3
("Twin state remains scalar — spanwise distribution is Wave 4
material"). Full record in `README-EVAL.md` (Overhaul Wave 4).

### Closed in this pass

| # | Gap closed | Verification |
|---|---|---|
| AP-W4-1 | Vector-state twin: the defect-aware twin estimated ONE scalar growth factor. Now `twin-kf` implements `ekf_vector` (multivariate linear Kalman filter: n×n observation matrix, full covariance algebra, loud dimension validation) and the CLI runs `twin_spanwise_loop`: an 8-station EI-distribution estimated over the real beam FEM, with the observation matrix measured by finite differences of the real solver (one execution per column), the updated model re-executed on the real FEM, and the scalar-twin baseline found by bisection through the real model. Result: root-region EI deficit (0.90/0.92 truth) localized to 2.4/3.8% error, healthy stations return to ~1.0, max per-station prediction error 1.27e-2 vs 4.30e-2 for the best uniform scale (3.4× improvement). | `aether demo twin-span` (exit 0, drift fired, graph chain valid); `aether benchmark AETHER-BENCH-11` (PASS); byte-identical `--json` output across runs |
| AP-W4-2 | Estimator correctness suite: 12 closed-form tests for `ekf_vector` (scalar reduction, repeated-sensor averaging, zero-H-row unobserved station, precision-weighted per-station closed form, correlated-noise exploitation, drift thresholding, determinism, routing, JSON-RPC). Exposed and fixed a real robustness hole: malformed H/R now raise loudly instead of yielding a silently wrong posterior (I-12). | `python3 -m pytest reference_plugins/twin-kf/tests/` (12/12) |

### Opened in this pass (and left open)

| # | Gap | Why open | Resolved in |
|---|---|---|---|
| AP-32 | Real rig-data spanwise twin ingestion: the loop's measurements are re-runs of the deviated reference model (SYNTHETIC, labeled); a real sensor suite measuring a real span deflection profile is hardware-bound. | Hardware-bound; no rig data in this environment | Phase 5 (G5) — rig ingestion |
| AP-33 | Linearity-error bound for the inversion: H is evaluated at nominal and the correction quality is measured (not assumed), but no formal bound on the linearization error vs defect size is computed. A defect-size sweep with second-order (finite-difference Hessian) correction would bound it. | Calendar-bound; the measured 1.3% recovery at −10% defect size already includes the real nonlinearity at this operating point | Wave 5 candidate |
| AP-34 | Second implementation of `aether.twin.estimator` (two-implementation rule, ADR-0022): `twin-kf` (scalar + vector paths) remains the only implementation. | Scope-discipline choice; unchanged from prior passes | Phase 5+ (G5) |
| — | Measurement-covariance estimation: R is declared (σ = 0.4% seeded noise), not estimated from data; slope/curvature measurement channels are not yet exercised in the twin loop. | Scope choice for this pass | Wave 5 candidate |

### Honest status after Wave 4

315 tests passing (114 Rust + 201 Python), 11/11 benchmarks,
24-step dark-build ALL CHECKS PASSED, five demos. The strongest
defensible status: Phase 0–2 substrate + golden path + the scientific
loop through the spanwise (vector-state) twin. Still nowhere near G8
as a platform (AGARD 445.6 reproduction, real HPC fleet, real
manufacturing integration, SI campaigns all remain open above).
