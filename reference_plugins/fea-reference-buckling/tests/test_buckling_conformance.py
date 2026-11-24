# SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
# SPDX-FileCopyrightText: 2026 Crocell Company
"""Conformance suite for the linear buckling reference plugin."""

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


def _problem(case: str, n: int = 16) -> dict:
    return {
        "problem_id": "buckling-conformance",
        "physics_class": "structural",
        "length_m": 2.0,
        "E_pa": 210e9,
        "area_m2": 0.01,
        "I_m4": 8.333e-6,
        "case": case,
        "n_elements": n,
    }


def test_discover_declares_real_physics() -> None:
    assert _call("discover")["result"]["is_real_physics"] is True


@pytest.mark.parametrize("case", ["euler_pinned", "euler_cantilever"])
def test_matches_euler(case: str) -> None:
    diagnostics = _call("execute", _problem(case))["result"]["diagnostics"]
    assert diagnostics["passed"] is True, diagnostics
    assert diagnostics["relative_error"] <= 2e-2


def test_validation_rejects_bad_case() -> None:
    assert _call("validate", _problem("post_buckling"))["result"]["valid"] is False