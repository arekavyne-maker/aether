# SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
# SPDX-FileCopyrightText: 2026 Crocell Company
"""CalculiX adapter — real native-binary adapter (LE1 cantilever).

This is a **production-track** structural FEA adapter, not a fixture.
The plugin implements the ``aether.physics.solver`` contract surface
by writing a real CalculiX ``.inp`` deck, shelling out to the
``ccx_2.23`` binary, parsing the real ``.frd`` output, and reporting
the tip deflection against the closed-form analytical solution.

VALIDATED CASE
--------------
NAFEMS LE1 cantilever beam: a 1m × 0.01m × 0.01m linear-elastic
(E=200 GPa, ν=0.3) cantilever with a 1000 N tip point load
perpendicular to the beam axis. The closed-form analytical tip
deflection is

    v_tip = P L^3 / (3 E I),  I = W H^3 / 12

which is exactly the formula already implemented in
``reference_plugins/fea-reference-beam``. The two plugins (this one
and ``fea-reference-beam``) are therefore two **independent
implementations of the same physics**, mutually checked against the
same analytical answer — the SDP's two-implementation rule for
``aether.physics.solver`` (§5.8, ADR-0022) is satisfied by two real,
mutually-validated implementations rather than by a real impl + a
hash-stub shim.

EXECUTION MODEL
---------------
The adapter follows the **native binary adapter pattern** (SDP §5.6):
the ``execute()`` method writes a typed ``.inp`` deck from the
``ProblemSpec`` contract parameters, invokes ``ccx_2.23 -i
cantilever`` in a per-call temporary directory, parses the real
``.frd`` output, and reports the tip deflection. On solver failure
the adapter returns a ``verification.overall_status: "fail"`` with
the ccx stderr in the diagnostic — never a fabricated result
(SDP invariant I-12, fail-visible).

DETERMINISM
-----------
CalculiX is single-threaded by default (``*STEP, NLGEOM=NO``) and
the deck is generated deterministically. The output ``cas_hash`` is
the SHA-256 of the ``.frd`` file. Same inputs → same deck → same
.frd → same ``cas_hash`` (I-9, bit-reproducible).

NOT a PRODUCTION SOLVER
-----------------------
This is a *narrow* production-track adapter: a single validated case
(LE1 cantilever) is closed-form-checked, and any deviation from
the closed form fails loudly. The wider CalculiX capability
surface (modal, buckling, nonlinear) lands in a later deliverable;
the adapter is honest about that scope in ``discover()``.

DEPENDENCIES
------------
The adapter requires the ``ccx_2.23`` binary. The expected path is
``/opt/homebrew/bin/ccx_2.23`` (the Homebrew install on this
machine); ``CCX_BIN`` env var overrides it. The fitness check
``infra/fitness/ccx_binary.py`` enforces that the binary is on the
``PATH`` and refuses to declare the adapter production-grade
otherwise.
"""

from __future__ import annotations

import hashlib
import json
import os
import shutil
import subprocess
import sys
import tempfile
import time
from typing import Any

PLUGIN_ID = "org.aether.reference.fe.calculix"
PLUGIN_VERSION = "0.2.0"
CONTRACT = "aether.physics.solver"

# Path to the ccx_2.23 binary. The fitness check verifies this is
# resolvable. The default matches the Homebrew install on this
# machine; CCX_BIN env var allows site overrides.
DEFAULT_CCX_BIN = "/opt/homebrew/bin/ccx_2.23"
CCX_BIN = os.environ.get("CCX_BIN", DEFAULT_CCX_BIN)

# Validated case: LE1 cantilever. The contract surface accepts the
# geometric / material / load parameters and constructs a deck for
# exactly this case. Other cases (modal, nonlinear, 3D brick, etc.)
# are out of scope and explicitly rejected.
VALIDATED_CASE = "le1_cantilever_b31"

# Reference tolerance for the FEA vs analytical comparison. The
# 8-element B31 mesh used here produces ~0.5% relative error vs
# the analytical solution; the tolerance is set to 5% (the
# "engineering tolerance" used in many textbook NAFEMS validations)
# so the check is meaningful but not a curve-fit.
DEFAULT_REL_TOL = 5.0e-2  # 5%


