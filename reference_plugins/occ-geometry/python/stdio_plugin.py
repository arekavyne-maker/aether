# SPDX-License-Identifier: Aether-VPSL-1.0
# SPDX-FileCopyrightText: 2026 The Aether Consortium
"""OCC-shaped geometry kernel (aether.geometry.kernel) — Phase 0/3 reference.

This is a **conformance fixture** (SDP §5.6, §21): a deliberately simple
Python implementation of the `aether.geometry.kernel` contract that
follows the same import / parameterize / tessellate / boolean / fillet
API surface as a real OpenCASCADE binding but performs **bounding-box
operations** rather than real B-rep. It is the second of the two
implementations that satisfy the two-impl rule (ADR-0022) for the
contract, so MVR-06 (CAD connector replacement: OCC vs. reference) can
exercise plugin interchangeability with this fixture.

NOT REAL B-REP — BLOCKING DEPENDENCY
------------------------------------
This plugin does not perform any real B-rep operation. All
``import`` / ``boolean`` / ``fillet`` / ``tessellate`` operations
operate on bounding-box primitives only. Real B-rep would require
either a Python binding to OpenCASCADE (``pythonocc-core``) or
``trimesh``/``manifold3d`` for tessellated-only B-rep equivalents;
neither is in the air-gap-friendly Python toolchain pinned by
``pyproject.toml`` at the time of writing. The
``infra/fitness/geom_fixture_label.py`` fitness check enforces this
sentinel: a regression that drops the ``not real B-rep`` marker
from this file's discover() response fails the build.

The contract-required second implementation is the bounding-box
fixture itself for the Phase 0–2 conformance substrate; the
*production* OCC connector is a G3 deliverable, tracked in
``workflows/paper-run/gaps.md``.
"""

from __future__ import annotations

import hashlib
import json
import math
import sys
import uuid
from typing import Any

PLUGIN_ID = "org.aether.reference.geometry.occ"
PLUGIN_VERSION = "0.1.0"
CONTRACT = "aether.geometry.kernel"


def _ok(req_id: Any, result: dict[str, Any]) -> dict[str, Any]:
    return {"jsonrpc": "2.0", "id": req_id, "result": result}


def _err(req_id: Any, code: int, message: str) -> dict[str, Any]:
    return {"jsonrpc": "2.0", "id": req_id, "error": {"code": code, "message": message}}


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
        "contract_id": CONTRACT,
        "contract_version_range": ">=0.1.0 <1.0.0",
        "kind": "reference",
        "determinism_class": "bit-reproducible",
        "is_real_brep": False,
        "fixture_kind": "bounding-box",
        "note": "Phase 0/3 reference (bounding-box kernel); not real B-rep. Replaced by real OpenCASCADE in production (G3 deliverable, workflows/paper-run/gaps.md).",
    }


def method_capabilities(_params: dict[str, Any] | None) -> dict[str, Any]:
    return {
        "operations": ["import", "export", "parameterize", "tessellate", "boolean", "fillet", "check"],
        "imports": ["step-ap242", "iges", "stl", "native"],
        "exports": ["step-ap242", "iges", "stl", "native"],
        "features": {"boolean": True, "fillet": True, "tessellate": True},
    }


def method_import(params: dict[str, Any] | None) -> dict[str, Any]:
    """Import a geometry. The OCC-shaped reference interprets the recipe's
    parameters and returns a bounding-box B-rep.

    The CAS hash of the returned geometry is content-addressed over the
    recipe + parameters, so two runs of the same recipe + params
    produce the same geometry.
    """
    if not params:
        return {"error": "no params"}
    inputs = params.get("inputs", params)
    recipe = inputs.get("recipe", {})
    parameters = recipe.get("parameters", {}) or {}
    if not recipe:
        # No recipe: fall back to a unit cube.
        bbox = [[-0.05, -0.05, -0.05], [0.05, 0.05, 0.05]]
    else:
        bbox = _bounding_box_from_recipe(recipe, parameters)
    return {
        "plugin_id": PLUGIN_ID,
        "plugin_version": PLUGIN_VERSION,
        "determinism_class": "bit-reproducible",
        "outputs": {
            "geometry": {
                "cas_hash": _hash_geometry(bbox, recipe, parameters),
                "name": "geometry.step",
                "format": "step-ap242",
                "bounding_box": bbox,
            }
        },
    }


