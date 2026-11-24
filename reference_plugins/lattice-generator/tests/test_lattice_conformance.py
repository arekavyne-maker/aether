# SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
# SPDX-FileCopyrightText: 2026 Crocell Company
"""Conformance suite for the lattice generator plugin."""

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
    raise AssertionError(f"no response: {proc.stdout!r} {proc.stderr!r}")


def _problem(case: str) -> dict:
    return {
        "problem_id": "lattice-conformance",
        "physics_class": "structural",
        "case": case,
        "cell_size_mm": 10.0,
        "strut_diameter_mm": 1.0,
    }


def test_discover_declares_real_physics() -> None:
    assert _call("discover")["result"]["is_real_physics"] is True


@pytest.mark.parametrize(
    "case", ["simple_cubic", "body_centered_cubic", "octet_truss"]
)
def test_generates_a_lattice(case: str) -> None:
    result = _call("execute", _problem(case))["result"]["diagnostics"]
    assert result["n_nodes"] > 0
    assert result["n_struts"] > 0
    assert 0.0 < result["relative_density"] < 1.0
    assert result["passed"] is True


def test_relative_density_is_reproducible() -> None:
    a = _call("execute", _problem("octet_truss"))["result"]["diagnostics"]
    b = _call("execute", _problem("octet_truss"))["result"]["diagnostics"]
    assert a["relative_density"] == b["relative_density"]


def test_validation_rejects_bad_case() -> None:
    response = _call("validate", _problem("bending_only"))
    assert response["result"]["valid"] is False