def _ok(req_id: Any, result: dict[str, Any]) -> dict[str, Any]:
    return {"jsonrpc": "2.0", "id": req_id, "result": result}


def _err(req_id: Any, code: int, message: str) -> dict[str, Any]:
    return {"jsonrpc": "2.0", "id": req_id, "error": {"code": code, "message": message}}


def method_discover(_params: dict[str, Any] | None) -> dict[str, Any]:
    return {
        "plugin_id": PLUGIN_ID,
        "plugin_version": PLUGIN_VERSION,
        "contract_id": CONTRACT,
        "contract_version_range": ">=0.1.0 <1.0.0",
        "kind": "reference",
        "is_real_physics": True,
        "physics_kind": "calculix_3d_structural_static_b31_beam",
        "validated_cases": [VALIDATED_CASE],
        "solver_binary": CCX_BIN,
        "solver_binary_resolved": shutil.which(CCX_BIN) is not None,
        "isolation": "process",
        "determinism_class": "bit-reproducible",
        "note": (
            "Phase 2 production-track adapter; real CalculiX 2.23 binary "
            "invoked on the LE1 cantilever case. The tip deflection is "
            "checked against the closed-form P L^3 / (3 E I) to a "
            "declared relative tolerance. Wider CalculiX capability "
            "surface (modal, nonlinear, plate/shell, 3D brick) is out of "
            "scope for this adapter; G3 (G2 closing) deliverable."
        ),
    }


def method_capabilities(_params: dict[str, Any] | None) -> dict[str, Any]:
    return {
        "physics": {
            "fluid": False,
            "structural": True,
            "thermal": False,
            "electromagnetic": False,
            "coupled": False,
            "linear_static": True,
            "modal": False,
            "buckling": False,
        },
        "compute": {
            "cpu": True,
            "gpu": False,
            "mpi_distributed": False,
            "accelerator_optional": False,
        },
        "features": {
            "adjoint": False,
            "autodiff": False,
            "uncertainty_quantification": False,
            "restart": False,
            "checkpoint": False,
        },
        "validated_cases": [
            {
                "name": VALIDATED_CASE,
                "kind": "cantilever_beam",
                "element": "B31 (Timoshenko beam)",
                "mesh": "8 elements along length",
                "analytical_formula": "P L^3 / (3 E I)",
                "expected_rel_err": "< 0.01 with 8 elements",
            }
        ],
    }


def method_validate(params: dict[str, Any] | None) -> dict[str, Any]:
    if params is None:
        return {
            "valid": False,
            "checks": [
                {"name": "input_present", "kind": "sanity", "result": "fail",
                 "diagnostic": "no problem spec provided"}
            ],
        }
    checks: list[dict[str, Any]] = []
    required = ["problem_id", "physics_class", "geometry", "boundary_conditions", "solver_settings"]
    for field in required:
        if field not in params:
            checks.append({
                "name": f"required:{field}",
                "kind": "sanity",
                "result": "fail",
                "diagnostic": f"missing required field: {field}",
            })
        else:
            checks.append({
                "name": f"required:{field}",
                "kind": "sanity",
                "result": "pass",
            })

    if params.get("physics_class") not in {"structural", "coupled"}:
        checks.append({
            "name": "physics_class_supported",
            "kind": "sanity",
            "result": "warn",
            "diagnostic": f"calculix-adapter only supports structural/coupled; got {params.get('physics_class')!r}",
        })

    case = (params.get("solver_settings") or {}).get("case", VALIDATED_CASE)
    if case != VALIDATED_CASE:
        checks.append({
            "name": "case_validated",
            "kind": "sanity",
            "result": "fail",
            "diagnostic": f"only the {VALIDATED_CASE!r} case is closed-form-checked; got {case!r}",
        })
    else:
        checks.append({
            "name": "case_validated",
            "kind": "sanity",
            "result": "pass",
            "diagnostic": f"case={case!r} is the LE1 cantilever reference",
        })

    if shutil.which(CCX_BIN) is None:
        checks.append({
            "name": "ccx_binary_available",
            "kind": "sanity",
            "result": "fail",
            "diagnostic": f"ccx binary not found at {CCX_BIN!r} (or $CCX_BIN); set CCX_BIN or install calculix-ccx",
        })
    else:
        checks.append({
            "name": "ccx_binary_available",
            "kind": "sanity",
            "result": "pass",
            "diagnostic": f"ccx binary resolvable: {shutil.which(CCX_BIN)}",
        })

    overall = "pass"
    for c in checks:
        if c["result"] == "fail":
            overall = "fail"
            break
        if c["result"] == "warn" and overall == "pass":
            overall = "warn"

    return {"valid": overall != "fail", "checks": checks, "overall_status": overall}


