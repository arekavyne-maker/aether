# SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
# SPDX-FileCopyrightText: 2026 Crocell Company
"""Conformance suite for the process twin plugin."""

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


@pytest.mark.parametrize(
    "case,extra",
    [
        ("machining", {}),
        ("layup", {}),
        ("additive", {}),
        ("tolerance_stack", {"tolerances_mm": [0.1, 0.2, 0.15]}),
    ],
)
def test_process_model_runs(case: str, extra: dict) -> None:
    params = {"case": case}
    params.update(extra)
    result = _call("simulate", params)["result"]
    assert result["diagnostics"]["autonomous_machine_control"] is False
    assert result["diagnostics"]["weights_hash"]


def test_unknown_case_is_rejected() -> None:
    assert "error" in _call("simulate", {"case": "telepathy"})


def test_tolerance_stack_math() -> None:
    result = _call("simulate", {
        "case": "tolerance_stack", "tolerances_mm": [0.1, 0.2, 0.15]
    })["result"]["diagnostics"]
    assert abs(result["worst_case_mm"] - 0.45) < 1e-9
