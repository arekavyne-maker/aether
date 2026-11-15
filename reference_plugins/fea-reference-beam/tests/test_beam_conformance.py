# SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
# SPDX-FileCopyrightText: 2026 Crocell Company
"""Conformance tests for fea-reference-beam (SDP §5.6, §21; ADR-0022).

These tests exercise the plugin's ``execute`` method against four
analytical problems (Euler-Bernoulli beam theory). The conformance
criterion is the closed-form deflection at the canonical load point
matches the FEA-computed value within ``DEFAULT_REL_TOL = 1e-6``.

The tests use the plugin in two ways:

1. Direct Python import: import ``stdio_plugin`` as a module and
   call ``method_execute`` directly. This is the fast path for
   unit-level verification of the numerical correctness.

2. JSON-RPC round-trip: spawn the plugin as a subprocess, send a
   ``discover`` and an ``execute`` request, parse the response.
   This is the contract-surface path that proves the plugin
   speaks stdio JSON-RPC correctly (the IPC substrate the
   rest of the platform depends on).
"""

from __future__ import annotations

import json
import math
import os
import subprocess
import sys
from pathlib import Path

import pytest


REPO_ROOT = Path(__file__).resolve().parents[3]
PLUGIN = REPO_ROOT / "reference_plugins" / "fea-reference-beam" / "python" / "stdio_plugin.py"
sys.path.insert(0, str(PLUGIN.parent))

# Importing as a module so we can call the methods directly without
# spawning a subprocess. The module is named "stdio_plugin" but it
# is importable; we use a unique alias to avoid colliding with the
# cfd-reference-minimal plugin which has the same filename.
import importlib.util

_spec = importlib.util.spec_from_file_location("fea_ref_beam_stdio_plugin", PLUGIN)
assert _spec and _spec.loader
beam = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(beam)


# Common beam geometry: L=1 m, E=200 GPa (steel), I = 1e-6 m^4
# (1 cm^4). These values are arbitrary; what matters is the
# dimensionless agreement with the analytical formula.
COMMON = dict(
    beam_length_m=1.0,
    E_pa=200e9,
    I_m4=1e-6,
    physics_class="structural",
    n_elements=8,
)


def _run_case_direct(case: str, **extra) -> dict:
    params = {**COMMON, "case": case, **extra}
    return beam.method_execute(params)


# ----- Direct-call tests -----

def test_cantilever_point_load_analytical() -> None:
    """Cantilever, point load P=1000 N at free end.

    Analytical: v_tip = P L^3 / (3 E I)
              = 1000 * 1^3 / (3 * 200e9 * 1e-6) = 1.6667e-3 m = 1.6667 mm
    """
    P = 1000.0
    result = _run_case_direct("cantilever_point", P_n=P, problem_id="test-cp-1")
    v = result["diagnostics"]["v_numerical"]
    v_ana = result["diagnostics"]["v_analytical"]
    rel_err = result["diagnostics"]["rel_err"]
    expected = P * COMMON["beam_length_m"] ** 3 / (3.0 * COMMON["E_pa"] * COMMON["I_m4"])
    assert math.isclose(v_ana, expected, rel_tol=1e-12), (
        f"analytical formula wrong: got {v_ana}, expected {expected}"
    )
    assert rel_err < beam.DEFAULT_REL_TOL, (
        f"cantilever_point: rel_err={rel_err} >= tol {beam.DEFAULT_REL_TOL}; "
        f"v_num={v}, v_ana={v_ana}"
    )
    assert result["verification"]["overall_status"] == "pass"


def test_cantilever_udl_analytical() -> None:
    """Cantilever, UDL q=1000 N/m.

    Analytical: v_tip = q L^4 / (8 E I)
              = 1000 * 1^4 / (8 * 200e9 * 1e-6) = 6.25e-4 m = 0.625 mm
    """
    q = 1000.0
    result = _run_case_direct("cantilever_udl", q_n_per_m=q, problem_id="test-cu-1")
    v_ana = result["diagnostics"]["v_analytical"]
    expected = q * COMMON["beam_length_m"] ** 4 / (8.0 * COMMON["E_pa"] * COMMON["I_m4"])
    assert math.isclose(v_ana, expected, rel_tol=1e-12)
    assert result["diagnostics"]["rel_err"] < beam.DEFAULT_REL_TOL
    assert result["verification"]["overall_status"] == "pass"