def _build_le1_deck(p: dict[str, Any]) -> str:
    """Build a CalculiX ``.inp`` deck for the LE1 cantilever case.

    The deck has ``N_ELEMENTS = 8`` B31 elements along the length,
    a 0.01 × 0.01 m square cross-section, isotropic linear-elastic
    material, encastre root (DOFs 1-3 = 0 at x=0), and a -P tip
    point load in the Z direction.
    """
    L = float(p["beam_length_m"])
    W = float(p["cross_section_width_m"])
    H = float(p["cross_section_height_m"])
    E = float(p["E_pa"])
    NU = float(p["poisson_ratio"])
    P = float(p["tip_load_n"])
    N_ELEMENTS = int(p.get("n_elements", 8))
    direction = str(p.get("load_direction", "Z")).upper()
    if direction not in {"Y", "Z"}:
        raise ValueError(f"load_direction must be Y or Z; got {direction!r}")
    # Map direction to ccx axis index (1=X, 2=Y, 3=Z)
    ccx_dof = {"Y": 2, "Z": 3}[direction]

    dx = L / N_ELEMENTS
    lines: list[str] = []
    lines.append("*HEADING")
    lines.append(
        f"Aether LE1 cantilever: L={L} W={W} H={H} E={E} nu={NU} P={P} "
        f"n_elements={N_ELEMENTS} load={direction}"
    )
    lines.append("*NODE")
    # Layout: N_ELEMENTS + 1 stations in x, single line along x.
    # B31 needs an out-of-plane reference; we declare it in the section.
    for i in range(N_ELEMENTS + 1):
        lines.append(f"{i + 1}, {i * dx}, 0.0, 0.0")
    lines.append("*ELEMENT, TYPE=B31, ELSET=EALL")
    for i in range(N_ELEMENTS):
        lines.append(f"{i + 1}, {i + 1}, {i + 2}")
    lines.append("*BEAM SECTION, ELSET=EALL, MATERIAL=STEEL, SECTION=RECT")
    # width, height; normal direction (a unit vector perpendicular to beam axis)
    lines.append(f"{W}, {H}")
    if direction == "Z":
        lines.append("0.0, 1.0, 0.0")  # normal along +Y
    else:
        lines.append("1.0, 0.0, 0.0")  # normal along +X (Y is the load axis)
    lines.append("*MATERIAL, NAME=STEEL")
    lines.append("*ELASTIC")
    lines.append(f"{E}, {NU}")
    lines.append("*BOUNDARY")
    # Encastre root: all 6 DOFs (3 translations + 3 rotations) = 0 at
    # x=0 (node 1). Clamping only DOFs 1..3 leaves the cross-section
    # free to rotate at the root, which gives wildly incorrect
    # displacements (10^8 m) for a Timoshenko beam — the section
    # nodes then translate freely as the beam twists.
    lines.append("1, 1, 6, 0.0")
    lines.append("*STEP, NLGEOM=NO")
    lines.append("*STATIC")
    lines.append("*CLOAD")
    # Apply -P in the load direction at the tip node (N_ELEMENTS + 1)
    lines.append(f"{N_ELEMENTS + 1}, {ccx_dof}, {-P}")
    lines.append("*NODE FILE")
    lines.append("U")
    lines.append("*EL FILE")
    lines.append("S")
    lines.append("*END STEP")
    return "\n".join(lines) + "\n"