def method_tessellate(params: dict[str, Any] | None) -> dict[str, Any]:
    if not params:
        return {"error": "no params"}
    inputs = params.get("inputs", params)
    geom = inputs.get("geometry", {})
    bbox = geom.get("bounding_box")
    if not bbox:
        return {"error": "no bounding_box on geometry"}
    tess = _tessellate_bbox(bbox, inputs.get("tessellation", {}))
    return {
        "plugin_id": PLUGIN_ID,
        "plugin_version": PLUGIN_VERSION,
        "outputs": {
            "tessellation": {
                "cas_hash": _hash_tess(tess),
                "name": "tess.stl",
                "format": "stl",
            }
        },
        "tessellation": tess,
    }


def method_boolean(params: dict[str, Any] | None) -> dict[str, Any]:
    if not params:
        return {"error": "no params"}
    inputs = params.get("inputs", params)
    a = inputs.get("a", {}).get("bounding_box")
    b = inputs.get("b", {}).get("bounding_box")
    op = inputs.get("op", "union")
    if not a or not b:
        return {"error": "boolean op requires a and b bounding boxes"}
    out = _boolean_bbox(a, b, op)
    return {
        "outputs": {
            "geometry": {
                "cas_hash": _hash_geometry(out, {"op": op}, {}),
                "name": "boolean.step",
                "format": "step-ap242",
                "bounding_box": out,
            }
        },
    }


def method_fillet(params: dict[str, Any] | None) -> dict[str, Any]:
    if not params:
        return {"error": "no params"}
    inputs = params.get("inputs", params)
    geom = inputs.get("geometry", {})
    bbox = geom.get("bounding_box")
    if not bbox:
        return {"error": "no bounding_box on geometry"}
    fillet = inputs.get("fillet", {}) or {}
    radius = float(fillet.get("radius_mm", 0.0)) / 1000.0
    out = _fillet_bbox(bbox, radius)
    return {
        "outputs": {
            "geometry": {
                "cas_hash": _hash_geometry(out, {"fillet": radius}, {}),
                "name": "fillet.step",
                "format": "step-ap242",
                "bounding_box": out,
            }
        },
    }


def method_check(params: dict[str, Any] | None) -> dict[str, Any]:
    if not params:
        return {"error": "no params"}
    inputs = params.get("inputs", params)
    geom = inputs.get("geometry", {})
    bbox = geom.get("bounding_box")
    if not bbox:
        return {"ok": False, "reason": "no bounding_box"}
    extent = [
        bbox[1][0] - bbox[0][0],
        bbox[1][1] - bbox[0][1],
        bbox[1][2] - bbox[0][2],
    ]
    return {
        "ok": all(e > 0 for e in extent),
        "extent": extent,
        "checks": [
            {"name": "bbox_positive", "kind": "sanity", "result": "pass" if all(e > 0 for e in extent) else "fail"},
        ],
    }


# -- B-rep operations on bounding boxes (the conformance-fixture B-rep) --

def _bounding_box_from_recipe(
    recipe: dict[str, Any],
    parameters: dict[str, Any],
) -> list[list[float]]:
    """Compute the bounding box from a parametric recipe.

    Honors the standard "rib-like" features (sketch + extrude) and
    falls back to a unit cube if the recipe is empty. The mm -> m
    conversion is the only unit handling the fixture needs.
    """
    # Default: a 240x80x4 mm rib.
    chord_mm = float(parameters.get("chord_mm", 240.0))
    height_mm = float(parameters.get("height_mm", 80.0))
    thickness_mm = float(parameters.get("thickness_mm", 4.0))
    half_c = chord_mm / 2000.0
    half_h = height_mm / 2000.0
    half_t = thickness_mm / 2000.0
    return [[-half_c, -half_h, -half_t], [half_c, half_h, half_t]]


