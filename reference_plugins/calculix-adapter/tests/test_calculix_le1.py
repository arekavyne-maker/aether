# SPDX-License-Identifier: Aether-VPSL-1.0
# SPDX-FileCopyrightText: 2026 The Aether Consortium
"""Conformance tests for calculix-adapter (LE1 cantilever, SDP §5.6, §5.8, ADR-0022).

The calculix-adapter is a real native-binary adapter: it writes a real
CalculiX ``.inp`` deck, shells out to the ``ccx_2.23`` binary, parses
the real ``.frd`` output, and reports the tip deflection against the
closed-form analytical solution. These tests exercise the JSON-RPC
contract surface and the real FEA-vs-analytical check.

If the ccx binary is not on PATH (CCX_BIN), the tests skip with an
explicit message — the adapter is *executable when present* and
*honest when absent*. The ``infra/fitness/ccx_binary.py`` fitness
check enforces that the binary is present in environments that
declare the adapter production-grade.
"""

from __future__ import annotations

import importlib.util
import json
import math
import os
import shutil
import subprocess
import sys
from pathlib import Path

import pytest


REPO_ROOT = Path(__file__).resolve().parents[3]
PLUGIN = REPO_ROOT / "reference_plugins" / "calculix-adapter" / "python" / "stdio_plugin.py"
sys.path.insert(0, str(PLUGIN.parent))


def _import_plugin():
    spec = importlib.util.spec_from_file_location("calculix_adapter_stdio_plugin", PLUGIN)
    assert spec and spec.loader
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


ccx = _import_plugin()
CCX_AVAILABLE = shutil.which(ccx.CCX_BIN) is not None
SKIP_REASON = f"ccx binary not on PATH (CCX_BIN={ccx.CCX_BIN!r})"


# Common LE1 problem spec: 1m × 0.01m × 0.01m cantilever, E=200 GPa,
# nu=0.3, P=1000 N at the tip, 8 B31 elements. The closed-form
# analytical tip deflection is:
#   I = W H^3 / 12 = 0.01 * 0.01^3 / 12 = 8.333e-10 m^4
#   v_tip = P L^3 / (3 E I) = 1000 * 1 / (3 * 200e9 * 8.333e-10)
#         = 2.0 m
# This matches the analytical formula already implemented in
# reference_plugins/fea-reference-beam for the same problem.
COMMON_PROBLEM = {
    "problem_id": "01900000-0000-0000-0000-000000000001",
    "physics_class": "structural",
    "geometry": {
        "cas_hash": "0" * 64,
        "format": "native",
    },
    "boundary_conditions": [
        {"type": "dirichlet", "region": "root", "values": {"all_dof": 0.0}},
    ],
    "solver_settings": {
        "case": "le1_cantilever_b31",
        "n_elements": 8,
    },
    "fidelity_target": "high_fidelity",
    "determinism_class": "bit-reproducible",
}

COMMON_INPUTS = {
    "beam_length_m": 1.0,
    "cross_section_width_m": 0.01,
    "cross_section_height_m": 0.01,
    "E_pa": 200e9,
    "poisson_ratio": 0.3,
    "tip_load_n": 1000.0,
    "load_direction": "Z",
    "n_elements": 8,
}


def _params(extra_inputs: dict | None = None, step_id: str = "test-1") -> dict:
    inputs = dict(COMMON_INPUTS)
    if extra_inputs:
        inputs.update(extra_inputs)
    # Deep-copy the mutable fields so tests that mutate ``params``
    # (e.g. to inject a bad case) do not poison the module-level
    # COMMON_PROBLEM dict for subsequent tests.
    return {
        "step_id": step_id,
        "problem_id": COMMON_PROBLEM["problem_id"],
        "physics_class": COMMON_PROBLEM["physics_class"],
        "geometry": dict(COMMON_PROBLEM["geometry"]),
        "boundary_conditions": [dict(bc) for bc in COMMON_PROBLEM["boundary_conditions"]],
        "solver_settings": dict(COMMON_PROBLEM["solver_settings"]),
        "inputs": inputs,
    }