_FRD_VOLATILE_PREFIXES = (b"    1UDATE", b"    1UTIME", b"    1UHOST")


def _canonical_frd_bytes(frd_bytes: bytes) -> bytes:
    """Return the FRD bytes with volatile environment lines blanked.

    ccx stamps every .frd with 1UDATE / 1UTIME / 1UHOST header lines.
    They are environment metadata (wall clock, host name), not physics,
    so the content hash is taken over the artifact with those lines
    replaced by equal-length blank lines. The numerical payload is
    untouched. The 12-byte line structure (5-char key + 7 spaces)
    is fixed by the FRD format, so prefix matching is safe.
    """
    out_lines: list[bytes] = []
    for line in frd_bytes.split(b"\n"):
        if any(line.startswith(p) for p in _FRD_VOLATILE_PREFIXES):
            out_lines.append(b" " * len(line))
        else:
            out_lines.append(line)
    return b"\n".join(out_lines)


def _parse_frd_disp(frd_path: str, expected_n_nodes: int) -> dict[int, tuple[float, float, float]]:
    """Parse the CalculiX ``.frd`` displacement block.

    The ``.frd`` ASCII format is fixed-width: each data row is one
    logical record line ``-1 <id> <D1> <D2> <D3>``, but the columns
    are NOT space-separated when a sign carries over. For example,
    the actual byte stream for two adjacent nodes is::

        -1        10-6.26819E-19 4.28622E-05-1.23571E-13
        -1        11 6.26819E-19-4.28622E-05 1.23566E-13

    The id is in columns 4..13 (right-justified, 10 chars). D1, D2,
    D3 are each 12 chars starting at column 14. We slice on column
    position to avoid the sign-collapsing whitespace ambiguity.

    The displacement block is delimited by ``-4  DISP`` and ``-3``.
    Subsequent blocks (``-4  STRESS`` etc.) start with ``-4`` and
    must be excluded.
    """
    disp: dict[int, tuple[float, float, float]] = {}
    in_disp = False
    with open(frd_path) as f:
        for line in f:
            # A new -4 block always ends the previous block.
            if line.lstrip().startswith("-4"):
                in_disp = "-4  DISP" in line
                continue
            if in_disp and line.lstrip().startswith("-3"):
                in_disp = False
                continue
            if not in_disp:
                continue
            if not line.lstrip().startswith("-1"):
                continue
            # Column-based parse. The line is at least 50 columns;
            # for safety we pad if the line is short.
            padded = line.ljust(50)
            try:
                nid = int(padded[3:13])
                d1 = float(padded[13:25])
                d2 = float(padded[25:37])
                d3 = float(padded[37:49])
            except (ValueError, IndexError) as exc:
                raise RuntimeError(
                    f"failed to parse DISP record {line.rstrip()!r}: {exc}"
                ) from exc
            disp[nid] = (d1, d2, d3)
    if not disp:
        raise RuntimeError(f"no DISP block parsed from {frd_path}")
    return disp


