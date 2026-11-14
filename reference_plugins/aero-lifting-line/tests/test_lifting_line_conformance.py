# SPDX-License-Identifier: Aether-VPSL-1.0
# SPDX-FileCopyrightText: 2026 The Aether Consortium
"""Conformance tests for aero-lifting-line (ADR-0022 aero side).

The conformance criterion: the Glauert-Fourier solution of the
discretized monoplane equation must reproduce the classical
elliptical-loading closed form

    CL = a0 (alpha - alpha_0) / (1 + a0 / (pi AR))

at machine precision (the elliptical planform is a fixed point of the
discretization: higher Fourier coefficients vanish), and the induced
drag identity CDi / CL^2 = 1/(pi AR) must hold to ~1e-16 relative.

Both the direct-import path and the stdio JSON-RPC round-trip are
exercised, mirroring the fea-reference-beam conformance suite.
"""

from __future__ import annotations

import json
import math
import subprocess
import sys
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parents[3]
PLUGIN = REPO_ROOT / "reference_plugins" / "aero-lifting-line" / "python" / "stdio_plugin.py"

import importlib.util

_spec = importlib.util.spec_from_file_location("aero_lifting_line_stdio_plugin", PLUGIN)
assert _spec and _spec.loader
aero = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(aero)


def _spec(alpha=5.0, a0=2 * math.pi, ar=8.0, e=0.85, n=16, tol=1e-6, case="elliptical"):
    return {
        "problem_id": "aero-conformance",
        "physics_class": "aerodynamic",
        "case": case,
        "alpha_deg": alpha,
        "cl_alpha_2d": a0,
        "alpha_0_deg": 0.0,
        "aspect_ratio": ar,
        "oswald_e": e,
        "n_sections": n,
        "rel_tolerance": tol,
    }


def _analytical_cl(alpha, a0, ar):
    return a0 * math.radians(alpha) / (1.0 + a0 / (math.pi * ar))


# ----------------------------------------------------------------------
# Numerical correctness (direct import)
# ----------------------------------------------------------------------


@pytest.mark.parametrize("ar", [4.0, 8.0, 12.0, 20.0])
@pytest.mark.parametrize("alpha", [-3.0, 2.0, 5.0])
def test_cl_matches_elliptical_closed_form(alpha, ar):
    res = aero.method_execute(_spec(alpha=alpha, ar=ar))
    assert res["verification"]["overall_status"] == "pass"
    diag = res["diagnostics"]
    assert diag["rel_err"] <= 1e-9
    assert diag["cl_total"] == pytest.approx(_analytical_cl(alpha, 2 * math.pi, ar), rel=1e-9)


@pytest.mark.parametrize("n", [4, 8, 16, 32])
def test_discretization_is_exact_for_elliptical_planform(n):
    """Higher Fourier coefficients must vanish: the elliptical loading is
    a fixed point of the monoplane discretization."""
    res = aero.method_execute(_spec(n=n))
    coeffs = res["diagnostics"]["fourier_coefficients"]
    assert abs(coeffs[0]) > 1e-3
    for c in coeffs[1:]:
        assert abs(c) < 1e-12


def test_induced_drag_identity():
    res = aero.method_execute(_spec())
    diag = res["diagnostics"]
    cdi_expected = diag["cl_total"] ** 2 / (math.pi * 8.0)
    assert diag["cdi_total"] == pytest.approx(cdi_expected, rel=1e-9)


def test_2d_limit_recovers_section_slope():
    """AR=200: induced share a0/(pi AR) ~= 1%; the closed form must sit
    proportionally below the 2D slope."""
    ar = 200.0
    res = aero.method_execute(_spec(ar=ar))
    cl = res["diagnostics"]["cl_total"]
    cl_2d = 2 * math.pi * math.radians(5.0)
    assert cl < cl_2d  # induced downwash reduces the lift
    assert cl == pytest.approx(cl_2d / (1.0 + 2 * math.pi / (math.pi * ar)), rel=1e-9)


def test_alpha_zero_gives_zero_lift():
    res = aero.method_execute(_spec(alpha=0.0))
    assert res["diagnostics"]["cl_total"] == pytest.approx(0.0, abs=1e-14)


def test_lift_is_odd_in_alpha():
    r1 = aero.method_execute(_spec(alpha=4.0))["diagnostics"]["cl_total"]
    r2 = aero.method_execute(_spec(alpha=-4.0))["diagnostics"]["cl_total"]
    assert r1 == pytest.approx(-r2, rel=1e-12)


def test_validation_rejects_bad_specs():
    bad = _spec()
    bad["physics_class"] = "structural"
    v = aero.method_validate(bad)
    assert not v["valid"]
    bad2 = _spec(case="rectangular")  # not in validated cases
    assert not aero.method_validate(bad2)["valid"]
    bad3 = _spec(n=7)  # odd
    assert not aero.method_validate(bad3)["valid"]


def test_determinism_same_input_same_bundle_hash():
    h1 = aero.method_execute(_spec())["outputs"]["result.bundle"]["cas_hash"]
    h2 = aero.method_execute(_spec())["outputs"]["result.bundle"]["cas_hash"]
    assert h1 == h2


# ----------------------------------------------------------------------
# Contract surface (stdio JSON-RPC round-trip)
# ----------------------------------------------------------------------


def _rpc(method: str, params: dict, req_id: int = 1) -> dict:
    req = json.dumps({"jsonrpc": "2.0", "id": req_id, "method": method, "params": params})
    proc = subprocess.run(
        [sys.executable, str(PLUGIN)],
        input=req,
        capture_output=True,
        text=True,
        timeout=60,
    )
    assert proc.returncode == 0, proc.stderr
    lines = [json.loads(x) for x in proc.stdout.splitlines() if x.strip()]
    assert lines, "plugin produced no response"
    return lines[-1]


def test_rpc_discover_advertises_contract():
    resp = _rpc("discover", {})
    assert "result" in resp
    d = resp["result"]
    assert d["plugin_id"] == "org.aether.reference.aero.liftingline"
    assert d["contract_id"] == "aether.physics.solver"
    assert d["determinism_class"] == "bit-reproducible"
    assert d["is_real_physics"] is True


def test_rpc_execute_envelope_round_trip():
    """The executor dispatches steps as {step_id, inputs}; the plugin must
    unwrap that envelope and produce a verified result."""
    resp = _rpc("execute", {"step_id": "aero-step", "inputs": _spec()})
    res = resp["result"]
    assert res["verification"]["overall_status"] == "pass"
    assert res["step_id"] == "aero-step"
    assert res["diagnostics"]["classification"] == "REFERENCE"


def test_rpc_flat_params_also_accepted():
    resp = _rpc("execute", _spec())
    assert resp["result"]["verification"]["overall_status"] == "pass"


def test_rpc_validate_rejects_unknown_case():
    resp = _rpc("validate", {"case": "delta_wing", "physics_class": "aerodynamic"})
    assert resp["result"]["valid"] is False
