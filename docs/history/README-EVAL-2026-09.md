# Aether Platform — Status Evaluation (alignment passes)

**This file replaces the earlier "G8 / GA-3 / full v1.0 platform"
claim that was checked into this repository.** That claim was not
defensible against the project gate scripts and the SDP. This
document records the **defensible line-by-line status** as of the
alignment passes of 2026-09-03 and 2026-09-04.

The status evaluation is grounded in the project's own definition
of "green" — `infra/ci/dark-build.sh` and
`infra/release/g8-final-acceptance.py` — and in the SDP's
acceptance criteria for each phase. Every number in the table
below comes from code that actually ran in this environment, not
from prose. See "Verification" at the bottom of this document for
the commands and their output.

## Summary

| Claim | Pre-2026-09-03 status | 2026-09-03 status | 2026-09-04 status | Verification |
|---|---|---|---|---|
| Substrate-complete (Phase 0–2) | Partial (kernel + IPC, 4/4 fitness checks green, but reference physics plugins were UUID/hash stubs) | Yes (kernel + IPC + 5/5 fitness checks green, 2 real reference physics implementations with analytical verification) | **Yes** (kernel + IPC + 6/6 fitness checks green, 3 real reference physics implementations; the third is a real native-binary CalculiX adapter, not a shim) | `bash infra/ci/dark-build.sh` exits 0; `python3 infra/fitness/run.py` passes 6/6 |
| G8 / GA-3 / full v1.0 platform | Claimed but false (`infra/release/g8-final-acceptance.json` was a stale, foreign-machine artifact with `exit_code: 127, passed: false`) | Retracted. G8 gate produces a fresh verdict per run, with `passed: true` on the substrate check; this does not mean the platform has reached G8. | **Retracted** (unchanged from 2026-09-03). The substrate has 3 real reference physics impls, 6 fitness checks, 24-step dark-build, and 229 tests passing; the strongest defensible status remains Phase 0–2 substrate-complete. | `python3 infra/release/g8-final-acceptance.py` exit 0, machine-local artifact |
| Test counts | Claimed 103 Rust + 84 Python = 187 (independently verified) | 107 Rust + 108 Python = 215 | **107 Rust + 122 Python = 229** (+14 from `calculix-adapter` tests: 11 in `test_calculix_le1.py` + 3 in `test_cross_plugin_consistency.py`) | `cargo test --workspace` and `pytest` on `python/` + `reference_plugins/*/tests/` |
| Two-implementation rule (ADR-0022) for `aether.physics.solver` | Partially satisfied by hash-stub plugins | Satisfied by 2 real implementations (Euler-Bernoulli beam FEM + analytical Poiseuille flow) + 2 protocol shims (openfoam-adapter, calculix-adapter) | **Satisfied by 3 real implementations + 1 protocol shim** — the calculix-adapter is now real (closed-form-checked LE1 cantilever via `ccx_2.23`), and the beam vs calculix cross-plugin consistency test asserts the two real implementations agree to within 5% | `reference_plugins/fea-reference-beam/tests/test_beam_conformance.py` (14); `reference_plugins/cfd-reference-minimal/tests/test_poiseuille.py` (10); `reference_plugins/calculix-adapter/tests/test_calculix_le1.py` (11); `reference_plugins/calculix-adapter/tests/test_cross_plugin_consistency.py` (3) |
| Composition engine correctness | Substrate only; one test (AGARD 445.6) with self-admitted limitation that it does not reproduce the published flutter speed | Substrate + a second, independently-checkable coupling case (1-DOF pitch-flutter closed form) | **Unchanged.** No composition-engine work in this session. | `cargo test -p aether_compose` (8 tests passing) |
| AGARD 445.6 numerical reproduction | Not done; the engine is the G4 Broyden-stub | Not done. The engine remains a G4 Broyden-stub; the substrate is honestly labelled. | **Not done** (unchanged). | `crates/aether_compose/src/lib.rs` self-description; `tests/agard_445_6_flutter.rs` docstring |
| MVR-04/05/12 cross-hardware | Test was titled "cross-hardware" but ran the same plugin twice on one host | Honest. The test now documents its single-host scope clearly; MVR-04/05/12 (real cross-architecture) remains a Phase 7 deliverable. | **Partially closed in 2026-09-04.** A new fitness check `rosetta_cross_arch` exercises the *equivalence check* on this machine (aarch64 native vs x86_64 Rosetta cas_hash) — identical. The strong MVR-04/05/12 claim (real second physical architecture) remains open as AP-31. | `python3 infra/fitness/run.py rosetta_cross_arch`; `crates/aether_exec/tests/mvr_04_05_12.rs` |
| Real OpenFOAM integration | `openfoam-adapter` was a byte-identical copy of `cfd-reference-minimal` (same `PLUGIN_ID`) | Thin shell-wrapper that delegates to `cfd-reference-minimal`; the wrapper has its own `PLUGIN_ID = org.aether.reference.fe.opencfd` | **Unchanged.** No OpenFOAM in this env; AP-11 remains open. | `reference_plugins/openfoam-adapter/python/stdio_plugin.py` |
| Real CalculiX integration | `calculix-adapter` was a UUID-stub (`execute()` returned a UUID-named bundle with no FEA solve) | Protocol shim (same surface; no real FEA) | **Real native-binary adapter** in 2026-09-04: writes a real `cantilever.inp` deck for NAFEMS LE1, shells out to `ccx_2.23` at `/opt/homebrew/bin/ccx_2.23`, parses the real `.frd` output, reports tip deflection against the closed-form `P L³/(3EI)`. Result: v_numerical=1.99121 m vs analytical 2.0 m, rel_err=4.4e-3 (0.44% FEM-vs-exact, 5% engineering tolerance). | `python3 -m pytest reference_plugins/calculix-adapter/tests/` (14/14); `python3 infra/fitness/run.py ccx_binary` (OK) |
| Real B-rep geometry | `occ-geometry` was a bounding-box fixture; docstring said "not real B-rep" | Still a bounding-box fixture, but the docstring is strengthened and a machine-checkable `is_real_brep: false` flag in `discover()` is enforced by a new `geom_fixture_label` fitness check | **Unchanged.** No pythonocc-core available; AP-13 remains open. | `infra/fitness/geom_fixture_label.py` |
| 10M-node graph scale (Phase 0 walking skeleton b) | x86_64 measurement from a prior session (insert 565 s, replay 6.4 s); not re-measured in this codebase | Same — the prior number was on the README, not re-measured | **Closed in 2026-09-04 (substrate scale test).** Fresh aarch64 measurement: insert 546.35 s, replay 6.30 s. Replay SDP budget (< 5 min) met with 47.6× headroom. Insert Phase 1 implied budget (< 5 min) NOT met (1.82× over); recorded honestly in AP-29. | `infra/release/scale_10m_2026-09-04.json`; `crates/aether_graph/README.md` |
| License broker (Phase 5) | Not present | Not present (out of scope for this session; calendar-bound) | **Not present** (unchanged). | — |
| Real rig-data twin ingestion (Phase 5) | Synthetic | Still synthetic (out of scope; data-bound) | **Still synthetic** (unchanged). | — |
| Real manufacturing genealogy (Phase 6) | Closed-loop workflow fixture; no real machine integration | Still a fixture (out of scope; hardware-bound) | **Still a fixture** (unchanged). | — |
| SI benchmark campaigns (Phase 8) | Not present | Not present (out of scope; calendar-bound) | **Not present** (unchanged). | — |
| Real HPC fleet exercise (Slurm/PBS/LSF) | `crates/aether_exec/src/backends/registry.rs` self-admits it produces synthetic scripts only | Still synthetic scripts only; the dark-build is single-host. The substrate (Backends implement the same `Backend` trait, contract surface parity) is in place. | **Unchanged.** | `crates/aether_exec/src/backends/registry.rs` |
| Fuzz harness for parser substrate | Not present | Not present | **Closed in 2026-09-04 (substrate scope).** A `hypothesis`-based `fuzz_contracts` harness exercises the JSON contract validation parser for 200 examples × 19 contracts (3,800 adversarial inputs) with zero unhandled exceptions. Honest label: this is the JSON contract validation parser; the AP242/CGNS format-parser fuzz the SDP envisions is AP-30 (G3+ deliverable). | `python3 infra/fitness/run.py fuzz_contracts` |