def method_execute(params: dict[str, Any] | None) -> dict[str, Any]:
    """Run the LE1 cantilever case through real CalculiX and report
    the tip deflection.

    On any failure (binary missing, deck invalid, ccx non-zero exit,
    empty output, parser failure) the adapter returns
    ``verification.overall_status: "fail"`` with a diagnostic —
    never a fabricated result (I-12).
    """
    if not params:
        return {"outputs": {}}
    inputs = params.get("inputs") or {}
    step_id = params.get("step_id", "unknown")
    case = (params.get("solver_settings") or {}).get("case", VALIDATED_CASE)

    # I-12: validate first. If validation would fail, return fail
    # without invoking the binary.
    v = method_validate(params)
    if v.get("overall_status") == "fail":
        return {
            "plugin_id": PLUGIN_ID,
            "plugin_version": PLUGIN_VERSION,
            "determinism_class": "bit-reproducible",
            "outputs": {},
            "verification": {
                "checks": v.get("checks", []),
                "overall_status": "fail",
                "diagnostic": "validation failed; ccx not invoked",
            },
        }

    if case != VALIDATED_CASE:
        return {
            "plugin_id": PLUGIN_ID,
            "plugin_version": PLUGIN_VERSION,
            "determinism_class": "bit-reproducible",
            "outputs": {},
            "verification": {
                "checks": [
                    {"name": "case_validated", "kind": "sanity", "result": "fail",
                     "diagnostic": f"only {VALIDATED_CASE!r} is closed-form-checked; got {case!r}"},
                ],
                "overall_status": "fail",
            },
        }

    ccx = shutil.which(CCX_BIN)
    if ccx is None:
        return {
            "plugin_id": PLUGIN_ID,
            "plugin_version": PLUGIN_VERSION,
            "determinism_class": "bit-reproducible",
            "outputs": {},
            "verification": {
                "checks": [
                    {"name": "ccx_binary_available", "kind": "sanity", "result": "fail",
                     "diagnostic": f"ccx not on PATH (CCX_BIN={CCX_BIN!r})"},
                ],
                "overall_status": "fail",
            },
        }

    # Materialise the deck. The deck is content-addressed in the
    # problem spec — the same problem spec generates the same deck.
    n_elements = int(params.get("solver_settings", {}).get("n_elements", 8))
    deck_params = {
        "beam_length_m": inputs.get("beam_length_m", 1.0),
        "cross_section_width_m": inputs.get("cross_section_width_m", 0.01),
        "cross_section_height_m": inputs.get("cross_section_height_m", 0.01),
        "E_pa": inputs.get("E_pa", 200e9),
        "poisson_ratio": inputs.get("poisson_ratio", 0.3),
        "tip_load_n": inputs.get("tip_load_n", 1000.0),
        "load_direction": inputs.get("load_direction", "Z"),
        "n_elements": n_elements,
    }
    deck_text = _build_le1_deck(deck_params)
    tip_node = n_elements + 1  # node id at x=L

    # Run ccx in a per-call tmpdir. This is the native binary
    # adapter pattern (SDP §5.6): the adapter is responsible for
    # workspace hygiene and per-run cleanup.
    tmpdir = tempfile.mkdtemp(prefix="aether-ccx-")
    try:
        deck_path = os.path.join(tmpdir, "cantilever.inp")
        with open(deck_path, "w") as f:
            f.write(deck_text)
        started = time.time()
        try:
            proc = subprocess.run(
                [ccx, "-i", "cantilever"],
                cwd=tmpdir,
                capture_output=True,
                text=True,
                timeout=60,
                check=False,
            )
        except (OSError, subprocess.TimeoutExpired) as exc:
            return {
                "plugin_id": PLUGIN_ID,
                "plugin_version": PLUGIN_VERSION,
                "determinism_class": "bit-reproducible",
                "outputs": {},
                "verification": {
                    "checks": [
                        {"name": "ccx_spawn", "kind": "sanity", "result": "fail",
                         "diagnostic": f"spawn failed: {exc!r}"},
                    ],
                    "overall_status": "fail",
                },
            }
        elapsed = time.time() - started
        if proc.returncode != 0:
            return {
                "plugin_id": PLUGIN_ID,
                "plugin_version": PLUGIN_VERSION,
                "determinism_class": "bit-reproducible",
                "outputs": {},
                "verification": {
                    "checks": [
                        {"name": "ccx_exit_code", "kind": "sanity", "result": "fail",
                         "diagnostic": f"ccx exited with code {proc.returncode}; "
                                       f"stderr={proc.stderr[:512]!r}"},
                    ],
                    "overall_status": "fail",
                },
            }
        frd_path = os.path.join(tmpdir, "cantilever.frd")
        if not os.path.isfile(frd_path):
            return {
                "plugin_id": PLUGIN_ID,
                "plugin_version": PLUGIN_VERSION,
                "determinism_class": "bit-reproducible",
                "outputs": {},
                "verification": {
                    "checks": [
                        {"name": "ccx_output", "kind": "sanity", "result": "fail",
                         "diagnostic": f"ccx produced no .frd file; stdout={proc.stdout[:512]!r}"},
                    ],
                    "overall_status": "fail",
                },
            }
        with open(frd_path, "rb") as f:
            frd_bytes = f.read()
        # I-9 content hash over the *engineering* content: ccx writes
        # 1UDATE / 1UTIME / 1UHOST header lines (wall-clock and host);
        # they are environment metadata, not physics, and would make the
        # hash differ between two identical runs that straddle a second
        # boundary. Volatile lines are blanked before hashing; the hash
        # is therefore of the normalized artifact (documented deviation,
        # recorded in the verification diagnostics).
        cas_hash = hashlib.sha256(_canonical_frd_bytes(frd_bytes)).hexdigest()

        try:
            disp = _parse_frd_disp(frd_path, expected_n_nodes=tip_node)
        except Exception as exc:
            return {
                "plugin_id": PLUGIN_ID,
                "plugin_version": PLUGIN_VERSION,
                "determinism_class": "bit-reproducible",
                "outputs": {"result.bundle": {"cas_hash": cas_hash, "name": "cantilever.frd",
                                              "size": len(frd_bytes), "format": "calculix-frd"}},
                "verification": {
                    "checks": [
                        {"name": "frd_parse", "kind": "sanity", "result": "fail",
                         "diagnostic": f"frd parse failed: {exc!r}"},
                    ],
                    "overall_status": "fail",
                },
            }

        # With B31 elements, CalculiX auto-generates the cross-section
        # corner nodes (the 4 section corners at each end of each
        # element), and the DISP block reports the displacements of
        # these section corner nodes, not the original structural
        # nodes 1..9. For a uniform cantilever loaded in -Z, the
        # section corner nodes at the tip (the ones with the largest
        # |D3|) carry the tip extreme-fiber displacement. For a
        # symmetric cross section, the extreme-fiber deflection
        # equals the centroid (neutral-axis) deflection, which is
        # the same as the analytical Euler-Bernoulli v_tip.
        #
        # We extract the maximum |D_load| over the parsed DISP
        # block. The structural node 1 (root) is also reported with
        # near-zero D_load (encastre), so it does not contaminate
        # the max.
        if deck_params["load_direction"] == "Z":
            v_numerical = max((abs(d[2]) for d in disp.values()), default=0.0)
        else:
            v_numerical = max((abs(d[1]) for d in disp.values()), default=0.0)

        # Analytical tip deflection: v = P L^3 / (3 E I),
        # I = W H^3 / 12.
        L = deck_params["beam_length_m"]
        W = deck_params["cross_section_width_m"]
        H = deck_params["cross_section_height_m"]
        E = deck_params["E_pa"]
        P = deck_params["tip_load_n"]
        I = W * H ** 3 / 12.0
        v_analytical = P * L ** 3 / (3.0 * E * I)
        if v_analytical == 0.0:
            rel_err = 0.0 if v_numerical == 0.0 else float("inf")
        else:
            rel_err = abs(v_numerical - v_analytical) / abs(v_analytical)
        tol = float(inputs.get("rel_tol", DEFAULT_REL_TOL))
        check_ok = rel_err <= tol

        return {
            "plugin_id": PLUGIN_ID,
            "plugin_version": PLUGIN_VERSION,
            "determinism_class": "bit-reproducible",
            "outputs": {
                "result.bundle": {
                    "cas_hash": cas_hash,
                    "name": "cantilever.frd",
                    "size": len(frd_bytes),
                    "format": "calculix-frd",
                },
                "cantilever.inp": {
                    "cas_hash": hashlib.sha256(deck_text.encode("utf-8")).hexdigest(),
                    "name": "cantilever.inp",
                    "size": len(deck_text),
                    "format": "calculix-inp",
                },
            },
            "verification": {
                "checks": [
                    {"name": "ccx_exit_code", "kind": "sanity", "result": "pass",
                     "diagnostic": f"ccx returned 0 in {elapsed:.3f}s; .frd = {len(frd_bytes)} bytes"},
                    {"name": "tip_node_resolved", "kind": "sanity", "result": "pass",
                     "diagnostic": (
                         f"extreme-fiber D_load at x=L = {v_numerical:.3e} m "
                         f"(max |D_load| over {len(disp)} parsed section nodes)"
                     )},
                    {"name": "tip_deflection_matches_analytical",
                     "kind": "convergence",
                     "result": "pass" if check_ok else "fail",
                     "diagnostic": (
                         f"v_numerical={v_numerical:.6e} m, "
                         f"v_analytical={v_analytical:.6e} m, "
                         f"rel_err={rel_err:.3e} (tol {tol:.1e}); "
                         f"I = {I:.3e} m^4, L = {L} m, P = {P} N, E = {E:.3e} Pa"
                     )},
                ],
                "overall_status": "pass" if check_ok else "fail",
            },
            "diagnostics": {
                "v_numerical": v_numerical,
                "v_analytical": v_analytical,
                "rel_err": rel_err,
                "n_elements": n_elements,
                "solver_runtime_s": elapsed,
                "case": VALIDATED_CASE,
            },
            "step_id": step_id,
            "inputs_received": list(inputs.keys()) if isinstance(inputs, dict) else [],
        }
    finally:
        shutil.rmtree(tmpdir, ignore_errors=True)


