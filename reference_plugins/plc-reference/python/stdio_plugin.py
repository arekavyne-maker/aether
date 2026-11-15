# SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
# SPDX-FileCopyrightText: 2026 Crocell Company
"""Manufacturing, inspection, and test reference plugins.

A single conformance fixture that implements all three G6 contracts
(manufacturing.process, inspection.device, test.rig) so the
closed-loop AWL can exercise the full physical-to-digital surface
in one Python process. Each contract is routed by the method name
in the JSON-RPC request:

  - aether.manufacturing.process: plan / simulate / export_program / record_as_run
  - aether.inspection.device: device_profile / acquire / register / export_measurements
  - aether.test.rig: rig_profile / configure / run / stream_telemetry / collect

The reference plugin is honest about being a stub: it returns
content-hashed synthetic outputs so the conformance tests can
verify protocol invariants and CAS stability without needing
real machines.
"""

from __future__ import annotations

import hashlib
import json
import math
import sys
import uuid
from typing import Any

PLUGIN_ID = "org.aether.reference.manufacturing.baseline"
PLUGIN_VERSION = "0.1.0"
CONTRACT = "aether.manufacturing.process"


def _ok(req_id: Any, result: dict[str, Any]) -> dict[str, Any]:
    return {"jsonrpc": "2.0", "id": req_id, "result": result}


def _err(req_id: Any, code: int, message: str) -> dict[str, Any]:
    return {"jsonrpc": "2.0", "id": req_id, "error": {"code": code, "message": message}}


def _cas_hash(payload: dict[str, Any]) -> str:
    s = json.dumps(payload, sort_keys=True, separators=(",", ":"))
    return hashlib.sha256(s.encode("utf-8")).hexdigest()


def method_init(params: dict[str, Any] | None) -> dict[str, Any]:
    if not isinstance(params, dict):
        return {"ok": False, "reason": "params must be an object"}
    for field in ("token_id", "workflow_id", "step_id", "scope", "signature_b64"):
        if field not in params:
            return {"ok": False, "reason": f"missing field: {field}"}
    return {"ok": True, "plugin_id": PLUGIN_ID, "plugin_version": PLUGIN_VERSION}


def method_discover(_params: dict[str, Any] | None) -> dict[str, Any]:
    return {
        "plugin_id": PLUGIN_ID,
        "plugin_version": PLUGIN_VERSION,
        "contract_id": "aether.manufacturing.process",
        "contract_version_range": ">=0.1.0 <1.0.0",
        "kind": "reference",
        "determinism_class": "bit-reproducible",
        "machines": ["3_axis_cnc", "5_axis_cnc", "l_pbf", "ded", "afp", "atl", "autoclave", "trim"],
        "note": "Phase 6 reference manufacturing plugin. Combined with inspection + test references.",
    }


def method_capabilities(_params: dict[str, Any] | None) -> dict[str, Any]:
    return {
        "machines": ["3_axis_cnc", "5_axis_cnc", "l_pbf", "ded", "afp", "atl", "autoclave", "trim"],
        "post_dialects": ["G-code", "CL", "MTConnect"],
    }


# -- Manufacturing: plan / simulate / export_program / record_as_run --

def method_plan(params: dict[str, Any] | None) -> dict[str, Any]:
    if not params or "inputs" not in params:
        return {"error": "no inputs"}
    p = params["inputs"]
    geom = p.get("geometry_ref", {})
    machine = p.get("machine", {})
    program = {
        "name": "manufacturing-program",
        "geometry_ref": {"cas_hash": geom.get("cas_hash", "")},
        "machine": machine,
        "steps": [
            {"sequence_index": 0, "operation": "mill", "parameters": {"feed_mm_min": 800, "spindle_rpm": 12000}, "acceptance_criteria": {"tol_mm": 0.05}},
            {"sequence_index": 1, "operation": "fillet", "parameters": {"radius_mm": 1.0}, "acceptance_criteria": {}},
            {"sequence_index": 2, "operation": "scan", "parameters": {"resolution_mm": 0.1}, "acceptance_criteria": {}},
        ],
    }
    return {
        "plugin_id": PLUGIN_ID,
        "plugin_version": PLUGIN_VERSION,
        "determinism_class": "bit-reproducible",
        "outputs": {
            "program": {
                "cas_hash": _cas_hash(program),
                "name": "program.json",
                "size": 0,
                "format": "json",
            }
        },
        "result": program,
    }