## What is real and substrate-complete (this is what "Phase 0–2 done" means)

The following pieces are genuinely real, and the project can claim
**substrate completeness for Phase 0–2** with this scope:

1. **Aether microkernel** (`crates/aether_kernel`, ~200 LOC) +
   graph engine (`aether_graph`, ~2.4k LOC) + workflow engine
   (`aether_workflow`, ~1.6k LOC) + provenance (`aether_provenance`,
   ~0.85k LOC) + policy (`aether_policy`, ~0.5k LOC) + registry
   (`aether_registry`, ~0.6k LOC) + exec service (`aether_exec`,
   ~1.85k LOC) + compose engine (`aether_compose`) + CLI
   (`aether_cli`, ~2.4k LOC). Total kernel + boundary ~12.4k LOC
   (well under the 50k LOC budget — `infra/fitness/loc_budget.py`
   reports 7.96k kernel LOC + 2.43k boundary LOC).

2. **Stdout/JSON-RPC 2.0 plugin runtime** (ADR-0015). The
   walking-skeleton test (`crates/aether_kernel/tests/walking_skeleton_a.rs`)
   spawns a Python plugin as a subprocess, exchanges
   `discover`/`capabilities`/`validate`/`execute` requests, and
   asserts the responses.

3. **Event-sourced graph with append-only log, hash-chained
   provenance, RBAC/CEL policy, TUF verification path** for
   offline plugin installation. The TUF author/verify round-trip
   is exercised by `crates/aether_cli/tests/walking_skeleton_c.rs`.

4. **19 contract packages** in `contracts/` (graph.model,
   plugin.manifest, physics.solver, compute.backend, io.format,
   geometry.kernel, geometry.recipe, mesh.engine, field.model,
   requirement, physics.composable, uq.method, si.governor,
   optimizer, twin.estimator, surrogate.builder, manufacturing.process,
   inspection.device, test.rig). 37 JSON schemas, all valid
   (`infra/fitness/schema_semver.py`).

5. **Three real reference physics implementations** of
   `aether.physics.solver`:
   - **`fea-reference-beam`** — Euler-Bernoulli beam FEM, 4
     closed-form-checkable cases (cantilever/SS × point/UDL load),
     relative error vs the analytical formula < 1e-6 with 8
     elements and < 1e-8 with 16 elements.
   - **`cfd-reference-minimal`** — analytical steady Poiseuille
     flow, sampling the parabolic velocity profile
     `u(y) = (G/(2 mu)) y (h - y)`. The `execute` method reports
     `verification.overall_status: pass` with `rel_err` of zero
     (the samples are exact).
   - **`calculix-adapter`** (added 2026-09-04) — a real
     native-binary adapter that drives the `ccx_2.23` binary on
     the NAFEMS LE1 cantilever case. The tip deflection is checked
     against the closed-form `P L³/(3EI)` to 0.44% relative
     error. The wider CalculiX surface (modal, nonlinear,
     plate/shell, 3D brick) is out of scope for this adapter and
     is declared as such in `discover()` and `capabilities()`.
   - One protocol shim (`openfoam-adapter` thin wrapper) remains
     the contract-surface half for the fluid side; tracks AP-11
     (real OpenFOAM).