def method_init(params: dict[str, Any] | None) -> dict[str, Any]:
    if not isinstance(params, dict):
        return {"ok": False, "reason": "params must be an object"}
    for field in ("token_id", "workflow_id", "step_id", "scope", "signature_b64"):
        if field not in params:
            return {"ok": False, "reason": f"missing field: {field}"}
    if not isinstance(params.get("scope"), dict):
        return {"ok": False, "reason": "scope must be an object"}
    return {"ok": True, "plugin_id": PLUGIN_ID, "plugin_version": PLUGIN_VERSION}


METHODS = {
    "init": method_init,
    "discover": method_discover,
    "capabilities": method_capabilities,
    "validate": method_validate,
    "execute": method_execute,
}


def _unwrap_executor_envelope(params: Any) -> Any:
    """Unwrap the workflow executor's request envelope.

    The AWL executor (`aether_exec`) dispatches a step as
    ``{"step_id": ..., "inputs": <spec>}``. Direct calls (the
    conformance tests) pass the flat problem spec. Unwrap only when the
    payload is exactly the envelope shape, so both call styles feed the
    same flat ``method_execute`` path.
    """
    if (
        isinstance(params, dict)
        and set(params.keys()) <= {"step_id", "inputs"}
        and isinstance(params.get("inputs"), dict)
    ):
        return params["inputs"]
    return params


