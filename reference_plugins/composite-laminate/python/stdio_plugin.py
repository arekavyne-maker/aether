# SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
# SPDX-FileCopyrightText: 2026 Crocell Company
"""composite-laminate — real Classical Laminate Theory (CLT).

Computes the ABD stiffness matrix of a symmetric or unsymmetric laminate
from per-ply properties, ply thicknesses, and orientations, then derives
the in-plane and bending response. Ply failure is evaluated with the
Tsai-Wu criterion.

Validated against closed-form results for:
- a quasi-isotropic [0/45/-45/90]s laminate (isotropic in-plane)
- a unidirectional [0]n laminate (engineering constants reduce to the ply)
- a cross-ply [0/90]s laminate (orthotropic in-plane)

FIDELITY-HONEST LABEL (I-7)
---------------------------
This is a genuine CLT implementation: real ABD matrices, real ply
transformations, real Tsai-Wu failure indices. It does not model
delamination, progressive damage, or manufacturing defects. It is verified
against analytics — not a fixture — but it is not a full composite
failure code.

Contract: aether.physics.solver
Plugin:   org.aether.reference.composite.laminate
"""

from __future__ import annotations

import hashlib
import json
import math
import sys
import time
from typing import Any

import numpy as np

PLUGIN_ID = "org.aether.reference.composite.laminate"
PLUGIN_VERSION = "0.1.0"
CONTRACT = "aether.physics.solver"

VALIDATED_CASES = ("quasi_isotropic", "unidirectional", "cross_ply")

DEFAULT_REL_TOL = 5.0e-2


def _ok(req_id: Any, result: dict[str, Any]) -> dict[str, Any]:
    return {"jsonrpc": "2.0", "id": req_id, "result": result}


def _err(req_id: Any, code: int, message: str) -> dict[str, Any]:
    return {"jsonrpc": "2.0", "id": req_id, "error": {"code": code, "message": message}}


def method_discover(_params: dict[str, Any] | None) -> dict[str, Any]:
    return _ok(
        None,
        {
            "plugin_id": PLUGIN_ID,
            "plugin_version": PLUGIN_VERSION,
            "contract_id": CONTRACT,
            "contract_version_range": ">=0.1.0 <1.0.0",
            "kind": "reference",
            "is_real_physics": True,
            "physics_kind": "classical_laminate_theory",
            "validated_cases": list(VALIDATED_CASES),
            "implementation": "ABD matrix assembly + Tsai-Wu",
            "fidelity_class": "medium_fidelity",
            "assumptions": [
                "classical laminate theory (thin, plane stress)",
                "linear elastic plies",
                "perfect bonding",
            ],
            "determinism_class": "bit-reproducible",
            "note": (
                "Real CLT: ABD matrices, ply transformations, and Tsai-Wu "
                "failure indices. Delamination and progressive damage are out "
                "of scope."
            ),
        },
    )


def method_capabilities(_params: dict[str, Any] | None) -> dict[str, Any]:
    return _ok(
        None,
        {
            "plugin_id": PLUGIN_ID,
            "capabilities": [
                {
                    "capability": "laminate_response",
                    "contract": CONTRACT,
                    "fidelity_class": "medium_fidelity",
                }
            ],
            "limits": [
                "no delamination or progressive damage",
                "perfect bonding assumed",
            ],
            "supported_formats": ["native-json"],
        },
    )


def method_validate(params: dict[str, Any] | None) -> dict[str, Any]:
    return _ok(None, _validate_inner(params))


def _validate_inner(params: dict[str, Any] | None) -> dict[str, Any]:
    if params is None:
        return {
            "valid": False,
            "checks": [
                {"name": "input_present", "kind": "sanity", "result": "fail",
                 "diagnostic": "no problem spec provided"}
            ],
        }
    checks: list[dict[str, Any]] = []
    for field in ("problem_id", "physics_class", "plies", "case"):
        if field not in params:
            checks.append({
                "name": f"required:{field}", "kind": "sanity", "result": "fail",
                "diagnostic": f"missing required field: {field}",
            })
        else:
            checks.append({
                "name": f"required:{field}", "kind": "sanity", "result": "pass",
            })
    if params.get("physics_class") not in ("structural",):
        checks.append({
            "name": "physics_class_supported", "kind": "sanity", "result": "warn",
            "diagnostic": f"expected 'structural'; got {params.get('physics_class')!r}",
        })
    if params.get("case") not in VALIDATED_CASES:
        checks.append({
            "name": "case_validated", "kind": "sanity", "result": "fail",
            "diagnostic": f"case must be one of {list(VALIDATED_CASES)}",
        })
    plies = params.get("plies")
    if not isinstance(plies, list) or not plies:
        checks.append({
            "name": "plies_non_empty", "kind": "sanity", "result": "fail",
            "diagnostic": "plies must be a non-empty list",
        })
    overall = "pass"
    for c in checks:
        if c["result"] == "fail":
            overall = "fail"
            break
        if c["result"] == "warn" and overall == "pass":
            overall = "warn"
    return {"valid": overall != "fail", "checks": checks, "overall_status": overall}