6. **Composition engine** (`aether_compose`) with unit-consistency
   checks across participants, conservation-residual reporting,
   IQN-ILS (Broyden-style) acceleration, divergence abort. Two
   coupling test cases: AGARD 445.6 (substrate; does *not*
   reproduce the published flutter speed — the test docstring
   is honest about this) and the new 1-DOF pitch-flutter
   closed-form case (coupling machinery is verified; analytical
   flutter speed is recorded separately).

7. **Fitness pipeline** (`infra/fitness/`) — **six** checks
   (loc_budget, token_scan, schema_semver, sbom,
   geom_fixture_label, ccx_binary) all passing. The
   `geom_fixture_label` check is the machine-enforceable honesty
   sentinel for the geometry fixture; the `ccx_binary` check is
   the same for the new CalculiX adapter. Two further checks
   (`rosetta_cross_arch`, `fuzz_contracts`) are wired into the
   pipeline but recorded as partial / scope-honest in
   `gaps.md`.

8. **Dark-build CI runner** (`infra/ci/dark-build.sh`) — 24
   steps, runs end-to-end in ~46 s, exits 0. The previous
   self-recursion bug (step 22/22 invoked
   `g8-final-acceptance.py` which invoked the dark-build which
   invoked the gate — infinite loop) is fixed; the new step 22
   performs a self-check on the gate script's parse-validity
   and reports the gate verdict's freshness. Steps 21–24 are
   new in 2026-09-04: the new reference-plugin tests, the
   updated fitness pipeline, a release-hygiene step that
   machine-enforces no scratch artifacts in the tree, and the
   G8 self-check.

9. **G8 final integrated acceptance gate**
   (`infra/release/g8-final-acceptance.py`) — runs the dark-build,
   captures stdout+stderr, emits a machine-readable JSON verdict.
   The previous stale, foreign-machine artifact is gone; the
   current verdict is fresh, machine-local, and reports
   `passed: true` for the substrate check.

10. **Reference plugins with real algorithms**:
    - `uq-basic` — Sobol/LHS on Ishigami, ZDT1, DTLZ2
    - `optimizer-reference` — NSGA-II
    - `surrogates-core` — GP/POD
    - `twin-kf` — Kalman-family estimators
    These are unchanged in this session and remain genuine.

11. **24 ADRs** (`specs/adr/0000..0024`) covering §2 invariants
    and §4 technology decisions.

## What is *not* done (this is the honest gap)

The SDP spans 8 phases, ~136 weeks of work, and requires real
hardware, real data, real solvers, and a multi-person team. The
following items remain undone and are recorded explicitly in
`workflows/paper-run/gaps.md` (the existing G0 gaps log, now
extended with this alignment pass):

1. **Real OpenFOAM integration** (shell-wrapper only).
2. **Real CalculiX integration** (JSON-RPC shim only).
3. **Real B-rep geometry** (bounding-box fixture only; a real
   OCC binding is a G3 deliverable).
4. **AGARD 445.6 numerical reproduction** (engine is Broyden-stub;
   production-grade IQN-ILS is G4.5).
5. **Real Slurm/PBS/LSF fleet** (synthetic script fixtures only;
   no real cluster integration in this environment).
6. **License broker** (`aether.compute.license`, FlexLM/RLM) —
   Phase 5 deliverable.
7. **Real rig-data twin ingestion** (synthetic only) — Phase 5.
8. **Real manufacturing genealogy** (closed-loop workflow fixture
   only) — Phase 6.
9. **SI benchmark campaigns** (adaptive DOE vs classical, neural
   operator vs GP, symbolic discovery) — Phase 8.
10. **180-day offline soak** (MVR-08) — calendar-bound.
11. **MVR-04/05/12** (true cross-architecture) — requires a
    second architecture with a real QEMU/binfmt run; the test is
    honest about its single-host scope.

## What this session *did* close (the alignment pass of 2026-09-04)

The full changelog is at
`/Users/asseen/Documents/projects/aethers/aether-platform/ALIGNMENT_CHANGELOG_2026-09-04.md`
and the prior pass's changelog is at
`ALIGNMENT_CHANGELOG.md` (2026-09-03). In summary, this
session:

1. **Removed tracked scratch artifacts** (`infra/run/*.aether.db`,
   `infra/run/_mvr05/`) and extended `.gitignore` to catch
   future recurrences. New step 23/24 in `dark-build.sh`
   machine-enforces the absence in every future build.
2. **Replaced the `calculix-adapter` UUID stub with a real
   native-binary adapter** that drives `ccx_2.23` on the NAFEMS
   LE1 cantilever case. The adapter writes a real `.inp` deck,
   shells out to the real binary, parses the real `.frd`
   output, and reports the tip deflection against the closed-form
   `P L³/(3EI) = 2.0 m`. Result: v_numerical=1.99121 m, rel_err
   0.44%. The `ccx_binary` fitness check machine-enforces the
   production-track claim.
3. **Added a cross-plugin consistency test** that runs the same
   LE1 problem spec through both `fea-reference-beam` and
   `calculix-adapter` and asserts the two numerical tip
   deflections agree to within 5%. The two-implementation rule
   for `aether.physics.solver` is now satisfied by two real,
   mutually-checked implementations of the same physics.
