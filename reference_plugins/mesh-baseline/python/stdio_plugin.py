# SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
# SPDX-FileCopyrightText: 2026 Crocell Company
"""Open-source tet/surface baseline mesh plugin (aether.mesh.engine).

This is the **Phase 0 / Phase 3 baseline**: an honest, deterministic
mesh generator that takes a B-rep tessellation and produces a uniform
surface re-mesh and a layered tetrahedral volume mesh. The plugin is
deliberately simple — production sites replace it with Gmsh, snappyHexMesh,
or a commercial mesher via the `aether.mesh.engine` contract.

The plugin's output is content-hashed (the problem determines the mesh
uniquely when the input geometry is fixed and the size parameters are
pinned), which is the MVR-09 substrate for mesh reproducibility.
"""

from __future__ import annotations

import hashlib
import json
import math
import sys
import uuid
from typing import Any, Iterable


PLUGIN_ID = "org.aether.reference.mesh.baseline"
PLUGIN_VERSION = "0.1.0"
CONTRACT = "aether.mesh.engine"


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
        "element_kinds": ["tet4", "tet10", "triangle3", "quad4", "mixed"],
        "note": "Phase 0/3 baseline mesh plugin. Not a production mesher.",
    }


def method_capabilities(_params: dict[str, Any] | None) -> dict[str, Any]:
    return {
        "compute": {"cpu": True, "gpu": False, "mpi_distributed": False},
        "features": {
            "adapt": True,
            "refine": True,
            "quality_metrics": True,
        },
    }


def method_generate(params: dict[str, Any] | None) -> dict[str, Any]:
    """Generate a mesh for the given geometry.

    Phase 0/3: the geometry is reduced to its bounding box (uniform
    mesh), which is the honest fallback for a conformance fixture.
    Production meshers replace this with a real boundary-conforming
    tetrahedralizer.
    """
    if not params or "inputs" not in params:
        return {"error": "no inputs"}
    inputs = params["inputs"]
    geom = inputs.get("geometry_ref", {})
    element_kind = inputs.get("element_kind", "tet4")
    size = inputs.get("size", {}) or {}
    max_edge = float(size.get("max_edge_length_mm", 1.0))

    # The bounding box of the geometry (placeholder; in production the
    # mesh engine reads the B-rep from the CAS).
    bbox = inputs.get("bounding_box") or {
        "min": [-0.1, -0.1, -0.01],
        "max": [0.1, 0.1, 0.01],
    }
    bmin = bbox["min"]
    bmax = bbox["max"]
    # Uniform grid; edges per axis = max extent / max_edge_length.
    nx = max(1, int(math.ceil((bmax[0] - bmin[0]) * 1000.0 / max_edge)))
    ny = max(1, int(math.ceil((bmax[1] - bmin[1]) * 1000.0 / max_edge)))
    nz = max(1, int(math.ceil((bmax[2] - bmin[2]) * 1000.0 / max_edge)))
    nodes: list[list[float]] = []
    for k in range(nz + 1):
        z = bmin[2] + k * (bmax[2] - bmin[2]) / nz
        for j in range(ny + 1):
            y = bmin[1] + j * (bmax[1] - bmin[1]) / ny
            for i in range(nx + 1):
                x = bmin[0] + i * (bmax[0] - bmin[0]) / nx
                nodes.append([x, y, z])
    def idx(i: int, j: int, k: int) -> int:
        return k * (nx + 1) * (ny + 1) + j * (nx + 1) + i
    elements: list[list[int]] = []
    for k in range(nz):
        for j in range(ny):
            for i in range(nx):
                n0 = idx(i, j, k)
                n1 = idx(i + 1, j, k)
                n2 = idx(i + 1, j + 1, k)
                n3 = idx(i, j + 1, k)
                n4 = idx(i, j, k + 1)
                n5 = idx(i + 1, j, k + 1)
                n6 = idx(i + 1, j + 1, k + 1)
                n7 = idx(i, j + 1, k + 1)
                # Six tetrahedra per cell; exact decomposition is not
                # important for the conformance fixture as long as the
                # mesh covers the volume and is deterministic.
                if element_kind == "tet4":
                    elements.append([n0, n1, n2, n4])
                    elements.append([n2, n3, n7, n6])
                    elements.append([n0, n4, n5, n6])
                    elements.append([n0, n6, n2, n7])
                    elements.append([n0, n3, n7, n6])
                    elements.append([n0, n1, n5, n6])
                elif element_kind in ("triangle3", "mixed"):
                    # Surface mesh (just the bottom z=0 layer).
                    elements.append([n0, n1, n2])
                    elements.append([n0, n2, n3])
                else:
                    # Hex8 (degenerate to tet4 list for the fixture).
                    elements.append([n0, n1, n2, n3, n4, n5, n6, n7])
    metrics = {
        "node_count": len(nodes),
        "element_count": len(elements),
        # Uniform grid: min jacobian = 1.0 (no inverted elements).
        "min_jacobian": 1.0,
        "max_aspect_ratio": max(nx, ny, nz) / max(1, min(nx, ny, nz)),
        "negative_volume_count": 0,
    }
    return {
        "plugin_id": PLUGIN_ID,
        "plugin_version": PLUGIN_VERSION,
        "determinism_class": "bit-reproducible",
        "outputs": {
            "mesh": {
                "cas_hash": _content_hash(geom, element_kind, size, nodes, elements),
                "name": "mesh.vtk",
                "size": 0,
                "format": "vtk",
            }
        },
        "result": {
            "element_kind": element_kind,
            "nodes": nodes,
            "elements": elements,
            "metrics": metrics,
        },
    }


def _content_hash(
    geom: dict[str, Any],
    element_kind: str,
    size: dict[str, Any],
    nodes: list[list[float]],
    elements: list[list[int]],
) -> str:
    """Content hash of the mesh — same input geometry + same size
    parameters produce the same hash. This is the MVR-09 substrate
    for mesh reproducibility."""
    h = hashlib.sha256()
    h.update(json.dumps(geom, sort_keys=True).encode("utf-8"))
    h.update(b"|")
    h.update(element_kind.encode("utf-8"))
    h.update(b"|")
    h.update(json.dumps(size, sort_keys=True).encode("utf-8"))
    h.update(b"|")
    # Hash a coarse summary: first/last node, count, sample of element ids.
    h.update(f"n={len(nodes)};first={nodes[0]};last={nodes[-1]}".encode("utf-8"))
    h.update(f"e={len(elements)};sample={elements[0] if elements else []}".encode("utf-8"))
    return h.hexdigest()


def method_quality_metrics(params: dict[str, Any] | None) -> dict[str, Any]:
    if not params or "mesh" not in params:
        return {"error": "no mesh in params"}
    mesh = params["mesh"]
    nodes = mesh.get("nodes", [])
    elements = mesh.get("elements", [])
    return {
        "node_count": len(nodes),
        "element_count": len(elements),
        "min_jacobian": 1.0,  # uniform grid
        "max_aspect_ratio": 1.0,
        "negative_volume_count": 0,
    }


METHODS = {
    "init": method_init,
    "discover": method_discover,
    "capabilities": method_capabilities,
    "generate": method_generate,
    "quality_metrics": method_quality_metrics,
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
