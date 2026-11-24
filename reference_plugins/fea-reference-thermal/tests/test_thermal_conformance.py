# SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
# SPDX-FileCopyrightText: 2026 Crocell Company
"""Conformance suite for the steady thermal reference plugin."""

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


def _problem(case: str, n: int = 8, **kw) -> dict:
    p = {
        "problem_id": "thermal-conformance",
        "physics_class": "thermal",
        "length_m": 1.0,
        "k_w_per_m_k": 50.0,
        "case": case,
        "n_elements": n,
        "T_left_c": 100.0,
        "T_right_c": 0.0,
    }
    p.update(kw)
    return p


def test_discover_declares_real_physics() -> None:
    result = _call("discover")["result"]
    assert result["is_real_physics"] is True


@pytest.mark.parametrize("case,kw", [
    ("rod_no_source", {}),
    ("rod_uniform_source", {"q_w_per_m3": 1e6}),
])
def test_matches_closed_form(case: str, kw: dict) -> None:
    response = _call("execute", _problem(case, 8, **kw))
    diagnostics = response["result"]["diagnostics"]
    assert diagnostics["passed"] is True, diagnostics
    assert diagnostics["relative_error"] <= 2e-2


def test_validation_rejects_bad_case() -> None:
    assert _call("validate", _problem("radiation"))["result"]["valid"] is False