4. **Ran the 10M-node graph scale test on this aarch64 box**
   (the existing `aether_graph_scale` binary, fresh on-machine
   measurement): insert 546.35 s, replay 6.30 s. Replay SDP
   budget (< 5 min) is met with 47.6× headroom. The insert
   Phase 1 implied budget is not met (1.82× over) and is
   recorded honestly as AP-29 in `gaps.md`. The artifact is
   at `infra/release/scale_10m_2026-09-04.json`; the
   `crates/aether_graph/README.md` is updated.
5. **Added a Rosetta cross-architecture fitness check** that
   runs the same `cfd-reference-minimal` Poiseuille problem
   spec under both `arch -x86_64` (Rosetta) and native aarch64
   and asserts identical cas_hash. The strong MVR-04 claim (real
   second physical architecture) remains open as AP-31.
6. **Added a `hypothesis`-based fuzz harness** for the JSON
   contract validation parser substrate: 200 examples × 19
   contracts (3,800 adversarial inputs) with zero unhandled
   exceptions. Honest label: this is the JSON contract
   validation parser; the AP242/CGNS format-parser fuzz the
   SDP envisions is AP-30 (G3+ deliverable).
7. **Dark-build is now 24 steps** with a machine-enforced
   hygiene check. All checks pass in 46.3 s.
8. **Test counts:** 107 Rust + 122 Python = 229 tests
   passing (was 215 pre-pass; +14 from CalculiX).

The 2026-09-03 pass's prior work (2 real reference physics
implementations, the `geom_fixture_label` honesty sentinel,
the 1-DOF pitch-flutter test, the dark-build self-recursion
fix, the 5-check fitness pipeline, the G8-gate freshness fix,
the gap log reorganization) is unchanged.

## Verification (the actual commands and their results)

Run on aarch64-apple-darwin, rustc 1.97.1, Python 3.14.7, on
2026-09-04. The working directory is the platform monorepo
root.

```bash
# Rust: 107 tests passing
$ export PATH=$HOME/.cargo/bin:$PATH
$ cargo test --workspace
# ... (full output)
test result: ok. 107 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

# Python: 122 tests passing (across all project test trees)
$ PYTHONPATH=python/aether_sdk:python/aether_testing:python/aether_geometry:python/aether_requirements \
    python3 -m pytest python/aether_testing/tests/ python/aether_geometry/tests/ \
                                python/aether_requirements/tests/ \
                                reference_plugins/fea-reference-beam/tests/ \
                                reference_plugins/cfd-reference-minimal/tests/ \
                                reference_plugins/calculix-adapter/tests/
============================= 122 passed in 1.15s ==============================

# Fitness pipeline: 6/6 checks passing (default)
$ python3 infra/fitness/run.py
[OK ] loc_budget: kernel LOC 7960 within budget (15.9%)
[OK ] token_scan: no prohibited tokens found
[OK ] schema_semver: 19 contract(s), 37 schema(s) — all valid
[OK ] sbom: SPDX 2.3 SBOM emitted: 251 rust + 6 python packages
       infra/release/sbom.spdx.json
[OK ] geom_fixture_label: geometry fixture honest: is_real_brep=False, note fragment 'not real B-rep' present
[OK ] ccx_binary: calculix-adapter is real: ccx=/opt/homebrew/bin/ccx_2.23; LE1 rel_err=4.395e-03 (tol 5e-2)
       solver_binary: /opt/homebrew/bin/ccx_2.23
       v_numerical: 1.99121 m
       v_analytical: 1.9999999999999998 m
6/6 checks passed.

# Dark-build: 24 steps, ALL CHECKS PASSED, 46.3 s
$ time bash infra/ci/dark-build.sh
[2026-09-04T01:47:51Z] [22/24] fitness pipeline (loc_budget, token_scan, schema_semver, sbom, geom_fixture_label, ccx_binary)
6/6 checks passed.
[2026-09-04T01:47:51Z] [23/24] release hygiene (no scratch artifacts in the distributed tree)
[2026-09-04T01:47:51Z] release hygiene: clean
[2026-09-04T01:47:51Z] [24/24] G8 final integrated acceptance gate (self-check)
[2026-09-04T01:47:51Z] G8 gate script parses: /Users/asseen/Documents/projects/aethers/aether-platform/infra/release/g8-final-acceptance.py
[2026-09-04T01:47:51Z] dark-build: ALL CHECKS PASSED
46.305 total

# G8 gate: fresh machine-local verdict
$ python3 infra/release/g8-final-acceptance.py
[g8-gate] exit code: 0
[g8-gate] duration: 46.1s
[g8-gate] log: /Users/asseen/Documents/projects/aethers/aether-platform/infra/release/g8-final-acceptance.log
[g8-gate] summary: /Users/asseen/Documents/projects/aethers/aether-platform/infra/release/g8-final-acceptance.json

# 10M-node graph scale test (this aarch64 box)
$ ./target/debug/aether_graph_scale --nodes 10000000 \
      --db /tmp/aether_scale_10m.db --budget-s 600
{
  "node_count": 10000000,
  "insert_seconds": 546.349579042,
  "replay_seconds": 6.300766792,
  "total_seconds": 552.655977125,
  "events_per_second_insert": 18303.299542272114,
  "nodes_replayed": 10000000,
  "distinct_types": 5,
  "db_size_bytes": 7255519232
}
```

The contents of `infra/release/g8-final-acceptance.json` after
this run are:

```json
{
  "gate": "g8-final-acceptance",
  "started_at": "2026-09-04T01:48:11Z",
  "duration_seconds": 46.10,
  "exit_code": 0,
  "passed": true,
  "phases": { "G0": [...], "G1": [...], ... },
  "log_path": "/Users/asseen/Documents/projects/aethers/aether-platform/infra/release/g8-final-acceptance.log"
}
```

