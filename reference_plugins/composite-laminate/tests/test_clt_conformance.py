# SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
# SPDX-FileCopyrightText: 2026 Crocell Company
"""Conformance suite for the composite laminate (CLT) plugin."""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

import pytest

PLUGIN = Path(__file__).resolve().parents[1] / "python" / "stdio_plugin.py"

pytest.importorskip("numpy")


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


def _ply(angle: float, thickness: float = 0.125) -> dict:
    return {"thickness_mm": thickness, "angle_deg": angle}


def _problem(case: str, plies: list[dict]) -> dict:
    return {
        "problem_id": "clt-conformance",
        "physics_class": "structural",
        "case": case,
        "plies": plies,
    }


def test_discover_declares_real_physics() -> None:
    assert _call("discover")["result"]["is_real_physics"] is True


@pytest.mark.parametrize(
    "case,plies",
    [
        ("quasi_isotropic", [_ply(0), _ply(45), _ply(-45), _ply(90),
                            _ply(90), _ply(-45), _ply(45), _ply(0)]),
        ("unidirectional", [_ply(0), _ply(0), _ply(0), _ply(0)]),
        ("cross_ply", [_ply(0), _ply(90), _ply(90), _ply(0)]),
    ],
)
def test_matches_closed_form(case: str, plies: list[dict]) -> None:
    diagnostics = _call("execute", _problem(case, plies))["result"]["diagnostics"]
    assert diagnostics["passed"] is True, diagnostics
    assert diagnostics["relative_error"] <= 5e-2


def test_tsai_wu_index_is_reported() -> None:
    result = _call("execute", _problem("unidirectional", [_ply(0)]))["result"]
    assert "max_tsai_wu_index" in result["diagnostics"]


def test_validation_rejects_bad_case() -> None:
    response = _call("validate", _problem("delamination", [_ply(0)]))
    assert response["result"]["valid"] is False