# SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
# SPDX-FileCopyrightText: 2026 Crocell Company
"""fea-reference-beam — real Euler-Bernoulli beam FEM with analytical
verification (SDP §5.6, §21; ADR-0022 two-implementation rule for
``aether.physics.solver``).

This is a **deliberately minimal structural FEA solver used only for
conformance testing** (SDP §21). It is *not* production structural
analysis. Its purpose is to provide a real, closed-form-checkable
implementation of the ``aether.physics.solver`` contract's
``execute`` path so that:

  1. The two-implementation rule (ADR-0022) for
     ``aether.physics.solver`` is satisfied by a *real* structural
     reference (the other reference, ``calculix-adapter``, is a
     hash-stub protocol shim that speaks the same contract surface;
     together they prove the contract is two-implementable).

  2. The conformance suite can assert, for a real classical
     problem, that the produced deflection matches the analytical
     solution to a stated tolerance — turning the contract's
     ``verify()`` method (I-7) from a structural claim into a
     numerical one.

FORMULATION
-----------
Euler-Bernoulli beam theory, plane bending, small deflections.
Each element is a 2-noded beam with 4 DOFs (v_i, theta_i, v_j,
theta_j). The element stiffness matrix K_e and consistent load
vector f_e are the standard textbook forms.

PROBLEMS SUPPORTED (closed-form-checkable)
------------------------------------------
1. Cantilever, point load P at free end
     v_tip = P L^3 / (3 E I)
2. Simply-supported, point load P at midspan
     v_mid = P L^3 / (48 E I)
3. Cantilever, uniformly distributed load q (N/m)
     v_tip = q L^4 / (8 E I)
4. Simply-supported, uniformly distributed load q (N/m)
     v_mid = 5 q L^4 / (384 E I)

For each problem, the plugin computes the numerical deflection
(assembled FEA solve) and the analytical deflection, then reports
the relative error in the ``verification.checks`` block. The
``verification.overall_status`` is ``pass`` iff the error is
within ``DEFAULT_REL_TOL = 1e-6`` and the solve converges.

DETERMINISM
-----------
The solver uses Python's float (IEEE 754 double). Element assembly
and the global solve use the closed-form inverse of the positive-
definite global stiffness (the beam problems here are small enough
to invert directly; no iterative solver is needed). The result is
bit-reproducible: same inputs produce the same bytes.

CONTRACT METHODS
----------------
- ``discover``   — static descriptor
- ``capabilities`` — capability facts (incl. validated-cases list)
- ``validate``   — checks the problem spec; reports missing fields
- ``execute``    — runs the FEA solve; returns result bundle +
  verification report
- ``init``       — run-token shape check (no Ed25519 verify yet)

NOT A PRODUCTION SOLVER
-----------------------
This plugin is a conformance fixture. Production structural
analysis is the domain of the production CalculiX / Code_Aster
adapters (G2/G3 deliverable, tracked in
``workflows/paper-run/gaps.md``). This fixture is what
``physics.solver`` tests against to prove the contract is
implementable by a real FEA algorithm.
"""

from __future__ import annotations

import hashlib
import json
import math
import sys
import time
import uuid
from typing import Any


PLUGIN_ID = "org.aether.reference.fe.beam"
PLUGIN_VERSION = "0.2.0"
CONTRACT = "aether.physics.solver"

# Default relative tolerance for the analytical-vs-numerical check.
# The closed-form solutions for the four supported problems are exact
# (no series truncation, no small-parameter expansion); the only
# source of numerical error is the FEA discretization. With N >= 4
# elements of a uniform beam, the relative L2 error of the tip/mid
# deflection vs the analytical value is < 1e-6; with N = 8 it is
# well below 1e-10. We choose 1e-6 as the default to allow reasonable
# N values; tighter tolerances require more elements.
DEFAULT_REL_TOL = 1.0e-6

# Validated cases — what ``capabilities()`` advertises and what the
# ``validate()`` method will accept. Keeping the set closed lets the
# conformance suite pin the contract surface and the four analytic
# problems together.
VALIDATED_CASES = (
    "cantilever_point",
    "cantilever_udl",
    "ss_point",
    "ss_udl",
)