This is the **only** G8 verdict the project can defend as of
this alignment pass. It is a *substrate* verdict: it asserts
that the dark-build runs end-to-end, the fitness pipeline is
green, the conformance suites pass, the reference plugins
speak the contract correctly, and a real native-binary FEA
adapter (CalculiX) produces a verified result against a
closed-form analytical solution. It is **not** a G8 *platform*
verdict: AGARD 445.6 is not numerically reproduced; real HPC
federation is not done; real manufacturing integration is not
done; SI campaigns are not done; the MVR-04 strong cross-
architecture claim is not made. Those are tracked in
`workflows/paper-run/gaps.md` and belong to Phases 4–8.

---

# Overhaul Wave 1 (2026-09-11) — golden path delivered

This section records the first overhaul wave (Phases 0–2 of the
master plan): reconnaissance, stabilization, and the user-facing
spine. Every claim below was executed on this machine
(aarch64-apple-darwin, rustc 1.97.1, **Python 3.9.6** — note the
environment drift from the 3.14.7 recorded above; `jsonschema`,
`pynacl`, and `tomli` are now pinned in `requirements-dev.txt`
accordingly).

## What was implemented

| Capability | Status | Verification |
|---|---|---|
| Project-local engineering state | **Done.** Standalone projects store their graph DB at `<project>/.aether/state/graph.db` (was: monorepo `infra/run/`); `AETHER_STATE_DIR` overrides. | `cargo test -p aether_cli --test project_state` (2 tests) |
| `aether doctor` | **Done.** Probes core runtime, graph, workflow, provenance, structural analysis (beam FEM vs closed form), uncertainty, SI governor, CalculiX (optional), OpenFOAM (optional). Exit 1 only if a *core* probe fails. | `cargo test -p aether_cli --test golden_path`; `aether doctor` output in `install.sh` run |
| `aether demo aircraft / structure / reproducibility` | **Done.** Deterministic, offline, embedded AWL workflows executing the real beam FEM + SI governor plugins; reproducibility demo compares artifact identity across two executions. | `cargo test -p aether_cli --test golden_path::demo_aircraft_runs_deterministically` |
| `aether init --profile {core,research,aerospace,developer}` | **Done.** Layered profile scaffolds (workflow + `experiment.yaml`) on the existing init; base `init` unchanged. | golden-path test `full_golden_path_end_to_end` |
| `aether run` | **Done.** Executes the project workflow, records SimulationRun → Result → Evidence into the project-local graph with input-snapshot hash, plugin identity, workflow hash, and determinism class. | golden-path test; verified via installed launcher from an arbitrary directory |
| `aether explain` | **Done.** Returns the latest engineering claim with value, units, analytical reference, relative error, fidelity class, validation status, and producing plugin identity. | golden-path test asserts `validation_status == "verified"` |
| `aether evidence export` | **Done.** Writes `evidence/evidence.json` + `manifest.json` (sha256) into the project; records carry the producing plugin's own classification (REFERENCE/SYNTHETIC/...); bundle includes the graph-chain verdict. | golden-path test asserts `graph_chain.valid == true` |
| `aether benchmark` | **Done (2 of 10).** AETHER-BENCH-01 (beam FEM mesh convergence vs closed form, rel_err ~1e-13) and AETHER-BENCH-08 (beam FEM vs real CalculiX, 0.44% agreement) — executed, not asserted. | `aether benchmark` output recorded below |
| `install.sh` | **Done.** Builds release CLI, installs a launcher pinning the platform home (`AETHER_MONOREPO`), runs doctor. | Full golden path re-verified through the installed launcher in `/tmp` |
| Golden-path regression in dark-build | **Done.** New step [20b/24] runs `cargo test -p aether_cli --test golden_path` (4 tests: doctor, demos, full init→run→explain→evidence, benchmarks). | `bash infra/ci/dark-build.sh` exits 0 |

## Substrate fixes found by exercising the golden path

Two real bugs surfaced only when the full path ran against float-bearing
data — both are substrate fixes, not spine-only patches:

1. **Event-chain float drift (integrity bug).** `serde_json`'s default
   float parsing (no `float_roundtrip` feature) can be off by 1 ULP, so
   an event payload re-read from SQLite hashed differently than at
   write time and `verify_chain` failed on any event carrying floats.
   Fix: enable `float_roundtrip` in the workspace `Cargo.toml`. Every
   existing test passed before this because prior tests only wrote
   string-valued properties.
2. **Executor-input envelope mismatch.** The workflow executor wraps
   step inputs as `{"step_id", "inputs"}`, but the beam, CFD, and
   CalculiX plugins read flat params — they were silently executing
   with empty inputs (and "succeeding"). Fixed in the plugins' JSON-RPC
   handle layer; executor-envelope round-trip verified end-to-end.
3. **CalculiX `.frd` wall-clock hashing (test flake).** The `.frd`
   format embeds `1UDATE`/`1UTIME`/`1UHOST` lines, so the adapter's
   content hash changed across a second boundary and the
   "bit-identical re-invocation" test was inherently flaky. Fix: hash
   normalized engineering content (environment metadata lines
   excluded); 5× repeated runs stable.

## Gate status after Wave 1 (measured 2026-09-11, this machine)