def method_simulate(_params: dict[str, Any] | None) -> dict[str, Any]:
    # Conformance fixture: return a placeholder distortion map.
    return {
        "ok": True,
        "distortion_mm": {"max": 0.4, "mean": 0.05},
        "build_time_h": 4.2,
    }


def method_export_program(_params: dict[str, Any] | None) -> dict[str, Any]:
    return {"ok": True, "dialect": "G-code", "lines": 1024}


def method_record_as_run(_params: dict[str, Any] | None) -> dict[str, Any]:
    return {"ok": True, "recorded_at": "2026-08-31T00:00:00Z"}


# -- Inspection: device_profile / acquire / register / export_measurements --

INSPECTION_PLUGIN = "org.aether.reference.inspection.cmm"
INSPECTION_CONTRACT = "aether.inspection.device"


def method_device_profile(_params: dict[str, Any] | None) -> dict[str, Any]:
    return {
        "plugin_id": INSPECTION_PLUGIN,
        "plugin_version": PLUGIN_VERSION,
        "contract_id": INSPECTION_CONTRACT,
        "kind": "cmm",
        "channels": 1,
        "accuracy_mm": 0.001,
    }


def _acquire_cmm(geometry_hash: str, n_points: int = 200) -> dict[str, Any]:
    """Generate synthetic CMM points around a nominal rectangle."""
    points: list[dict[str, Any]] = []
    for i in range(n_points):
        x = -0.1 + 0.2 * (i % 20) / 20.0
        y = -0.05 + 0.1 * (i // 20) / 10.0
        z = math.sin(x * 5) * 0.001 + 0.0005  # small deviation
        points.append({"x": x, "y": y, "z": z, "deviation_mm": abs(z) * 1000})
    measurements = {
        "device_kind": "cmm",
        "geometry_ref": {"cas_hash": geometry_hash},
        "points": points,
        "summary": {
            "n_points": len(points),
            "max_deviation_mm": max(p["deviation_mm"] for p in points),
            "mean_deviation_mm": sum(p["deviation_mm"] for p in points) / len(points),
            "rms_deviation_mm": math.sqrt(sum(p["deviation_mm"] ** 2 for p in points) / len(points)),
            "coverage_pct": 0.987,
            "passed": True,
        },
    }
    measurements["cas_hash"] = _cas_hash(measurements)
    return measurements


def method_acquire(params: dict[str, Any] | None) -> dict[str, Any]:
    if not params or "inputs" not in params:
        return {"error": "no inputs"}
    p = params["inputs"]
    geom = p.get("geometry_ref", {}).get("cas_hash", "")
    measurements = _acquire_cmm(geom)
    return {
        "plugin_id": INSPECTION_PLUGIN,
        "plugin_version": PLUGIN_VERSION,
        "determinism_class": "bit-reproducible",
        "outputs": {
            "inspection": {
                "cas_hash": measurements["cas_hash"],
                "name": "inspection.cmm",
                "size": 0,
                "format": "json",
            }
        },
        "result": measurements,
    }


def method_register(_params: dict[str, Any] | None) -> dict[str, Any]:
    return {"ok": True, "registered_at": "2026-08-31T00:00:00Z"}


def method_export_measurements(_params: dict[str, Any] | None) -> dict[str, Any]:
    return {"ok": True, "format": "json"}


# -- Test: rig_profile / configure / run / stream_telemetry / collect --

TEST_PLUGIN = "org.aether.reference.test.daq"
TEST_CONTRACT = "aether.test.rig"


def method_rig_profile(_params: dict[str, Any] | None) -> dict[str, Any]:
    return {
        "plugin_id": TEST_PLUGIN,
        "plugin_version": PLUGIN_VERSION,
        "contract_id": TEST_CONTRACT,
        "kind": "servo_hydraulic",
        "channels": 64,
        "max_sampling_rate_hz": 10000.0,
    }


def method_configure(_params: dict[str, Any] | None) -> dict[str, Any]:
    return {"ok": True}


def method_run(params: dict[str, Any] | None) -> dict[str, Any]:
    if not params or "inputs" not in params:
        return {"error": "no inputs"}
    p = params["inputs"]
    channels = p.get("rig", {}).get("channels", 64)
    rate = p.get("rig", {}).get("max_sampling_rate_hz", 1000.0)
    duration = p.get("load_profile", {}).get("duration_s", 60.0)
    n = max(1, int(rate * duration / 10))  # 10x downsampled for the fixture
    import time as _t
    chans: list[dict[str, Any]] = []
    for c in range(min(channels, 4)):  # conformance fixture uses 4 channels
        samples = [0.0001 * i * math.sin(2 * math.pi * 5 * (i / rate)) for i in range(n)]
        rms = math.sqrt(sum(s * s for s in samples) / max(n, 1))
        chans.append({
            "id": f"ch{c}",
            "name": f"channel_{c}",
            "unit": "V",
            "samples": n,
            "sampling_rate_hz": rate,
            "summary": {
                "min": min(samples),
                "max": max(samples),
                "mean": sum(samples) / max(n, 1),
                "rms": rms,
            },
        })
    run = {
        "rig_id": p.get("rig", {}).get("id", "unknown"),
        "started_at": "2026-08-31T00:00:00Z",
        "ended_at": "2026-08-31T00:01:00Z",
        "status": "succeeded",
        "channels": chans,
    }
    return {
        "plugin_id": TEST_PLUGIN,
        "plugin_version": PLUGIN_VERSION,
        "determinism_class": "bit-reproducible",
        "outputs": {
            "test_run": {
                "cas_hash": _cas_hash(run),
                "name": "test_run.json",
                "size": 0,
                "format": "json",
            }
        },
        "result": run,
    }


def method_stream_telemetry(_params: dict[str, Any] | None) -> dict[str, Any]:
    return {"ok": True, "stream": "mtconnect", "samples": 1000}


def method_collect(_params: dict[str, Any] | None) -> dict[str, Any]:
    return {"ok": True, "collected": True}


METHODS = {
    # Common
    "init": method_init,
    "discover": method_discover,
    "capabilities": method_capabilities,
    # Manufacturing
    "plan": method_plan,
    "simulate": method_simulate,
    "export_program": method_export_program,
    "record_as_run": method_record_as_run,
    # Inspection
    "device_profile": method_device_profile,
    "acquire": method_acquire,
    "register": method_register,
    "export_measurements": method_export_measurements,
    # Test
    "rig_profile": method_rig_profile,
    "configure": method_configure,
    "run": method_run,
    "stream_telemetry": method_stream_telemetry,
    "collect": method_collect,
}


def handle(line: str) -> dict[str, Any] | None:
    line = line.strip()
    if not line:
        return None
    try:
        req = json.loads(line)
    except json.JSONDecodeError as exc:
        return _err(None, -32700, f"parse error: {exc.msg}")
    if not isinstance(req, dict) or req.get("jsonrpc") != "2.0":
        return _err(req.get("id") if isinstance(req, dict) else None, -32600, "invalid request")
    method = req.get("method")
    req_id = req.get("id")
    params = req.get("params")
    if not isinstance(method, str):
        return _err(req_id, -32601, "method must be a string")
    fn = METHODS.get(method)
    if fn is None:
        return _err(req_id, -32601, f"method not found: {method}")
    try:
        result = fn(params)
    except Exception as exc:  # noqa: BLE001
        return _err(req_id, -32603, f"internal error: {exc!r}")
    return _ok(req_id, result)


def main() -> int:
    correlation = uuid.uuid4().hex[:8]
    print(f"[{correlation}] plugin ready (pid={__import__('os').getpid()})", file=sys.stderr)
    for raw in sys.stdin:
        response = handle(raw)
        if response is None:
            continue
        sys.stdout.write(json.dumps(response) + "\n")
        sys.stdout.flush()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