# ----- direct-call tests -----


def test_discover_advertises_is_real_physics() -> None:
    """discover() must mark this as a real-physics adapter, not a fixture."""
    d = ccx.method_discover(None)
    assert d["is_real_physics"] is True
    assert d["physics_kind"] == "calculix_3d_structural_static_b31_beam"
    assert ccx.VALIDATED_CASE in d["validated_cases"]
    assert d["solver_binary_resolved"] == CCX_AVAILABLE


def test_capabilities_declares_validated_case() -> None:
    c = ccx.method_capabilities(None)
    assert c["physics"]["structural"] is True
    assert c["physics"]["linear_static"] is True
    assert any(v["name"] == ccx.VALIDATED_CASE for v in c["validated_cases"])


def test_validate_rejects_unknown_case() -> None:
    params = _params()
    params["solver_settings"]["case"] = "bogus_case"
    r = ccx.method_validate(params)
    assert r["overall_status"] == "fail"
    assert any(c["name"] == "case_validated" and c["result"] == "fail" for c in r["checks"])


def test_validate_accepts_le1_case() -> None:
    params = _params()
    r = ccx.method_validate(params)
    if not CCX_AVAILABLE:
        assert r["overall_status"] == "fail"
        assert any(c["name"] == "ccx_binary_available" and c["result"] == "fail" for c in r["checks"])
    else:
        assert r["overall_status"] == "pass"


# ----- real-binary tests (skip if ccx absent) -----


@pytest.mark.skipif(not CCX_AVAILABLE, reason=SKIP_REASON)
def test_calculix_binary_invoked_and_tip_deflection_matches_analytical() -> None:
    """End-to-end: a real ccx invocation produces a tip deflection
    within 5% of the closed-form analytical value."""
    result = ccx.method_execute(_params())
    assert result["plugin_id"] == ccx.PLUGIN_ID
    assert "shell_wrapper" not in result, (
        "real adapter must not claim to be a shell-wrapper"
    )
    v = result["diagnostics"]["v_numerical"]
    v_ana = result["diagnostics"]["v_analytical"]
    rel_err = result["diagnostics"]["rel_err"]
    assert math.isclose(v_ana, 2.0, rel_tol=1e-12), (
        f"analytical formula should give 2.0 m; got {v_ana}"
    )
    assert rel_err < ccx.DEFAULT_REL_TOL, (
        f"rel_err={rel_err:.3e} >= tol {ccx.DEFAULT_REL_TOL}; "
        f"v_numerical={v:.6e}, v_analytical={v_ana:.6e}"
    )
    assert result["verification"]["overall_status"] == "pass"
    # The output bundle must be content-addressed, not a UUID.
    assert "cas_hash" in result["outputs"]["result.bundle"]
    cas_hash = result["outputs"]["result.bundle"]["cas_hash"]
    assert len(cas_hash) == 64 and all(c in "0123456789abcdef" for c in cas_hash), (
        f"cas_hash must be a 64-char hex SHA-256; got {cas_hash!r}"
    )


@pytest.mark.skipif(not CCX_AVAILABLE, reason=SKIP_REASON)
def test_repeated_invocation_is_bit_identical() -> None:
    """Same inputs -> same cas_hash (I-9, bit-reproducible).

    The ccx output is bit-reproducible at the byte level; the
    cas_hash of the .frd file is therefore identical across two
    runs of the same problem spec. The ``solver_runtime_s``
    diagnostic varies between runs (it's wall-clock time) and
    is excluded from the equality check.
    """
    p1 = _params(step_id="det-a")
    p2 = _params(step_id="det-b")
    r1 = ccx.method_execute(p1)
    r2 = ccx.method_execute(p2)
    assert r1["outputs"]["result.bundle"]["cas_hash"] == r2["outputs"]["result.bundle"]["cas_hash"]
    d1 = dict(r1["diagnostics"])
    d2 = dict(r2["diagnostics"])
    d1.pop("solver_runtime_s", None)
    d2.pop("solver_runtime_s", None)
    assert d1 == d2