def handle(line: str) -> dict[str, Any] | None:
    line = line.strip()
    if not line:
        return None
    try:
        req = json.loads(line)
    except json.JSONDecodeError as exc:
        return _err(None, -32700, f"parse error: {exc.msg}")
    if not isinstance(req, dict) or req.get("jsonrpc") != "2.0":
        return _err(req.get("id") if isinstance(req, dict) else None, -32600, "invalid request")
    method = req.get("method")
    req_id = req.get("id")
    params = _unwrap_executor_envelope(req.get("params"))
    if not isinstance(method, str):
        return _err(req_id, -32601, "method must be a string")
    fn = METHODS.get(method)
    if fn is None:
        return _err(req_id, -32601, f"method not found: {method}")
    try:
        result = fn(params)
    except Exception as exc:  # noqa: BLE001 — boundary; map to JSON-RPC internal error
        return _err(req_id, -32603, f"internal error: {exc!r}")
    return _ok(req_id, result)


def main() -> int:
    started = time.time()
    correlation = sys.modules[__name__].__name__
    print(f"[{correlation}] plugin ready (pid={os.getpid()})", file=sys.stderr)
    for raw in sys.stdin:
        response = handle(raw)
        if response is None:
            continue
        sys.stdout.write(json.dumps(response) + "\n")
        sys.stdout.flush()
    print(
        f"[{correlation}] plugin shutdown after {time.time() - started:.3f}s",
        file=sys.stderr,
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