```bash
$ cargo test --workspace
# 113 passed; 0 failed  (+6: project_state ×2, golden_path ×4)

$ PYTHONPATH=python/aether_sdk:python/aether_testing:python/aether_geometry:python/aether_requirements \
    python3 -m pytest python/aether_testing/tests/ python/aether_geometry/tests/ \
    python/aether_requirements/tests/ reference_plugins/fea-reference-beam/tests/ \
    reference_plugins/cfd-reference-minimal/tests/ reference_plugins/calculix-adapter/tests/
# 122 passed

$ bash infra/ci/dark-build.sh
# dark-build: ALL CHECKS PASSED (24 steps incl. golden-path regression [20b])

$ aether benchmark
PASS AETHER-BENCH-01 — Structural reference analysis (173 ms)
      rel_err 2.2e-13 @ 8 elements, 1.0e-12 @ 16 elements (closed-form Euler-Bernoulli)
PASS AETHER-BENCH-08 — Cross-implementation reproducibility (257 ms)
      beam_fem 2.0000 m vs calculix 1.99121 m → 0.44% relative difference (tol 5%)
```

Total: **235 tests passing** (113 Rust + 122 Python). The strongest
defensible status remains *Phase 0–2 substrate + golden path*; the
scientific loop (adaptive fidelity escalation, experiment engine,
digital twin, BENCH-02..07/09/10) is Wave 2+ and is tracked in
`workflows/paper-run/gaps.md`.

---

# Overhaul Wave 2 — the scientific loop (2026-09-11)

Wave 2 implements overhaul phases 3–8: the aerostructural core, the
adaptive-fidelity escalation loop, surrogate-assisted optimization with
real-model verification, the executable experiment engine, the
defect-aware twin, and five new benchmarks. Every number below is from
actual execution on this machine; classifications (REFERENCE /
SYNTHETIC) are carried on the artifacts themselves.

## What was implemented (phases 3–8)

| Phase | Capability | Entry point | Honesty classification |
|---|---|---|---|
| 3 | Wing-box sizing reference plugin (boom idealization: stress, shear buckling, deflection, mass, reserve margins) + batch mode | `workflows/wing-sizing.awl.yaml` → `aether run` | REFERENCE (closed-form, assumptions declared in-plugin) |
| 3 | Executor plugin routing: handlers keyed `contract::plugin`, one workflow may use multiple physics plugins | executor core | — |
| 3 | Sizing chain recorded in the project graph: LoadCase → SimulationRun → Result → Evidence | `aether run` | — |
| 4 | `FidelityEscalationGovernor`: deterministic escalate/stay decision from uncertainty, margin proximity, decision sensitivity; every decision logged as a graph Decision node | `aether run`, `aether benchmark AETHER-BENCH-07` | REFERENCE decision logic (illustrative cost table) |
| 5 | `aether optimize`: DOE (24 pts) → GP (real training) → NSGA-III on surrogate → **verification re-run of the real model at the optimum** with trust-envelope screening | `aether optimize` | surrogate values labeled; optimum is the real-model value |
| 6 | `aether experiment run`: baseline vs treatment workflows executed, metrics measured, verdict decided by declared gates, recorded as Evidence | `aether experiment <spec.yaml>` | verdict from measurement, never assertion |
| 7 | `aether demo twin`: synthetic manufacturing deviation → measurement sweep → KF state update → corrected prediction → adopt/keep Decision | `aether demo twin` | SYNTHETIC data (labeled on the record); model executions real |
| 8 | Benchmarks 04/05/06/07/09 (below) | `aether benchmark` | gates declared in the artifact |

## Demonstrated hypothesis results (measured, this machine)

- **H1 (adaptive fidelity / experiment engine):** two-arm experiment
  `h1-adaptive-mesh-coarsening` — 64-element vs 2-element beam FEM, both
  closed-form-verified: measured speedup **7.1×** at quality delta
  **5.1e-10** relative error → verdict *supported* by the declared gates.
- **H5 (defect-aware twin):** 13.9% synthetic cap-area deficit;
  EKF over 4 noisy deflection-growth measurements (seed 42); nominal
  prediction error **13.9%** → updated error **5.2e-5**
  (improvement ×2693). Full chain PhysicsModel → InspectionRecord →
  MeasurementDataset → Model → Evidence recorded with a verified graph
  event chain.
- **H2 (state coherence):** sizing `run` records LoadCase /
  SimulationRun / Result / Evidence with catalog-conformant edges;
  `evidence export` reports `"valid": true` on the event chain with
  13 records.
- **H4 (evidence-driven engineering):** every Result carries
  `fidelity_class` (enum-enforced) and `validation_status`; every claim
  carries producer classification and plugin identity.

## Benchmark suite status (all from real execution)

```
$ aether benchmark
PASS AETHER-BENCH-01 — Structural reference analysis (beam FEM vs closed form) (107 ms)
PASS AETHER-BENCH-04 — Structural optimization (verified surrogate loop) (202 ms)
      verified optimum 6351.1 kg; all margins >= 0.05; surrogate pred err 0.0%
PASS AETHER-BENCH-05 — Surrogate substitution (GP vs real model) (190 ms)
      GP train RMSE 363.5 kg on 24 DOE points; optimum pred err <= 2% gate
PASS AETHER-BENCH-06 — Uncertainty propagation (Sobol vs analytical) (116 ms)
      S1/ST vs analytical Ishigami values within estimator noise (n=1024, seed 42)
PASS AETHER-BENCH-07 — Adaptive fidelity (escalation decisions) (130 ms)
      4/4 governor cases: stay / escalate on proximity / escalate on U+sens / clamp
PASS AETHER-BENCH-08 — Cross-implementation reproducibility (beam vs CalculiX) (197 ms)
PASS AETHER-BENCH-09 — Manufacturing deviation / twin update (84 ms)
      nominal 13.9% -> updated 5.2e-5 relative error (SYNTHETIC data class)
```

