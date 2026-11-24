# SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
# SPDX-FileCopyrightText: 2026 Crocell Company
"""Conformance suite for the real OpenCASCADE geometry plugin.

These tests assert real B-rep behaviour: exact volumes, real booleans,
real fillets, and a STEP round-trip that preserves the geometric
signature. If the OCP bindings are missing, the tests skip rather than
pretend the fixture is real.
"""

from __future__ import annotations

import json
import math
import subprocess
import sys
from pathlib import Path

import pytest

PLUGIN = Path(__file__).resolve().parents[1] / "python" / "stdio_plugin.py"

OCP = pytest.importorskip("OCP", reason="OpenCASCADE bindings not installed")


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


def test_discover_reports_real_brep() -> None:
    result = _call("discover")["result"]
    assert result["is_real_brep"] is True
    assert result["geometry_kernel"] == "OpenCASCADE"
    assert result["geometry_kernel_version"] != "unavailable"


def test_check_validates_a_box() -> None:
    result = _call("check", {"dx": 10.0, "dy": 10.0, "dz": 10.0})["result"]
    assert result["valid"] is True
    assert result["counts"]["solids"] == 1
    assert result["counts"]["faces"] == 6


def test_boolean_volumes_are_exact() -> None:
    base = {"dx": 1.0, "dy": 1.0, "dz": 1.0}
    union = _call("boolean", {"op": "union", "a": base, "b": base})["result"]
    inter = _call("boolean", {"op": "intersect", "a": base, "b": base})["result"]
    sub = _call("boolean", {"op": "subtract", "a": base, "b": base})["result"]
    assert math.isclose(union["volume_m3"], 1.875, rel_tol=1e-9)
    assert math.isclose(inter["volume_m3"], 0.125, rel_tol=1e-9)
    assert math.isclose(sub["volume_m3"], 0.875, rel_tol=1e-9)


def test_fillet_changes_the_volume() -> None:
    plain = _call("check", {"dx": 10.0, "dy": 10.0, "dz": 10.0})["result"]
    filleted = _call("fillet", {"dx": 10.0, "dy": 10.0, "dz": 10.0, "radius_mm": 1.0})["result"]
    assert filleted["edges_filleted"] > 0
    # A fillet removes material, so the volume must drop.
    assert filleted["volume_m3"] < 1000.0


def test_tessellate_produces_triangles() -> None:
    result = _call("tessellate", {"dx": 10.0, "dy": 10.0, "dz": 10.0, "max_chord_mm": 0.1})["result"]
    assert result["triangles"] > 0
    assert result["vertices"] > 0


def test_step_round_trip_preserves_signature(tmp_path: Path) -> None:
    step = tmp_path / "box.step"
    exported = _call(
        "export",
        {"path": str(step), "target_format": "step-ap242", "dx": 10.0, "dy": 20.0, "dz": 30.0},
    )["result"]
    assert step.is_file()
    assert exported["is_real_brep"] is True

    imported = _call("import", {"path": str(step), "format": "step-ap242"})["result"]
    assert imported["is_real_brep"] is True
    # The round-trip must preserve the volume and the face count.
    assert math.isclose(imported["volume_m3"], 6000.0, rel_tol=1e-6)
    assert imported["counts"]["faces"] == 6


def test_import_rejects_garbage(tmp_path: Path) -> None:
    bad = tmp_path / "bad.step"
    bad.write_text("this is not a STEP file")
    response = _call("import", {"path": str(bad), "format": "step-ap242"})
    assert "error" in response