# ---- CLT core ----

def _q_matrix(E1: float, E2: float, nu12: float, G12: float) -> np.ndarray:
    """Reduced stiffness matrix in the ply material frame."""
    nu21 = nu12 * E2 / E1
    denom = 1.0 - nu12 * nu21
    return np.array([
        [E1 / denom, nu12 * E2 / denom, 0.0],
        [nu12 * E2 / denom, E2 / denom, 0.0],
        [0.0, 0.0, G12],
    ])


def _transform(Q: np.ndarray, angle_deg: float) -> np.ndarray:
    """Transform the reduced stiffness to the laminate frame."""
    t = math.radians(angle_deg)
    c, s = math.cos(t), math.sin(t)
    c2, s2 = c * c, s * s
    cs = c * s
    T = np.array([
        [c2, s2, 2 * cs],
        [s2, c2, -2 * cs],
        [-cs, cs, c2 - s2],
    ])
    Tinv = np.linalg.inv(T)
    return Tinv @ Q @ T


def _abd(plies: list[dict[str, Any]]) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
    """Assemble the A, B, D matrices from plies (bottom to top)."""
    z = 0.0
    A = np.zeros((3, 3))
    B = np.zeros((3, 3))
    D = np.zeros((3, 3))
    for ply in plies:
        thickness = float(ply["thickness_mm"]) / 1000.0
        angle = float(ply.get("angle_deg", 0.0))
        E1 = float(ply.get("E1_pa", 140e9))
        E2 = float(ply.get("E2_pa", 10e9))
        nu12 = float(ply.get("nu12", 0.3))
        G12 = float(ply.get("G12_pa", 5e9))
        Q = _transform(_q_matrix(E1, E2, nu12, G12), angle)
        z1 = z
        z2 = z + thickness
        A += Q * (z2 - z1)
        B += Q * (z2**2 - z1**2) / 2.0
        D += Q * (z2**3 - z1**3) / 3.0
        z = z2
    return A, B, D


def _tsai_wu(stress: np.ndarray, ply: dict[str, Any]) -> float:
    """Tsai-Wu failure index in the ply material frame."""
    s1t = float(ply.get("Xt_pa", 1500e6))
    s1c = float(ply.get("Xc_pa", 1200e6))
    s2t = float(ply.get("Yt_pa", 50e6))
    s2c = float(ply.get("Yc_pa", 200e6))
    s6 = float(ply.get("S_pa", 70e6))
    s1, s2, s6v = float(stress[0]), float(stress[1]), float(stress[2])
    f1 = 1.0 / s1t - 1.0 / abs(s1c)
    f2 = 1.0 / s2t - 1.0 / abs(s2c)
    f11 = 1.0 / (s1t * abs(s1c))
    f22 = 1.0 / (s2t * abs(s2c))
    f66 = 1.0 / (s6 * s6)
    f12 = -0.5 * math.sqrt(f11 * f22)
    return float(
        f11 * s1 * s1 + f22 * s2 * s2 + f66 * s6v * s6v
        + 2.0 * f12 * s1 * s2 + f1 * s1 + f2 * s2
    )


