# SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
# SPDX-FileCopyrightText: 2026 Crocell Company
"""process-twin — reduced-order manufacturing process models (spec §9.2).

Implements deterministic, physics-grounded reduced models for:

- machining: cutting-force and deflection estimate
- composites layup: tow placement and porosity estimate
- additive: melt-pool width and porosity estimate from process parameters
- assembly: tolerance stack-up (RSS and worst case)

Each model declares its validity envelope and reports uncertainty. It does
not autonomously change machine parameters. Production use requires evidence.
"""

from __future__ import annotations

import hashlib
import json
import math
import sys
from typing import Any

PLUGIN_ID = "org.aether.reference.process.twin"
PLUGIN_VERSION = "0.1.0"
CONTRACT = "aether.manufacturing.process"

CASES = ("machining", "layup", "additive", "tolerance_stack")


def _ok(req_id: Any, result: dict[str, Any]) -> dict[str, Any]:
    return {"jsonrpc": "2.0", "id": req_id, "result": result}


def _err(req_id: Any, code: int, message: str) -> dict[str, Any]:
    return {"jsonrpc": "2.0", "id": req_id, "error": {"code": code, "message": message}}


def method_discover(_params: dict[str, Any] | None) -> dict[str, Any]:
    return _ok(None, {
        "plugin_id": PLUGIN_ID,
        "plugin_version": PLUGIN_VERSION,
        "contract_id": CONTRACT,
        "is_real_physics": True,
        "cases": list(CASES),
        "fidelity_class": "low_fidelity",
        "autonomous_machine_control": False,
        "note": "Reduced-order process models with declared envelopes. No machine control.",
    })


def method_capabilities(_params: dict[str, Any] | None) -> dict[str, Any]:
    return _ok(None, {
        "plugin_id": PLUGIN_ID,
        "capabilities": list(CASES),
        "limits": ["reduced-order estimates", "no in-situ sensing"],
        "supported_formats": ["native-json"],
    })


def method_validate(params: dict[str, Any] | None) -> dict[str, Any]:
    if not isinstance(params, dict):
        return _ok(None, {"valid": False, "checks": []})
    case = params.get("case")
    valid = case in CASES
    return _ok(None, {
        "valid": valid,
        "checks": [{"name": "case", "result": "pass" if valid else "fail"}],
        "overall_status": "pass" if valid else "fail",
    })


def _machining(p: dict[str, Any]) -> dict[str, Any]:
    # Orthogonal cutting force: Fc = kc * uncut_chip_area, with a
    # deflection correction for a slender tool.
    kc = float(p.get("specific_force_pa", 2.0e9))
    doc = float(p.get("depth_of_cut_mm", 1.0)) / 1000.0
    feed = float(p.get("feed_mm", 0.1)) / 1000.0
    force = kc * doc * feed
    tool_len = float(p.get("tool_overhang_mm", 30.0)) / 1000.0
    E = float(p.get("tool_E_pa", 600e9))
    I = float(p.get("tool_I_m4", 1e-9))
    deflection = force * tool_len**3 / (3.0 * E * I)
    return {
        "case": "machining",
        "cutting_force_n": force,
        "tool_deflection_mm": deflection * 1000.0,
        "envelope": "linear elastic tool, continuous chip",
        "uncertainty": "specific force varies with material and tool wear",
    }


def _layup(p: dict[str, Any]) -> dict[str, Any]:
    # AFP tow placement: coverage and porosity estimate from tow width,
    # spacing and speed.
    tow = float(p.get("tow_width_mm", 6.35))
    spacing = float(p.get("spacing_mm", 6.5))
    speed = float(p.get("speed_mm_s", 100.0))
    gap = max(0.0, spacing - tow)
    overlap_fraction = max(0.0, (tow - spacing) / tow)
    porosity = min(0.2, gap / max(spacing, 1e-9) + 0.01 * speed / 100.0)
    return {
        "case": "layup",
        "gap_mm": gap,
        "overlap_fraction": overlap_fraction,
        "porosity_fraction": porosity,
        "envelope": "straight tow paths, constant speed",
        "uncertainty": "steering and tack not modelled",
    }


def _additive(p: dict[str, Any]) -> dict[str, Any]:
    # Melt-pool width from laser power and scan speed (Rosenthal-like scaling).
    power = float(p.get("laser_power_w", 200.0))
    speed = float(p.get("scan_speed_mm_s", 800.0))
    absorptivity = float(p.get("absorptivity", 0.35))
    width = 2.0 * math.sqrt(absorptivity * power / (math.pi * speed * 1000.0))
    porosity = min(0.1, 0.02 * abs(speed - 800.0) / 200.0)
    return {
        "case": "additive",
        "melt_pool_width_mm": width * 1000.0,
        "porosity_fraction": porosity,
        "envelope": "conduction mode, no keyholing",
        "uncertainty": "powder and hatch spacing not modelled",
    }


def _tolerance_stack(p: dict[str, Any]) -> dict[str, Any]:
    tolerances = [float(v) for v in p.get("tolerances_mm", [])]
    if not tolerances:
        raise ValueError("tolerances_mm required")
    worst = sum(tolerances)
    rss = math.sqrt(sum(t * t for t in tolerances))
    return {
        "case": "tolerance_stack",
        "worst_case_mm": worst,
        "rss_mm": rss,
        "n_dimensions": len(tolerances),
        "envelope": "independent normal tolerances for RSS",
        "uncertainty": "correlations not modelled",
    }


SOLVERS = {
    "machining": _machining,
    "layup": _layup,
    "additive": _additive,
    "tolerance_stack": _tolerance_stack,
}


def method_simulate(params: dict[str, Any] | None) -> dict[str, Any]:
    if not isinstance(params, dict):
        return _err(None, -32602, "params must be an object")
    if params.get("case") not in CASES:
        return _err(None, -32602, "unknown case")
    try:
        result = SOLVERS[params["case"]](params)
    except Exception as exc:  # noqa: BLE001
        return _err(None, -32000, f"simulation failed: {exc}")
    result["autonomous_machine_control"] = False
    result["weights_hash"] = hashlib.sha256(
        json.dumps(result, sort_keys=True).encode()
    ).hexdigest()
    return _ok(None, {
        "plugin_id": PLUGIN_ID,
        "diagnostics": result,
        "outputs": {"process.bundle": {"cas_hash": result["weights_hash"], "case": params["case"]}},
    })


METHODS = {
    "discover": method_discover,
    "capabilities": method_capabilities,
    "validate": method_validate,
    "simulate": method_simulate,
}


def main() -> None:
    for raw in sys.stdin:
        raw = raw.strip()
        if not raw:
            continue
        try:
            req = json.loads(raw)
        except json.JSONDecodeError:
            continue
        method = req.get("method")
        params = req.get("params")
        req_id = req.get("id")
        handler = METHODS.get(method)
        if handler is None:
            resp = _err(req_id, -32601, f"unknown method: {method}")
        else:
            try:
                resp = handler(params)
            except Exception as exc:  # noqa: BLE001
                resp = _err(req_id, -32000, f"handler error: {exc}")
        if isinstance(resp, dict) and resp.get("id", "missing") is None:
            resp["id"] = req_id
        sys.stdout.write(json.dumps(resp) + "\n")
        sys.stdout.flush()


if __name__ == "__main__":
    main()