def test_ss_point_load_analytical() -> None:
    """Simply-supported, point load P=1000 N at midspan.

    Analytical: v_mid = P L^3 / (48 E I)
              = 1000 * 1^3 / (48 * 200e9 * 1e-6) = 1.0417e-4 m
    """
    P = 1000.0
    result = _run_case_direct("ss_point", P_n=P, problem_id="test-sp-1")
    v_ana = result["diagnostics"]["v_analytical"]
    expected = P * COMMON["beam_length_m"] ** 3 / (48.0 * COMMON["E_pa"] * COMMON["I_m4"])
    assert math.isclose(v_ana, expected, rel_tol=1e-12)
    assert result["diagnostics"]["rel_err"] < beam.DEFAULT_REL_TOL
    assert result["verification"]["overall_status"] == "pass"


def test_ss_udl_analytical() -> None:
    """Simply-supported, UDL q=1000 N/m.

    Analytical: v_mid = 5 q L^4 / (384 E I)
              = 5 * 1000 * 1^4 / (384 * 200e9 * 1e-6) = 6.5104e-5 m
    """
    q = 1000.0
    result = _run_case_direct("ss_udl", q_n_per_m=q, problem_id="test-su-1")
    v_ana = result["diagnostics"]["v_analytical"]
    expected = (
        5.0 * q * COMMON["beam_length_m"] ** 4
        / (384.0 * COMMON["E_pa"] * COMMON["I_m4"])
    )
    assert math.isclose(v_ana, expected, rel_tol=1e-12)
    assert result["diagnostics"]["rel_err"] < beam.DEFAULT_REL_TOL
    assert result["verification"]["overall_status"] == "pass"


# ----- Convergence / refinement tests -----

@pytest.mark.parametrize("n_el,expected_max_err", [
    (2, 1e-2),
    (4, 1e-4),
    (8, 1e-6),
    (16, 1e-8),
])
def test_convergence_with_n_elements(n_el: int, expected_max_err: float) -> None:
    """The FEA error vs analytical should decrease as n_elements grows.
    With n=8 it is < 1e-6; with n=16 it is < 1e-8.
    """
    params = {**COMMON, "n_elements": n_el, "case": "cantilever_point", "P_n": 1000.0,
              "problem_id": f"conv-{n_el}"}
    result = beam.method_execute(params)
    rel_err = result["diagnostics"]["rel_err"]
    assert rel_err < expected_max_err, (
        f"n_el={n_el}: rel_err={rel_err:.3e} >= {expected_max_err:.1e}"
    )


# ----- Determinism tests -----

def test_repeated_invocation_is_bit_identical() -> None:
    """Same inputs -> same output hash (I-9, bit-reproducible)."""
    params = {**COMMON, "case": "cantilever_point", "P_n": 1234.5, "problem_id": "det-1"}
    r1 = beam.method_execute(params)
    r2 = beam.method_execute(params)
    assert r1["outputs"]["result.bundle"]["cas_hash"] == r2["outputs"]["result.bundle"]["cas_hash"]
    assert r1["diagnostics"] == r2["diagnostics"]


# ----- Validation tests -----

def test_unsupported_case_fails_validation() -> None:
    params = {**COMMON, "case": "mystery", "P_n": 100.0, "problem_id": "bad-1"}
    result = beam.method_execute(params)
    assert result["verification"]["overall_status"] == "fail"
    assert any(
        c["name"] == "case_validated" and c["result"] == "fail"
        for c in result["verification"]["checks"]
    )


def test_missing_load_magnitude_fails_validation() -> None:
    """Point-load case without P_n should fail validation."""
    params = {**COMMON, "case": "cantilever_point", "problem_id": "bad-2"}
    result = beam.method_execute(params)
    assert result["verification"]["overall_status"] == "fail"


def test_discover_advertises_is_real_physics() -> None:
    """The discover() response must mark this as a real-physics
    reference (the I-7 / honesty discipline)."""
    d = beam.method_discover(None)
    assert d["is_real_physics"] is True
    assert d["physics_kind"] == "euler_bernoulli_beam_fem"
    assert set(d["validated_cases"]) == set(beam.VALIDATED_CASES)


# ----- JSON-RPC substrate test (the contract surface) -----

