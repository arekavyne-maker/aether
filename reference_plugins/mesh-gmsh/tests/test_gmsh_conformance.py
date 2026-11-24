# SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
# SPDX-FileCopyrightText: 2026 Crocell Company
"""Conformance suite for the real Gmsh meshing plugin."""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

import pytest

PLUGIN = Path(__file__).resolve().parents[1] / "python" / "stdio_plugin.py"

pytest.importorskip("gmsh", reason="Gmsh bindings not installed")


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


def test_discover_reports_real_mesh() -> None:
    result = _call("discover")["result"]
    assert result["is_real_mesh"] is True
    assert result["mesher"] == "Gmsh"


def test_generate_produces_tetrahedra() -> None:
    result = _call(
        "generate",
        {"dx": 10.0, "dy": 10.0, "dz": 10.0, "element_size_mm": 2.0},
    )["result"]
    assert result["n_elements"] > 0
    assert result["element_type"] == "tetra4"
    assert result["n_nodes"] > 0


def test_quality_metrics_are_reported() -> None:
    result = _call(
        "generate",
        {"dx": 10.0, "dy": 10.0, "dz": 10.0, "element_size_mm": 2.0},
    )["result"]
    assert result["min_scaled_jacobian"] > 0.0
    assert result["min_aspect_ratio"] >= 1.0
    assert result["quality_pass"] is True


def test_invalid_element_size_is_rejected() -> None:
    response = _call(
        "generate",
        {"dx": 10.0, "dy": 10.0, "dz": 10.0, "element_size_mm": 0.0},
    )
    assert "error" in response