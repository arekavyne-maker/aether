# SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
# SPDX-FileCopyrightText: 2026 Crocell Company
"""high-temp-panel — Demonstrator E: atmospheric high-temperature panel.

Real models for:
- temperature-dependent material properties (linear interpolation)
- through-thickness thermal gradient and heat flux
- coupled thermal-structural stress
- oxidation/environment assumption with a declared envelope
- manufacturing route and inspection notes
- model uncertainty statement

Validation uses synthetic fixtures until measured coupon data exists.

Contract: aether.physics.solver
Plugin:   org.aether.reference.high_temp.panel
"""

from __future__ import annotations

import hashlib
import json
import sys
from typing import Any

PLUGIN_ID = "org.aether.reference.high_temp.panel"
PLUGIN_VERSION = "0.1.0"
CONTRACT = "aether.physics.solver"

CASES = ("temperature_dependent", "thermal_gradient", "thermal_structural")


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
        "demonstrator": "E",
        "cases": list(CASES),
        "maturity": "synthetic_fixture_validation_plan",
        "validation_plan": "docs/verification/demonstrator-e-validation-plan.md",
        "exclusion": "atmospheric only; no space or launch structures",
        "note": "Real thermal-structural models with a synthetic-fixture validation plan.",
    })


def method_capabilities(_params: dict[str, Any] | None) -> dict[str, Any]:
    return _ok(None, {
        "plugin_id": PLUGIN_ID,
        "capabilities": list(CASES),
        "limits": ["no oxidation kinetics model", "linear elastic structure"],
        "supported_formats": ["native-json"],
    })


def method_validate(params: dict[str, Any] | None) -> dict[str, Any]:
    valid = isinstance(params, dict) and params.get("case") in CASES
    return _ok(None, {
        "valid": valid,
        "checks": [{"name": "case", "result": "pass" if valid else "fail"}],
        "overall_status": "pass" if valid else "fail",
    })


def _interpolate(points: list[dict[str, float]], temperature: float) -> float:
    pts = sorted(points, key=lambda p: p["temperature_c"])
    if temperature < pts[0]["temperature_c"] or temperature > pts[-1]["temperature_c"]:
        raise ValueError("temperature outside material envelope")
    for a, b in zip(pts, pts[1:]):
        if a["temperature_c"] <= temperature <= b["temperature_c"]:
            span = b["temperature_c"] - a["temperature_c"]
            t = 0.0 if span == 0 else (temperature - a["temperature_c"]) / span
            return a["value"] + t * (b["value"] - a["value"])
    return pts[-1]["value"]


def _temperature_dependent(p: dict[str, Any]) -> dict[str, Any]:
    points = p.get("property_points", [
        {"temperature_c": 23.0, "value": 200e9},
        {"temperature_c": 800.0, "value": 120e9},
    ])
    temperature = float(p.get("temperature_c", 400.0))
    value = _interpolate(points, temperature)
    return {
        "case": "temperature_dependent",
        "temperature_c": temperature,
        "property_value": value,
        "basis": "interpolated between qualification points",
    }


def _thermal_gradient(p: dict[str, Any]) -> dict[str, Any]:
    k = float(p.get("k_w_m_k", 20.0))
    hot = float(p.get("hot_c", 900.0))
    cold = float(p.get("cold_c", 100.0))
    thickness = float(p.get("thickness_mm", 10.0)) / 1000.0
    flux = k * (hot - cold) / thickness
    return {
        "case": "thermal_gradient",
        "hot_c": hot,
        "cold_c": cold,
        "heat_flux_w_m2": flux,
        "gradient_c_per_m": (hot - cold) / thickness,
        "oxidation_assumption": "no active oxidation below 900 C in declared atmosphere",
    }


def _thermal_structural(p: dict[str, Any]) -> dict[str, Any]:
    alpha = float(p.get("alpha_per_k", 12e-6))
    E = float(p.get("E_pa", 160e9))
    hot = float(p.get("hot_c", 800.0))
    cold = float(p.get("cold_c", 100.0))
    nu = float(p.get("nu", 0.3))
    # Fully restrained thermal stress from through-thickness gradient.
    stress = E * alpha * (hot - cold) / (1.0 - nu)
    strain = alpha * (hot - cold)
    return {
        "case": "thermal_structural",
        "thermal_stress_pa": stress,
        "free_thermal_strain": strain,
        "restrained": True,
        "manufacturing_route": "superplastic forming + diffusion bond (declared)",
        "inspection": "through-transmission ultrasonic after forming",
        "uncertainty": "oxidation and creep not modelled; stress is an upper bound",
    }


SOLVERS = {
    "temperature_dependent": _temperature_dependent,
    "thermal_gradient": _thermal_gradient,
    "thermal_structural": _thermal_structural,
}


def method_execute(params: dict[str, Any] | None) -> dict[str, Any]:
    if not isinstance(params, dict):
        return _err(None, -32602, "params must be an object")
    case = params.get("case")
    if case not in SOLVERS:
        return _err(None, -32602, f"unknown case {case!r}")
    try:
        result = SOLVERS[case](params)
    except Exception as exc:  # noqa: BLE001
        return _err(None, -32000, f"execution failed: {exc}")
    result["demonstrator"] = "E"
    result["validation_fixture"] = "synthetic"
    result["cas_hash"] = hashlib.sha256(
        json.dumps(result, sort_keys=True).encode()
    ).hexdigest()
    return _ok(None, {
        "plugin_id": PLUGIN_ID,
        "diagnostics": result,
        "outputs": {"result.bundle": {"cas_hash": result["cas_hash"], "case": case}},
    })


METHODS = {
    "discover": method_discover,
    "capabilities": method_capabilities,
    "validate": method_validate,
    "execute": method_execute,
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
        handler = METHODS.get(method)
        req_id = req.get("id")
        if handler is None:
            resp = _err(req_id, -32601, f"unknown method: {method}")
        else:
            try:
                resp = handler(req.get("params"))
            except Exception as exc:  # noqa: BLE001
                resp = _err(req_id, -32000, f"handler error: {exc}")
        if isinstance(resp, dict) and resp.get("id", "missing") is None:
            resp["id"] = req_id
        sys.stdout.write(json.dumps(resp) + "\n")
        sys.stdout.flush()


if __name__ == "__main__":
    main()