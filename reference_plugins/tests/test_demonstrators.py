# SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
# SPDX-FileCopyrightText: 2026 Crocell Company
"""Conformance for Demonstrators C and E."""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]


def _call(plugin: str, method: str, params: dict | None = None) -> dict:
    proc = subprocess.run(
        [sys.executable, str(ROOT / plugin / "python" / "stdio_plugin.py")],
        input=json.dumps({"jsonrpc": "2.0", "id": "t", "method": method, "params": params}) + "\n",
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
    raise AssertionError(f"no response from {plugin}: {proc.stdout!r} {proc.stderr!r}")


@pytest.mark.parametrize(
    "case", ["multi_state_geometry", "compliant_stiffness", "aero_structural", "fatigue_cycles"]
)
def test_demonstrator_c(case: str) -> None:
    result = _call("morphing-te", "execute", {"case": case})["result"]["diagnostics"]
    assert result["demonstrator"] == "C"
    assert result["validation_fixture"] == "synthetic"
    assert result["cas_hash"]


@pytest.mark.parametrize(
    "case", ["temperature_dependent", "thermal_gradient", "thermal_structural"]
)
def test_demonstrator_e(case: str) -> None:
    result = _call("high-temp-panel", "execute", {"case": case})["result"]["diagnostics"]
    assert result["demonstrator"] == "E"
    assert result["validation_fixture"] == "synthetic"
    assert result["cas_hash"]


def test_discovery_declares_validation_plan() -> None:
    c = _call("morphing-te", "discover")["result"]
    e = _call("high-temp-panel", "discover")["result"]
    assert c["validation_plan"].endswith(".md")
    assert e["validation_plan"].endswith(".md")
    assert e["exclusion"].startswith("atmospheric")