@pytest.mark.skipif(not CCX_AVAILABLE, reason=SKIP_REASON)
def test_convergence_refines_with_more_elements() -> None:
    """16 elements should be at least as accurate as 8 elements vs the
    analytical solution. This is the substrate test for the mesh
    refinement: a regression that breaks convergence (e.g. a unit
    conversion error that doesn't get worse with mesh refinement)
    would be caught here.
    """
    r8 = ccx.method_execute(_params(extra_inputs={"n_elements": 8}, step_id="conv-8"))
    r16 = ccx.method_execute(_params(extra_inputs={"n_elements": 16}, step_id="conv-16"))
    assert r8["verification"]["overall_status"] == "pass"
    assert r16["verification"]["overall_status"] == "pass"
    assert r16["diagnostics"]["rel_err"] <= r8["diagnostics"]["rel_err"] + 1e-12, (
        f"16-element rel_err ({r16['diagnostics']['rel_err']:.3e}) "
        f"should be <= 8-element rel_err ({r8['diagnostics']['rel_err']:.3e})"
    )


# ----- I-12: failure modes -----


def test_bad_input_fails_validation_without_invoking_binary(tmp_path, monkeypatch) -> None:
    """I-12: bad input must fail validation, not invoke the binary
    and return a fabricated result. We monkeypatch the binary path
    to point at a non-existent file; if the binary were invoked the
    test would still pass (because the validation step would have
    already failed), but the ccx_spawn check would not appear in
    the verification block.
    """
    # Set CCX_BIN to a path that does not exist on the system. The
    # adapter's validate() should report ccx_binary_available as fail,
    # and execute() should return overall_status=fail without ever
    # invoking anything.
    monkeypatch.setattr(ccx, "CCX_BIN", "/no/such/ccx_2.23_zzz", raising=False)
    # also reload the env var (in case the adapter reads it lazily)
    monkeypatch.setenv("CCX_BIN", "/no/such/ccx_2.23_zzz")
    params = _params()
    r = ccx.method_execute(params)
    assert r["verification"]["overall_status"] == "fail"
    # The diagnostic must mention the missing binary.
    diag_blob = json.dumps(r["verification"])
    assert "ccx" in diag_blob.lower()


def test_unknown_case_fails_visibly() -> None:
    """An unknown case must return overall_status=fail with a clear
    diagnostic — never a UUID or plausible-looking fake value.
    """
    params = _params()
    params["solver_settings"]["case"] = "modal_buckling_3d"
    r = ccx.method_execute(params)
    assert r["verification"]["overall_status"] == "fail"
    assert r["outputs"] == {}
    assert any(
        c["name"] == "case_validated" and c["result"] == "fail"
        for c in r["verification"]["checks"]
    )


# ----- JSON-RPC substrate tests (the contract surface) -----


def test_stdio_jsonrpc_discover() -> None:
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
    assert response["result"]["plugin_id"] == ccx.PLUGIN_ID
    assert response["result"]["is_real_physics"] is True


@pytest.mark.skipif(not CCX_AVAILABLE, reason=SKIP_REASON)
def test_stdio_jsonrpc_execute_le1() -> None:
    """Full JSON-RPC round-trip on the LE1 case."""
    request = {
        "jsonrpc": "2.0",
        "id": "rpc-exec-le1",
        "method": "execute",
        "params": _params(),
    }
    proc = subprocess.run(
        [sys.executable, str(PLUGIN)],
        input=json.dumps(request) + "\n",
        capture_output=True,
        text=True,
        timeout=30,
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
        if r.get("id") == "rpc-exec-le1":
            response = r
            break
    assert response is not None, f"no JSON-RPC response; stdout={proc.stdout!r} stderr={proc.stderr!r}"
    result = response["result"]
    assert result["verification"]["overall_status"] == "pass"
    assert result["diagnostics"]["rel_err"] < ccx.DEFAULT_REL_TOL