def _err(req_id: Any, code: int, message: str) -> dict[str, Any]:
    return {"jsonrpc": "2.0", "id": req_id, "error": {"code": code, "message": message}}


def _ok(req_id: Any, result: dict[str, Any]) -> dict[str, Any]:
    return {"jsonrpc": "2.0", "id": req_id, "result": result}


def method_init(params: dict[str, Any] | None) -> dict[str, Any]:
    """Run-token shape check (structural only; no Ed25519 verify yet)."""
    if not isinstance(params, dict):
        return {"ok": False, "reason": "params must be an object"}
    for field in ("token_id", "workflow_id", "step_id", "scope", "signature_b64"):
        if field not in params:
            return {"ok": False, "reason": f"missing field: {field}"}
    if not isinstance(params.get("scope"), dict):
        return {"ok": False, "reason": "scope must be an object"}
    return {"ok": True, "plugin_id": PLUGIN_ID, "plugin_version": PLUGIN_VERSION}


def method_discover(_params: dict[str, Any] | None) -> dict[str, Any]:
    return {
        "plugin_id": PLUGIN_ID,
        "plugin_version": PLUGIN_VERSION,
        "contract_id": CONTRACT,
        "contract_version_range": ">=0.1.0 <1.0.0",
        "kind": "reference",
        "isolation": "process",
        "determinism_class": "bit-reproducible",
        "is_real_physics": True,
        "physics_kind": "euler_bernoulli_beam_fem",
        "validated_cases": list(VALIDATED_CASES),
        "note": (
            "Real Euler-Bernoulli beam FEM with closed-form analytical "
            "verification (I-7). Conformance fixture only; not a production "
            "structural solver (SDP §21). Production structural analysis is "
            "the domain of the G2/G3 CalculiX / Code_Aster adapters, "
            "tracked in workflows/paper-run/gaps.md."
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
            {"id": "cantilever_point", "kind": "linear_static",
             "description": "Cantilever beam, point load at free end",
             "analytical_formula": "v_tip = P L^3 / (3 E I)",
             "default_rel_tol": DEFAULT_REL_TOL},
            {"id": "cantilever_udl", "kind": "linear_static",
             "description": "Cantilever beam, uniformly distributed load",
             "analytical_formula": "v_tip = q L^4 / (8 E I)",
             "default_rel_tol": DEFAULT_REL_TOL},
            {"id": "ss_point", "kind": "linear_static",
             "description": "Simply-supported beam, point load at midspan",
             "analytical_formula": "v_mid = P L^3 / (48 E I)",
             "default_rel_tol": DEFAULT_REL_TOL},
            {"id": "ss_udl", "kind": "linear_static",
             "description": "Simply-supported beam, uniformly distributed load",
             "analytical_formula": "v_mid = 5 q L^4 / (384 E I)",
             "default_rel_tol": DEFAULT_REL_TOL},
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
    required = [
        "problem_id", "physics_class", "beam_length_m", "E_pa",
        "I_m4", "case", "n_elements",
    ]
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

    if params.get("physics_class") not in ("structural",):
        checks.append({
            "name": "physics_class_supported",
            "kind": "sanity",
            "result": "warn",
            "diagnostic": (
                f"fea-reference-beam only supports 'structural'; got "
                f"{params.get('physics_class')!r}"
            ),
        })

    if params.get("case") not in VALIDATED_CASES:
        checks.append({
            "name": "case_validated",
            "kind": "sanity",
            "result": "fail",
            "diagnostic": (
                f"case must be one of {list(VALIDATED_CASES)}; "
                f"got {params.get('case')!r}"
            ),
        })

    n_el = params.get("n_elements")
    if not isinstance(n_el, int) or n_el < 1:
        checks.append({
            "name": "n_elements_positive_integer",
            "kind": "sanity",
            "result": "fail",
            "diagnostic": f"n_elements must be a positive integer; got {n_el!r}",
        })

    # Load-magnitude checks (P for point, q for udl)
    case = params.get("case")
    if case in ("cantilever_point", "ss_point"):
        if not isinstance(params.get("P_n"), (int, float)):
            checks.append({
                "name": "P_n_required",
                "kind": "sanity",
                "result": "fail",
                "diagnostic": "point-load cases require P_n",
            })
    elif case in ("cantilever_udl", "ss_udl"):
        if not isinstance(params.get("q_n_per_m"), (int, float)):
            checks.append({
                "name": "q_n_per_m_required",
                "kind": "sanity",
                "result": "fail",
                "diagnostic": "UDL cases require q_n_per_m",
            })

    # Optional per-element EI scales: ei_scales[e] multiplies E*I on
    # element e (spanwise stiffness variation — as-built deviation).
    scales = params.get("ei_scales")
    if scales is not None:
        ok = (
            isinstance(scales, list)
            and len(scales) == params.get("n_elements")
            and all(isinstance(s, (int, float)) and s > 0 for s in scales)
        )
        if not ok:
            checks.append({
                "name": "ei_scales_wellformed",
                "kind": "sanity",
                "result": "fail",
                "diagnostic": (
                    "ei_scales must be a list of n_elements positive numbers"
                ),
            })
        else:
            checks.append({
                "name": "ei_scales_wellformed",
                "kind": "sanity",
                "result": "pass",
                "diagnostic": f"per-element EI scales: {len(scales)} elements",
            })

    overall = "pass"
    for c in checks:
        if c["result"] == "fail":
            overall = "fail"
            break
        if c["result"] == "warn" and overall == "pass":
            overall = "warn"

    return {"valid": overall != "fail", "checks": checks, "overall_status": overall}


# ---- FEA core ----

def _element_stiffness(E: float, I: float, L_e: float) -> list[list[float]]:
    """Standard 2-noded Euler-Bernoulli beam element stiffness matrix.

    DOFs: [v_i, theta_i, v_j, theta_j].
    Units: E in Pa, I in m^4, L_e in m -> stiffness in N/m for v-rows,
    N for theta-rows.
    """
    k = E * I / (L_e ** 3)
    return [
        [ 12 * k,   6 * k * L_e, -12 * k,   6 * k * L_e],
        [  6 * k * L_e,  4 * k * L_e * L_e, -6 * k * L_e,  2 * k * L_e * L_e],
        [-12 * k,  -6 * k * L_e,  12 * k,  -6 * k * L_e],
        [  6 * k * L_e,  2 * k * L_e * L_e, -6 * k * L_e,  4 * k * L_e * L_e],
    ]


def _element_load_point(L_e: float, P: float) -> list[float]:
    """Consistent nodal load vector for a uniformly distributed load q
    over an element of length L_e. For point loads, the load is
    applied as a concentrated force at the appropriate node (handled
    in the global assembly, not here)."""
    # Not used in the current validated cases (which use either
    # concentrated end forces or a per-element UDL assembled in
    # the global loop). Kept for completeness; would be invoked
    # for partial-UDL cases in a future expansion.
    raise NotImplementedError("point load is applied directly at nodes; use q-load path instead")


def _element_load_udl(L_e: float, q: float) -> list[float]:
    """Consistent nodal load vector for a uniformly distributed load q
    (N/m) over an element of length L_e. Standard result:
    f = q*L_e/2 * [1, L_e/6, 1, -L_e/6] (signs follow the convention
    that positive q acts in the +v direction; v is the transverse
    deflection)."""
    a = q * L_e / 2.0
    b = q * L_e * L_e / 12.0
    return [a, b, a, -b]


def _solve_global(K: list[list[float]], f: list[float], fixed_dofs: set[int]) -> list[float]:
    """Solve Ku = f with the listed DOFs clamped to zero. K is a
    dense (n x n) positive-definite matrix; we assemble the reduced
    system by striking rows/columns of fixed DOFs and invert it
    directly. For the small beam problems here (n_elements+1 nodes,
    2*(n_elements+1) DOFs) the cost is negligible and the result is
    bit-deterministic.
    """
    n = len(K)
    free = sorted(set(range(n)) - fixed_dofs)
    m = len(free)
    K_red = [[K[i][j] for j in free] for i in free]
    f_red = [f[i] for i in free]
    # Direct inverse: K_red is small (m <= ~32 for the validated cases).
    inv = _invert(K_red)
    u_red = [sum(inv[i][j] * f_red[j] for j in range(m)) for i in range(m)]
    u = [0.0] * n
    for k, idx in enumerate(free):
        u[idx] = u_red[k]
    return u


def _invert(M: list[list[float]]) -> list[list[float]]:
    """Gauss-Jordan inversion. Returns the inverse of M. Raises
    ValueError on singular input."""
    n = len(M)
    # Augment with identity.
    A = [list(row) + [1.0 if i == j else 0.0 for j in range(n)] for i, row in enumerate(M)]
    for i in range(n):
        # Pivot.
        piv = A[i][i]
        if abs(piv) < 1e-300:
            raise ValueError("singular matrix in _invert")
        inv_piv = 1.0 / piv
        for j in range(2 * n):
            A[i][j] *= inv_piv
        for k in range(n):
            if k == i:
                continue
            factor = A[k][i]
            if factor == 0.0:
                continue
            for j in range(2 * n):
                A[k][j] -= factor * A[i][j]
    return [row[n:] for row in A]


def _analytical_deflection(case: str, E: float, I: float, L: float,
                            P: float | None, q: float | None) -> tuple[float, str]:
    """Return (analytical_deflection, formula_text) for the four
    supported cases. Raises ValueError for unrecognised cases
    (caller has already filtered)."""
    if case == "cantilever_point":
        assert P is not None
        return P * L ** 3 / (3.0 * E * I), "v_tip = P L^3 / (3 E I)"
    if case == "cantilever_udl":
        assert q is not None
        return q * L ** 4 / (8.0 * E * I), "v_tip = q L^4 / (8 E I)"
    if case == "ss_point":
        assert P is not None
        return P * L ** 3 / (48.0 * E * I), "v_mid = P L^3 / (48 E I)"
    if case == "ss_udl":
        assert q is not None
        return 5.0 * q * L ** 4 / (384.0 * E * I), "v_mid = 5 q L^4 / (384 E I)"
    raise ValueError(f"unsupported case: {case!r}")


def _which_dof(case: str, n_nodes: int, L: float) -> int:
    """Return the global DOF index for the deflection we are checking.

    Beam nodes are indexed 0..n_nodes-1; each node has 2 DOFs
    (v = 2*node, theta = 2*node+1). We report the transverse
    deflection v at the load application point (tip for cantilever,
    midspan for SS)."""
    if case in ("cantilever_point", "cantilever_udl"):
        node = n_nodes - 1
    else:  # ss_point, ss_udl
        node = (n_nodes - 1) // 2  # midspan node
    return 2 * node  # v DOF


def _solve_beam(problem: dict[str, Any]) -> dict[str, Any]:
    """Run the FEA solve for one of the four validated cases.

    Returns a dict with:
      u: full displacement vector (length 2*(n_nodes))
      v_at_check: transverse deflection at the check point
      analytical_v: closed-form deflection at the same point
      rel_err: |v_at_check - analytical_v| / |analytical_v|
      formula: human-readable formula text
      converged: True
    """
    L = float(problem["beam_length_m"])
    E = float(problem["E_pa"])
    I = float(problem["I_m4"])
    n_el = int(problem["n_elements"])
    case = problem["case"]
    n_nodes = n_el + 1
    n_dof = 2 * n_nodes
    L_e = L / n_el

    # Global stiffness. ei_scales (optional) apply spanwise EI variation
    # element-by-element (as-built deviation). With scales present the
    # analytical_match check is reported as an informational model-form
    # comparison (result "info"), not a pass/fail gate.
    K = [[0.0] * n_dof for _ in range(n_dof)]
    Ke = _element_stiffness(E, I, L_e)
    scales = problem.get("ei_scales")
    for e in range(n_el):
        # Element DOF map: [v_i, th_i, v_j, th_j] -> global [2e, 2e+1, 2(e+1), 2(e+1)+1]
        gdof = [2 * e, 2 * e + 1, 2 * (e + 1), 2 * (e + 1) + 1]
        ke = Ke
        if scales is not None:
            ke = _element_stiffness(E, I * float(scales[e]), L_e)
        for a in range(4):
            for b in range(4):
                K[gdof[a]][gdof[b]] += ke[a][b]

    # Global load vector.
    f = [0.0] * n_dof
    if case == "cantilever_point":
        # Concentrated load P at the free end (node n_nodes-1, v DOF).
        f[2 * (n_nodes - 1)] = float(problem["P_n"])
    elif case == "ss_point":
        # Concentrated load P at midspan (node (n_nodes-1)//2).
        f[2 * ((n_nodes - 1) // 2)] = float(problem["P_n"])
    elif case in ("cantilever_udl", "ss_udl"):
        q = float(problem["q_n_per_m"])
        for e in range(n_el):
            fe = _element_load_udl(L_e, q)
            gdof = [2 * e, 2 * e + 1, 2 * (e + 1), 2 * (e + 1) + 1]
            for a in range(4):
                f[gdof[a]] += fe[a]
    else:
        raise ValueError(f"unsupported case: {case!r}")

    # Boundary conditions.
    fixed: set[int] = set()
    if case in ("cantilever_point", "cantilever_udl"):
        # Cantilever: v=0, theta=0 at x=0 (node 0).
        fixed.add(0)
        fixed.add(1)
    else:
        # SS: v=0 at both ends (nodes 0 and n_nodes-1).
        fixed.add(0)
        fixed.add(2 * (n_nodes - 1))

    u = _solve_global(K, f, fixed)

    check_dof = _which_dof(case, n_nodes, L)
    v_num = u[check_dof]

    P_arg = float(problem["P_n"]) if "P_n" in problem else None
    q_arg = float(problem["q_n_per_m"]) if "q_n_per_m" in problem else None
    v_ana, formula = _analytical_deflection(case, E, I, L, P_arg, q_arg)
    rel_err = abs(v_num - v_ana) / abs(v_ana) if v_ana != 0.0 else abs(v_num)

    # Spanwise profiles (sampled at element midpoints and ends).
    x_end = [ (e + 1) * L_e for e in range(n_el) ]
    v_end = [u[2 * (e + 1)] for e in range(n_el)]
    th_end = [u[2 * (e + 1) + 1] for e in range(n_el)]
    prof = {
        "x_m": [0.0] + x_end,
        "deflection_m": [u[0]] + v_end,
        "slope_rad": [u[1]] + th_end,
    }

    return {
        "u": u,
        "v_at_check": v_num,
        "analytical_v": v_ana,
        "rel_err": rel_err,
        "formula": formula,
        "check_dof": check_dof,
        "n_dof": n_dof,
        "n_nodes": n_nodes,
        "profile": prof,
        "has_ei_scales": scales is not None,
        "converged": True,
    }


def _bundle_hash(u: list[float], v_num: float, v_ana: float, rel_err: float) -> str:
    """Content hash for the result bundle. Stable over identical
    inputs (no UUIDs, no timestamps)."""
    h = hashlib.sha256()
    h.update(json.dumps(u, separators=(",", ":")).encode("utf-8"))
    h.update(b"|")
    h.update(f"{v_num!r}".encode("utf-8"))
    h.update(b"|")
    h.update(f"{v_ana!r}".encode("utf-8"))
    h.update(b"|")
    h.update(f"{rel_err!r}".encode("utf-8"))
    return h.hexdigest()


def method_execute(params: dict[str, Any] | None) -> dict[str, Any]:
    if not params:
        return {"outputs": {}}
    # Delegate to validate first so an obviously bad spec fails before
    # any numerics are touched.
    v = method_validate(params)
    if not v["valid"]:
        return {
            "plugin_id": PLUGIN_ID,
            "plugin_version": PLUGIN_VERSION,
            "determinism_class": "bit-reproducible",
            "outputs": {},
            "verification": {
                "checks": v["checks"],
                "overall_status": "fail",
            },
            "step_id": params.get("step_id", "unknown"),
        }
    try:
        result = _solve_beam(params)
    except Exception as exc:  # noqa: BLE001 — boundary
        return {
            "plugin_id": PLUGIN_ID,
            "plugin_version": PLUGIN_VERSION,
            "determinism_class": "bit-reproducible",
            "outputs": {},
            "verification": {
                "checks": [
                    {"name": "solve", "kind": "convergence", "result": "fail",
                     "diagnostic": f"exception in _solve_beam: {exc!r}"}
                ],
                "overall_status": "fail",
            },
            "step_id": params.get("step_id", "unknown"),
        }

    rel_tol = float(params.get("rel_tol", DEFAULT_REL_TOL))
    passes = result["rel_err"] <= rel_tol
    has_scales = result.get("has_ei_scales", False)
    # With ei_scales the uniform-EI closed form no longer applies: the
    # comparison is a model-form *diagnostic*, not a validation gate.
    if has_scales:
        match_check = {
            "name": "analytical_match", "kind": "model_form",
            "result": "info",
            "diagnostic": (
                f"v_numerical = {result['v_at_check']:.6e} with per-element "
                f"EI scales; uniform-EI closed form gives "
                f"{result['analytical_v']:.6e} ({result['formula']}) — "
                "informational comparison, not a validation gate"
            ),
            "rel_err": result["rel_err"],
        }
    else:
        match_check = {
            "name": "analytical_match", "kind": "validation",
            "result": "pass" if passes else "fail",
            "diagnostic": (
                f"v_numerical = {result['v_at_check']:.6e}, "
                f"v_analytical = {result['analytical_v']:.6e} "
                f"({result['formula']}), "
                f"rel_err = {result['rel_err']:.3e} "
                f"(tol {rel_tol:.1e})"
            ),
            "tolerance": rel_tol,
            "rel_err": result["rel_err"],
            "formula": result["formula"],
        }
    verification = {
        "checks": [
            {"name": "convergence", "kind": "convergence", "result": "pass",
             "diagnostic": "direct solve of assembled global system"},
            match_check,
        ],
        "overall_status":
            "pass" if (passes or has_scales) else "fail",
    }
    bundle_hash = _bundle_hash(
        result["u"], result["v_at_check"], result["analytical_v"], result["rel_err"]
    )
    return {
        "plugin_id": PLUGIN_ID,
        "plugin_version": PLUGIN_VERSION,
        "determinism_class": "bit-reproducible",
        "outputs": {
            "result.bundle": {
                "cas_hash": bundle_hash,
                "name": "result.bundle",
                "size": 0,
                "format": "native",
                "fidelity_class": "high_fidelity",
            }
        },
        "verification": verification,
        "diagnostics": {
            "v_numerical": result["v_at_check"],
            "v_analytical": result["analytical_v"],
            "rel_err": result["rel_err"],
            "formula": result["formula"],
            "n_dof": result["n_dof"],
            "n_nodes": result["n_nodes"],
            "profile": result["profile"],
        },
        "step_id": params.get("step_id", "unknown"),
        "inputs_received": [
            "problem_id", "physics_class", "beam_length_m", "E_pa",
            "I_m4", "case", "n_elements",
        ] + ([ "P_n" ] if "P_n" in params else [])
           + ([ "q_n_per_m" ] if "q_n_per_m" in params else [])
           + ([ "ei_scales" ] if "ei_scales" in params else []),
    }


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
    except Exception as exc:  # noqa: BLE001
        return _err(req_id, -32603, f"internal error: {exc!r}")
    return _ok(req_id, result)


def main() -> int:
    started = time.time()
    correlation = uuid.uuid4().hex[:8]
    print(f"[{correlation}] plugin ready (pid={__import__('os').getpid()})", file=sys.stderr)
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