def _tessellate_bbox(
    bbox: list[list[float]], opts: dict[str, Any]
) -> dict[str, Any]:
    """Tessellate a bounding box into 12 triangles (2 per face)."""
    bmin, bmax = bbox
    x0, y0, z0 = bmin
    x1, y1, z1 = bmax
    # 8 vertices, 6 faces, 2 triangles per face = 12 triangles.
    vertices = [
        [x0, y0, z0], [x1, y0, z0], [x1, y1, z0], [x0, y1, z0],
        [x0, y0, z1], [x1, y0, z1], [x1, y1, z1], [x0, y1, z1],
    ]
    triangles = [
        [0, 2, 1], [0, 3, 2],  # -z
        [4, 5, 6], [4, 6, 7],  # +z
        [0, 1, 5], [0, 5, 4],  # -y
        [2, 3, 7], [2, 7, 6],  # +y
        [0, 4, 7], [0, 7, 3],  # -x
        [1, 2, 6], [1, 6, 5],  # +x
    ]
    return {
        "vertices": vertices,
        "triangles": triangles,
        "bounding_box": {"min": bmin, "max": bmax},
        "element_quality": {
            "min_jacobian": 1.0,
            "max_aspect_ratio": max(
                abs(x1 - x0), abs(y1 - y0), abs(z1 - z0)
            ) / max(min(abs(x1 - x0), abs(y1 - y0), abs(z1 - z0)), 1e-9),
            "negative_volume_count": 0,
        },
    }


def _boolean_bbox(a: list[list[float]], b: list[list[float]], op: str) -> list[list[float]]:
    """Bounding-box boolean op."""
    if op == "union":
        return [
            [min(a[0][0], b[0][0]), min(a[0][1], b[0][1]), min(a[0][2], b[0][2])],
            [max(a[1][0], b[1][0]), max(a[1][1], b[1][1]), max(a[1][2], b[1][2])],
        ]
    if op == "intersect":
        return [
            [max(a[0][0], b[0][0]), max(a[0][1], b[0][1]), max(a[0][2], b[0][2])],
            [min(a[1][0], b[1][0]), min(a[1][1], b[1][1]), min(a[1][2], b[1][2])],
        ]
    if op == "subtract":
        return a  # approximate; not a real B-rep
    if op == "xor":
        return [
            [min(a[0][0], b[0][0]), min(a[0][1], b[0][1]), min(a[0][2], b[0][2])],
            [max(a[1][0], b[1][0]), max(a[1][1], b[1][1]), max(a[1][2], b[1][2])],
        ]
    return a


def _fillet_bbox(bbox: list[list[float]], radius: float) -> list[list[float]]:
    """A fillet shrinks the bounding box by `radius` on each face."""
    return [
        [bbox[0][0] + radius, bbox[0][1] + radius, bbox[0][2] + radius],
        [bbox[1][0] - radius, bbox[1][1] - radius, bbox[1][2] - radius],
    ]


def _hash_geometry(
    bbox: list[list[float]],
    recipe: dict[str, Any],
    parameters: dict[str, Any],
) -> str:
    h = hashlib.sha256()
    h.update(json.dumps(bbox, sort_keys=True).encode("utf-8"))
    h.update(b"|")
    h.update(json.dumps(recipe, sort_keys=True).encode("utf-8"))
    h.update(b"|")
    h.update(json.dumps(parameters, sort_keys=True).encode("utf-8"))
    return h.hexdigest()


def _hash_tess(tess: dict[str, Any]) -> str:
    h = hashlib.sha256()
    h.update(json.dumps(tess["vertices"]).encode("utf-8"))
    h.update(b"|")
    h.update(json.dumps(tess["triangles"]).encode("utf-8"))
    return h.hexdigest()


METHODS = {
    "init": method_init,
    "discover": method_discover,
    "capabilities": method_capabilities,
    "import": method_import,
    "tessellate": method_tessellate,
    "boolean": method_boolean,
    "fillet": method_fillet,
    "check": method_check,
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