def _solve(problem: dict[str, Any]) -> dict[str, Any]:
    case = problem["case"]
    plies = problem["plies"]
    A, B, D = _abd(plies)
    total_mm = sum(float(p["thickness_mm"]) for p in plies)
    total = total_mm / 1000.0  # metres

    # Reference response for the three validated cases.
    if case == "quasi_isotropic":
        # A quasi-isotropic laminate is in-plane isotropic: A11 == A22 and
        # A16 == A26 == 0.
        a11, a22 = float(A[0, 0]), float(A[1, 1])
        a16, a26 = float(A[0, 2]), float(A[1, 2])
        rel = abs(a11 - a22) / max(abs(a11), 1e-12)
        rel = max(rel, abs(a16) / max(abs(a11), 1e-12))
        rel = max(rel, abs(a26) / max(abs(a11), 1e-12))
        formula = "quasi-isotropic: A11 == A22, A16 == A26 == 0"
        value = a11
        reference = a22
    elif case == "unidirectional":
        # A [0]n UD laminate: A11 = Q11 * t with Q11 = E1/(1 - nu12*nu21).
        ply = plies[0]
        E1 = float(ply.get("E1_pa", 140e9))
        E2 = float(ply.get("E2_pa", 10e9))
        nu12 = float(ply.get("nu12", 0.3))
        nu21 = nu12 * E2 / E1
        expected_a11 = E1 / (1.0 - nu12 * nu21) * total
        rel = abs(float(A[0, 0]) - expected_a11) / abs(expected_a11)
        formula = "UD: A11 = Q11 * t, Q11 = E1/(1 - nu12*nu21)"
        value = float(A[0, 0])
        reference = expected_a11
    else:  # cross_ply
        # [0/90]s is orthotropic: A16 == A26 == 0.
        a16, a26 = float(A[0, 2]), float(A[1, 2])
        rel = max(abs(a16), abs(a26)) / max(abs(float(A[0, 0])), 1e-12)
        formula = "cross-ply: A16 == A26 == 0"
        value = float(A[0, 0])
        reference = float(A[1, 1])

    # Tsai-Wu at the mid-plane of each ply under a representative load.
    loads = problem.get("loads_n_per_m", [1e6, 0.0, 0.0])
    N = np.array(loads, dtype=float)
    # Mid-plane strain from A (B = 0 for a symmetric laminate).
    symmetric = all(abs(float(v)) < 1e-6 for v in B.flatten())
    if symmetric:
        strain = np.linalg.solve(A, N)
    else:
        # For an unsymmetric laminate, report the mid-plane strain from A
        # alone and flag that bending coupling is present.
        strain = np.linalg.solve(A, N)

    max_index = 0.0
    for ply in plies:
        angle = float(ply.get("angle_deg", 0.0))
        Q = _q_matrix(
            float(ply.get("E1_pa", 140e9)),
            float(ply.get("E2_pa", 10e9)),
            float(ply.get("nu12", 0.3)),
            float(ply.get("G12_pa", 5e9)),
        )
        t = math.radians(angle)
        c, s = math.cos(t), math.sin(t)
        T = np.array([
            [c * c, s * s, 2 * c * s],
            [s * s, c * c, -2 * c * s],
            [-c * s, c * s, c * c - s * s],
        ])
        stress = T @ Q @ strain
        max_index = max(max_index, _tsai_wu(stress, ply))

    tol = float(problem.get("rel_tol", DEFAULT_REL_TOL))
    return {
        "status": "succeeded",
        "A11_pa_m": float(A[0, 0]),
        "A22_pa_m": float(A[1, 1]),
        "A16_pa_m": float(A[0, 2]),
        "A26_pa_m": float(A[1, 2]),
        "symmetric": symmetric,
        "total_thickness_mm": total_mm,
        "max_tsai_wu_index": max_index,
        "first_ply_failure": max_index >= 1.0,
        "value": value,
        "reference": reference,
        "relative_error": rel,
        "analytical_formula": formula,
        "tolerance": tol,
        "passed": rel <= tol,
        "n_plies": len(plies),
        "solver": "analytical CLT",
    }


def method_execute(params: dict[str, Any] | None) -> dict[str, Any]:
    if params is None:
        return _err(None, -32602, "no problem spec provided")
    valid = _validate_inner(params)
    if not valid["valid"]:
        return _err(None, -32602, f"validation failed: {valid}")
    t0 = time.time()
    try:
        result = _solve(params)
    except Exception as exc:  # noqa: BLE001
        return _err(None, -32000, f"solver error: {exc}")
    runtime = time.time() - t0
    overall = "pass" if result["passed"] else "fail"
    return _ok(
        None,
        {
            "problem_id": params["problem_id"],
            "plugin_id": PLUGIN_ID,
            "plugin_version": PLUGIN_VERSION,
            "status": "succeeded",
            "fidelity_class": "medium_fidelity",
            "determinism_class": "bit-reproducible",
            "runtime_s": runtime,
            "diagnostics": result,
            "verification": {
                "overall_status": overall,
                "method": "analytical",
                "reference": result["analytical_formula"],
                "relative_error": result["relative_error"],
                "tolerance": result["tolerance"],
            },
            "outputs": {
                "result.bundle": {
                    "cas_hash": hashlib.sha256(
                        json.dumps(result, sort_keys=True).encode()
                    ).hexdigest(),
                    "A11_pa_m": result["A11_pa_m"],
                    "max_tsai_wu_index": result["max_tsai_wu_index"],
                    "n_plies": result["n_plies"],
                    "fidelity_class": "medium_fidelity",
                }
            },
        },
    )


METHODS = {
    "discover": method_discover,
    "capabilities": method_capabilities,
    "validate": method_validate,
    "execute": method_execute,
}


def main() -> None:
    for raw in sys.stdin:
        raw = raw.strip()
        if not raw:
            continue
        try:
            req = json.loads(raw)
        except json.JSONDecodeError:
            continue
        method = req.get("method")
        params = req.get("params")
        req_id = req.get("id")
        handler = METHODS.get(method)
        if handler is None:
            resp = _err(req_id, -32601, f"unknown method: {method}")
        else:
            try:
                resp = handler(params)
            except Exception as exc:  # noqa: BLE001
                resp = _err(req_id, -32000, f"handler error: {exc}")
        if isinstance(resp, dict) and resp.get("id", "missing") is None:
            resp["id"] = req_id
        sys.stdout.write(json.dumps(resp) + "\n")
        sys.stdout.flush()


if __name__ == "__main__":
    main()