**BENCH-06 caught a real bug**: the LHS stratum assignment was
rank-identical across columns, so the Saltelli A/B matrices were
nearly identical and every Sobol index collapsed to ~0. Fixed
(per-column independent stratified permutation); the analytical
reference values now reproduce within estimator noise. This is the
benchmark suite doing its job.

## Gate status after Wave 2 (measured 2026-09-11)

```bash
$ cargo test --workspace
# 113 passed; 0 failed

$ PYTHONPATH=python/aether_sdk:python/aether_testing:python/aether_geometry:python/aether_requirements \
    python3 -m pytest reference_plugins/ python/ -q
# 160 passed

$ bash infra/ci/dark-build.sh
# dark-build: ALL CHECKS PASSED (24 steps incl. golden-path regression [20b])
```

Total: **273 tests passing** (113 Rust + 160 Python). Golden path
verified end-to-end after every phase; four demos now runnable:
`aether demo aircraft | structure | twin | reproducibility`.

## What is still open (honest gaps after Wave 2)

- BENCH-02/03 (aerodynamic reference, aerostructural coupling) and
  BENCH-10 (closed-loop design→test→update) remain unimplemented.
- The experiment engine runs single-physics arms; multi-fidelity
  treatment arms (governor-driven escalation inside one arm) are
  scaffolding only.
- The twin demonstrator estimates one scalar growth factor; a
  vector-state twin (stiffness distribution over span stations) is the
  natural Wave 3 step.
- The escalation governor's cost table is illustrative (REFERENCE),
  not calibrated to this machine's measured costs.

---

# Overhaul completion — Wave 3 (2026-09-11): the full benchmark suite

Wave 3 closes the remaining gaps from the final definition of done:
the aerodynamic reference, the aerostructural coupling, the closed-loop
benchmark, and the adaptive-fidelity loop in `aether run`.

## What was implemented

- **`aero-lifting-line` reference plugin** (ADR-0022 aero side): Glauert
  Fourier collocation of the monoplane equation, stdlib-only, odd
  harmonics, closed-form Gaussian elimination. The elliptical planform
  is a fixed point of the discretization — CL reproduces the classical
  closed form at machine precision and all higher Fourier coefficients
  vanish (~1e-20). Two independent verification checks run in-plugin:
  CL vs closed form (tolerance-declared) and the span-efficiency
  identity CDi/CL² = 1/(π AR) at 1e-9. 26 conformance tests.
- **BENCH-02**: lifting line vs elliptical closed form across term
  counts; every case passes at machine precision.
- **BENCH-03**: an explicit aerostructural fixed-point loop — the aero
  plugin converts alpha to lift (L = CL·q·S), the sizing plugin
  converts lift to deflection, and the deflection reduces the effective
  alpha through a declared twist effectiveness. Converges in 5
  iterations; gates on the physical signature: elastic lift relief
  **1.76%** (coupled CL 0.4309 vs rigid 0.4386).
- **BENCH-10**: the closed loop design → test → update: verified
  optimum → synthetic as-built cap-area deviation (−8%) → EKF over four
  seeded deflection-growth measurements → updated prediction verified
  against the as-built truth. Measured: growth estimated 1.0862 vs
  1.0870 truth; design-stage error 8% reduced to **7.4e-4**.
- **Closed fidelity loop in `aether run`**: when the escalation
  governor decides to escalate after a sizing step (from the computed
  margin state), `run` re-executes the quantity of interest at the
  escalated ladder rung (beam FEM as the higher-fidelity model of the
  sizing deflection), records the escalated SimulationRun (marked
  `fidelity_escalation_of`), and stores the measured model-form
  difference between the two levels. Demonstrated: sizing margin −0.83
  → governor escalate → beam-FEM re-run (rel err 9.6e-12 vs its own
  closed form) → model-form difference recorded.

## Benchmark suite: complete

```
$ aether benchmark          # 10/10 PASS, all from real execution
01  Structural reference analysis (beam FEM vs closed form)      rel_err ~1e-13
02  Aerodynamic reference (lifting line vs elliptical form)      rel_err ~1e-16
03  Aerostructural coupling (elastic lift relief)                1.76%, 5 iters
04  Structural optimization (verified surrogate loop)            margins hold
05  Surrogate substitution (GP vs real model)                    gate <= 2%
06  Uncertainty propagation (Sobol vs analytical)                within noise
07  Adaptive fidelity (escalation decisions)                     4/4 cases
08  Cross-implementation reproducibility (beam vs CalculiX)      0.44% (tol 5%)
09  Manufacturing deviation / twin update                        ×2693 improvement
10  Closed loop: design -> test -> update                        8% -> 7.4e-4
```

## Gate status after Wave 3 (measured 2026-09-11)

```bash
$ cargo test --workspace                 # 113 passed; 0 failed
$ python3 -m pytest reference_plugins/ python/ -q   # 186 passed
$ bash infra/ci/dark-build.sh            # ALL CHECKS PASSED (24 steps)
$ aether benchmark                       # 10/10 PASS
```

Total: **299 tests passing** (113 Rust + 186 Python). Full surface:
`doctor / demo {aircraft,structure,twin,reproducibility} / init --profile /
run / explain / evidence export / optimize / experiment / benchmark` —
all verified from a clean directory in a single session.

## Remaining honest limitations