def test_stdio_jsonrpc_discover() -> None:
    """Spawn the plugin as a subprocess and verify it speaks
    stdio JSON-RPC 2.0 correctly."""
    request = {"jsonrpc": "2.0", "id": "rpc-discover", "method": "discover"}
    proc = subprocess.run(
        [sys.executable, str(PLUGIN)],
        input=json.dumps(request) + "\n",
        capture_output=True,
        text=True,
        timeout=15,
        check=False,
    )
    assert proc.returncode == 0
    response = None
    for raw in proc.stdout.splitlines():
        if not raw.strip():
            continue
        try:
            r = json.loads(raw)
        except json.JSONDecodeError:
            continue
        if r.get("id") == "rpc-discover":
            response = r
            break
    assert response is not None, f"no JSON-RPC response; stdout={proc.stdout!r}"
    assert response["jsonrpc"] == "2.0"
    assert response["result"]["plugin_id"] == beam.PLUGIN_ID


def test_stdio_jsonrpc_execute_cantilever() -> None:
    """Full JSON-RPC round-trip on a cantilever-point case."""
    request = {
        "jsonrpc": "2.0",
        "id": "rpc-exec-1",
        "method": "execute",
        "params": {**COMMON, "case": "cantilever_point", "P_n": 1000.0, "problem_id": "rpc-1"},
    }
    proc = subprocess.run(
        [sys.executable, str(PLUGIN)],
        input=json.dumps(request) + "\n",
        capture_output=True,
        text=True,
        timeout=15,
        check=False,
    )
    assert proc.returncode == 0
    response = None
    for raw in proc.stdout.splitlines():
        if not raw.strip():
            continue
        try:
            r = json.loads(raw)
        except json.JSONDecodeError:
            continue
        if r.get("id") == "rpc-exec-1":
            response = r
            break
    assert response is not None, f"no JSON-RPC response; stdout={proc.stdout!r}"
    result = response["result"]
    assert result["verification"]["overall_status"] == "pass"
    assert result["diagnostics"]["rel_err"] < beam.DEFAULT_REL_TOL


# ---- Wave 4: per-element EI scales + spanwise profiles ----

def _base_spec() -> dict:
    return {
        "problem_id": "wave4-ei-scales",
        "physics_class": "structural",
        "beam_length_m": 2.0,
        "E_pa": 72e9,
        "I_m4": 1.0e-6,
        "case": "cantilever_udl",
        "q_n_per_m": 100.0,
        "n_elements": 8,
    }


def test_profile_present_and_tip_slope_matches_closed_form() -> None:
    """The spanwise profile carries deflection and slope; the tip slope
    matches the independent closed form qL^3/(6EI) to FEM accuracy."""
    r = beam.method_execute(_base_spec())
    assert r["verification"]["overall_status"] == "pass"
    prof = r["diagnostics"]["profile"]
    assert len(prof["x_m"]) == 9  # n_elements + 1 nodes
    assert prof["x_m"][0] == 0.0
    assert prof["deflection_m"][0] == 0.0  # clamped
    ana_tip_slope = 100.0 * 8.0 / (6.0 * 72e9 * 1.0e-6)
    err = abs(prof["slope_rad"][-1] - ana_tip_slope) / ana_tip_slope
    assert err < 1e-6


def test_ei_scales_soften_deflection_and_downgrade_check_kind() -> None:
    """A 20% single-element stiffness loss increases tip deflection;
    the uniform-EI closed-form check is reported as a model-form
    comparison (info), not a validation gate."""
    scales = [1.0] * 8
    scales[3] = 0.80
    spec = _base_spec()
    spec["ei_scales"] = scales
    r = beam.method_execute(spec)
    d = r["diagnostics"]
    assert r["verification"]["overall_status"] == "pass"
    kinds = [c["kind"] for c in r["verification"]["checks"]]
    assert "model_form" in kinds and "validation" not in kinds
    assert d["v_numerical"] > d["v_analytical"]
    # A uniform scale list must reproduce the uniform solution exactly.
    spec2 = _base_spec()
    spec2["ei_scales"] = [1.0] * 8
    r2 = beam.method_execute(spec2)
    assert r2["diagnostics"]["v_numerical"] == pytest.approx(
        d["v_analytical"], rel=1e-12)


def test_malformed_ei_scales_fail_validation() -> None:
    """Wrong length, non-positive, and non-numeric scales are rejected."""
    for bad in ([1.0] * 7, [0.0] * 8, [1.0, "x", 1.0, 1.0, 1.0, 1.0, 1.0, 1.0]):
        spec = _base_spec()
        spec["ei_scales"] = bad
        v = beam.method_validate(spec)
        assert not v["valid"]
        assert any(c["name"] == "ei_scales_wellformed" and c["result"] == "fail"
                   for c in v["checks"])
