# SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
# SPDX-FileCopyrightText: 2026 Crocell Company
"""Conformance suite for the sparse beam-FEM reference plugin."""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

import pytest

PLUGIN = Path(__file__).resolve().parents[1] / "python" / "stdio_plugin.py"


def _call(method: str, params: dict | None = None) -> dict:
    request = {"jsonrpc": "2.0", "id": "t", "method": method, "params": params}
    proc = subprocess.run(
        [sys.executable, str(PLUGIN)],
        input=json.dumps(request) + "\n",
        capture_output=True,
        text=True,
        check=False,
    )
    for line in proc.stdout.splitlines():
        line = line.strip()
        if not line:
            continue
        response = json.loads(line)
        if response.get("id") == "t":
            return response
    raise AssertionError(f"no response from plugin: {proc.stdout!r} {proc.stderr!r}")


def _problem(case: str, n: int = 8, **kw) -> dict:
    p = {
        "problem_id": "spar-conformance",
        "physics_class": "structural",
        "length_m": 2.0,
        "E_pa": 210e9,
        "nu": 0.3,
        "area_m2": 0.01,
        "Iyy_m4": 8.333e-6,
        "Izz_m4": 8.333e-6,
        "J_m4": 1.6e-5,
        "case": case,
        "n_elements": n,
    }
    p.update(kw)
    return p


def test_discover_declares_real_physics() -> None:
    result = _call("discover")["result"]
    assert result["is_real_physics"] is True
    assert result["plugin_id"] == "org.aether.reference.fe.spar"


@pytest.mark.parametrize(
    "case,kw",
    [
        ("cantilever_tip_point", {"P_n": 1000.0}),
        ("ss_center_point", {"P_n": 1000.0}),
        ("ss_uniform", {"q_n_per_m": 1000.0}),
        ("axial_end_load", {"P_n": 1000.0}),
    ],
)
def test_matches_closed_form(case: str, kw: dict) -> None:
    response = _call("execute", _problem(case, 8, **kw))
    assert "result" in response, response
    result = response["result"]
    diagnostics = result["diagnostics"]
    assert diagnostics["passed"] is True, diagnostics
    assert diagnostics["relative_error"] <= 1e-2, diagnostics


def test_validation_rejects_bad_case() -> None:
    response = _call("validate", _problem("buckling", 4))
    assert response["result"]["valid"] is False


def test_unknown_method_is_an_error() -> None:
    response = _call("does_not_exist", {})
    assert "error" in response
    assert response["error"]["code"] == -32601