- The aero plugin covers elliptical/rectangular planforms under the
  lifting-line validity regime; swept/compressible/stalled regimes are
  explicitly out of scope (stated in the plugin's validity block).
- The BENCH-03 coupling uses a declared twist-effectiveness reduction
  rather than a computed beam twist distribution — the loop structure
  is real, the structural-to-aero transfer is a declared coefficient.
- Twin state remains scalar (spanwise distribution is Wave 4 material).
  *(Closed by Wave 4 — see the next section.)*
- The governor cost table remains illustrative (REFERENCE).

---

# Overhaul Wave 4 — the spanwise (vector-state) twin (2026-09-12)

Wave 4 closes the twin limitation named at the end of Wave 3: the
defect-aware twin estimated a single scalar growth factor. The span
distribution of stiffness (one EI per station) is the state a real
structural twin needs, and it is now estimated for real.

## What was implemented

| Capability | Entry point | Honesty classification |
|---|---|---|
| `ekf_vector`: multivariate linear Kalman filter in the `twin-kf` reference plugin (random-walk dynamics, n×n observation matrix, full covariance algebra, stdlib-only dense implementation, loud dimension validation) | `aether.twin.estimator` → `estimate` (shape-routed: list prior mean → vector path) | REFERENCE (exact for the linear-Gaussian case; verified against closed-form algebra) |
| `twin_spanwise_loop` in the CLI spine: 8-station EI-distribution estimation over the real beam FEM | `twin_spanwise_loop`, shared by the demo and the benchmark | deviation SYNTHETIC (labeled on every record); all model executions real |
| `aether demo twin-span` (fifth demo) | `aether demo twin-span` | verdict from measurement, never assertion |
| `AETHER-BENCH-11`: spanwise twin update quality, gated on per-station recovery, superiority over the best scalar twin, drift detection, and a valid graph chain | `aether benchmark AETHER-BENCH-11` | gates declared in the artifact |

## The inverse model is measured, not asserted

The loop's observation matrix **H is computed from the real FEM**:
for each element i, one extra beam-plugin execution at `ei_scales[i] +=
0.02` gives the finite-difference sensitivity of the whole deflection
profile to that element's stiffness (d growth_j / d s_i, evaluated at
nominal; linear model ⇒ exact columns up to float noise). The state is
the zero-centered EI-scale deviation z = s − 1 with a
tolerance-scale prior (σ = 1% per station), so a −10% root-region
deficit is a multi-sigma anomaly that drives both the drift flag and
the update. The measured profile (SYNTHETIC, seeded per-station noise,
σ = 0.4%) is inverted in one batch vector observation. The updated
model is then **re-executed on the real beam FEM** — the reported
correction quality never comes from the linearization itself.

The comparator is the strongest honest baseline: the best any *scalar*
twin could do, found by bisection through the real model on tip-deflection
growth (a scalar twin can match one observable but cannot localize a
spanwise defect). The vector state beats it on every station.

## Measured result (this machine, deterministic across runs)

```
$ aether demo twin-span
  station  EI_scale_truth  EI_scale_estimated
        0          1.0000              0.9875
        1          0.9000              0.9236
        2          0.9200              0.9577
        3          1.0000              0.9819
        4          1.0000              0.9924
        5          1.0000              0.9967
        6          1.0000              0.9990
        7          1.0000              0.9999

  max per-station err : 1.27e-2 (vector state) vs 4.30e-2 (uniform-scale best fit)
  drift detected      : true
Demo PASSED (gates: every station within 5% of truth; vector error < scalar
error on every station; drift fired; graph chain valid)
```

The defect is **localized**: stations 1–2 are recovered to within
2.4/3.8% while the healthy stations 3–7 correctly return to ~1.0. The
max per-station prediction error is **3.4× smaller** than the best
uniform-scale fit could achieve. Two consecutive `--json` runs produce
byte-identical output (sha256-verified).

## Estimator verification (twin-kf `test_vector_kf.py`, 12 tests)

Every expected value is computed from closed-form Kalman algebra or an
exact identity, not from the implementation: scalar reduction (n=1, H=1
equals the scalar KF update exactly), repeated-scalar averaging
(independent equal-noise sensors average exactly), zero-H-row unobserved
station (posterior stays exactly at the prior; malformed H raises —
I-12), per-station precision-weighted closed form with uncorrelated
posterior, correlated-noise exploitation (shared sensor noise makes the
station-difference channel exactly known — the identity that justifies
carrying full covariance matrices), drift on >3σ outliers and quiet on
well-behaved noise, bit-identical determinism, scalar-path envelope
unchanged, shape-based routing, and the stdio JSON-RPC round-trip in
the executor envelope. Found and fixed during this suite: the vector
path validates matrix dimensions loudly instead of silently producing
a wrong posterior.

## Gate status after Wave 4 (measured 2026-09-12)

```bash
$ cargo test --workspace                 # 114 passed; 0 failed
$ PYTHONPATH=python/aether_sdk:python/aether_testing:python/aether_geometry:python/aether_requirements \
    python3 -m pytest reference_plugins/ python/ -q   # 201 passed
$ bash infra/ci/dark-build.sh            # ALL CHECKS PASSED (24 steps)
$ aether benchmark                       # 11/11 PASS (incl. new BENCH-11)
```

Total: **315 tests passing** (114 Rust + 201 Python). Demo surface is
now five kinds: `aether demo aircraft | structure | twin | twin-span |
reproducibility`.

## Remaining honest limitations (Wave 4)

- The inversion is linearized (H evaluated at nominal). The measured
  1.3% recovery already includes the real model's nonlinearity at this
  defect size, but no formal linearity-error bound is computed (AP-33).
- One measurement channel: the deflection-growth profile. Slope/
  curvature channels and an estimated (rather than declared) measurement
  covariance R are future work.
- `twin-kf` remains the only `aether.twin.estimator` implementation; the
  contract's two-implementation rule is unchanged (as in prior waves).
- The scalar defect twin (BENCH-09/10) is unchanged and independent;
  both loops exercise the same estimator contract at different state
  dimensions.
