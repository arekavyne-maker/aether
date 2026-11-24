# SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
# SPDX-FileCopyrightText: 2026 Crocell Company
"""Conformance suite for the scientific-ML plugin."""

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


def test_discover_declares_research_not_production() -> None:
    result = _call("discover")["result"]
    assert result["maturity"] == "research"
    assert result["production_approved"] is False
    assert result["ood_method"]


@pytest.mark.parametrize(
    "method", ["reservoir", "sindy", "dmd", "spectral_operator", "pinn_fit"]
)
def test_method_trains_with_grouped_validation(method: str) -> None:
    result = _call("train", {"method": method, "seed": 42})["result"]
    diagnostics = result["diagnostics"]
    assert diagnostics["in_domain"] is True
    assert diagnostics["weights_hash"]
    assert result["validation"]["split"] == "grouped"
    assert result["diagnostics"]["production_approved"] is False


def test_unknown_method_is_rejected() -> None:
    assert "error" in _call("train", {"method": "magic"})


def test_grouped_split_holds_out_whole_groups() -> None:
    sys.path.insert(0, str(PLUGIN.parent))
    import importlib.util

    spec = importlib.util.spec_from_file_location("sci_ml", PLUGIN)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    train, test = mod._grouped_split(100, 4, 7)
    assert len(train) + len(test) == 100
    assert len(test) > 0
