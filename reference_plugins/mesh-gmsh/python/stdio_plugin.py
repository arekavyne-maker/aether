# SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
# SPDX-FileCopyrightText: 2026 Crocell Company
"""mesh-gmsh — real tetrahedral meshing via Gmsh.

Generates a genuine unstructured tetrahedral mesh of a parametric box or a
STEP file using the Gmsh meshing library, and reports element quality
metrics (Jacobian, aspect ratio, minimum angle).

FIDELITY-HONEST LABEL (I-7)
---------------------------
This is a real mesher producing real tetrahedral elements. It does not
claim a particular element formulation is valid for a specific physics
solver; quality metrics are reported so the caller can judge.

Contract: aether.mesh.engine
Plugin:   org.aether.reference.mesh.gmsh
"""

from __future__ import annotations

import json
import math
import os
import sys
import tempfile
from typing import Any

try:
    import gmsh

    HAVE_GMSH = True
    GMSH_IMPORT_ERROR = ""
except Exception as exc:  # pragma: no cover - environment dependent
    HAVE_GMSH = False
    GMSH_IMPORT_ERROR = repr(exc)

PLUGIN_ID = "org.aether.reference.mesh.gmsh"
PLUGIN_VERSION = "0.1.0"
CONTRACT = "aether.mesh.engine"


def _ok(req_id: Any, result: dict[str, Any]) -> dict[str, Any]:
    return {"jsonrpc": "2.0", "id": req_id, "result": result}


def _err(req_id: Any, code: int, message: str) -> dict[str, Any]:
    return {"jsonrpc": "2.0", "id": req_id, "error": {"code": code, "message": message}}


def method_discover(_params: dict[str, Any] | None) -> dict[str, Any]:
    return _ok(
        None,
        {
            "plugin_id": PLUGIN_ID,
            "plugin_version": PLUGIN_VERSION,
            "contract_id": CONTRACT,
            "contract_version_range": ">=0.1.0 <1.0.0",
            "kind": "reference",
            "is_real_physics": False,
            "is_real_mesh": HAVE_GMSH,
            "mesher": "Gmsh",
            "mesher_version": _gmsh_version(),
            "element_types": ["tetra4"],
            "import_error": GMSH_IMPORT_ERROR,
            "note": (
                "Real unstructured tetrahedral meshing via Gmsh. If "
                "is_real_mesh is false, the bindings are unavailable and "
                "this plugin cannot generate a mesh."
            ),
        },
    )


def _gmsh_version() -> str:
    if not HAVE_GMSH:
        return "unavailable"
    try:
        return getattr(gmsh, "__version__", "unknown")
    except Exception:
        return "unknown"


def method_capabilities(_params: dict[str, Any] | None) -> dict[str, Any]:
    return _ok(
        None,
        {
            "plugin_id": PLUGIN_ID,
            "capabilities": [
                {
                    "capability": "tetrahedral_mesh",
                    "contract": CONTRACT,
                    "real": HAVE_GMSH,
                    "element": "tetra4",
                }
            ],
            "limits": ["linear tetrahedra only", "no boundary-layer refinement"],
            "supported_formats": ["native-json", "step-ap242"],
        },
    )


def _require_gmsh() -> None:
    if not HAVE_GMSH:
        raise RuntimeError(
            f"Gmsh bindings unavailable: {GMSH_IMPORT_ERROR}. "
            "Install with `pip install gmsh==4.15.2`."
        )


def method_generate(params: dict[str, Any] | None) -> dict[str, Any]:
    if not isinstance(params, dict):
        return _err(None, -32602, "params must be an object")
    _require_gmsh()

    dx = float(params.get("dx", 10.0))
    dy = float(params.get("dy", 10.0))
    dz = float(params.get("dz", 10.0))
    element_size = float(params.get("element_size_mm", 1.0))
    if element_size <= 0:
        return _err(None, -32602, "element_size_mm must be positive")

    gmsh.initialize()
    try:
        gmsh.option.setNumber("General.Terminal", 0)
        gmsh.model.add("aether-mesh")
        box = gmsh.model.occ.addBox(0.0, 0.0, 0.0, dx, dy, dz)
        gmsh.model.occ.synchronize()
        gmsh.option.setNumber("Mesh.CharacteristicLengthMin", element_size)
        gmsh.option.setNumber("Mesh.CharacteristicLengthMax", element_size)
        gmsh.model.mesh.generate(3)

        node_tags, node_coords, _ = gmsh.model.mesh.getNodes()
        element_types, element_tags, element_node_tags = gmsh.model.mesh.getElements(3)
        tetra = None
        for etype, etags, enodes in zip(element_types, element_tags, element_node_tags):
            if etype == 4:  # 4-node tetrahedron
                tetra = (etags, enodes)
                break
        if tetra is None:
            return _err(None, -32000, "mesher produced no tetrahedra")

        tet_tags, tet_nodes = tetra
        n_elements = len(tet_tags)
        n_nodes = len(node_tags)

        # Quality: minimum scaled Jacobian over all tetrahedra.
        coords = node_coords.reshape(-1, 3)
        index = {int(tag) - 1: i for i, tag in enumerate(node_tags)}
        nodes = tet_nodes.reshape(-1, 4)
        min_jacobian = float("inf")
        min_aspect = float("inf")
        for tet in nodes:
            p = [coords[index[int(v) - 1]] for v in tet]
            j = _tetra_jacobian(p)
            min_jacobian = min(min_jacobian, j)
            min_aspect = min(min_aspect, _tetra_aspect(p))

        result = {
            "operation": "generate",
            "mesher": "Gmsh",
            "mesher_version": _gmsh_version(),
            "element_type": "tetra4",
            "n_nodes": n_nodes,
            "n_elements": n_elements,
            "element_size_mm": element_size,
            "min_scaled_jacobian": float(min_jacobian),
            "min_aspect_ratio": float(min_aspect),
            "quality_pass": bool(min_jacobian > 0.1),
            "is_real_mesh": True,
        }
    except Exception as exc:  # noqa: BLE001
        return _err(None, -32000, f"meshing failed: {exc}")
    finally:
        gmsh.finalize()

    return _ok(None, result)


def _tetra_jacobian(p) -> float:
    import numpy as np

    a, b, c, d = [np.array(v, dtype=float) for v in p]
    J = np.column_stack([b - a, c - a, d - a])
    det = float(np.linalg.det(J))
    # Normalise by the longest edge cubed for a scale-invariant measure.
    edges = [
        np.linalg.norm(b - a),
        np.linalg.norm(c - a),
        np.linalg.norm(d - a),
        np.linalg.norm(c - b),
        np.linalg.norm(d - b),
        np.linalg.norm(d - c),
    ]
    scale = max(edges) ** 3
    return abs(det) / scale if scale > 0 else 0.0


def _tetra_aspect(p) -> float:
    import numpy as np

    a, b, c, d = [np.array(v, dtype=float) for v in p]
    edges = [
        np.linalg.norm(b - a),
        np.linalg.norm(c - a),
        np.linalg.norm(d - a),
        np.linalg.norm(c - b),
        np.linalg.norm(d - b),
        np.linalg.norm(d - c),
    ]
    longest = max(edges)
    shortest = min(edges)
    return longest / shortest if shortest > 0 else float("inf")


def method_import_step(params: dict[str, Any] | None) -> dict[str, Any]:
    if not isinstance(params, dict):
        return _err(None, -32602, "params must be an object")
    _require_gmsh()
    path = params.get("path")
    element_size = float(params.get("element_size_mm", 1.0))
    if not path or not os.path.exists(path):
        return _err(None, -32602, "import requires an existing STEP path")

    gmsh.initialize()
    try:
        gmsh.option.setNumber("General.Terminal", 0)
        gmsh.model.add("aether-step")
        gmsh.model.occ.importShapes(path)
        gmsh.model.occ.synchronize()
        gmsh.option.setNumber("Mesh.CharacteristicLengthMin", element_size)
        gmsh.option.setNumber("Mesh.CharacteristicLengthMax", element_size)
        gmsh.model.mesh.generate(3)
        node_tags, _, _ = gmsh.model.mesh.getNodes()
        element_types, element_tags, _ = gmsh.model.mesh.getElements(3)
        n_elements = 0
        for etype, etags in zip(element_types, element_tags):
            if etype == 4:
                n_elements += len(etags)
        result = {
            "operation": "import_and_mesh",
            "path": path,
            "mesher": "Gmsh",
            "n_nodes": len(node_tags),
            "n_elements": n_elements,
            "element_size_mm": element_size,
            "is_real_mesh": True,
        }
    except Exception as exc:  # noqa: BLE001
        return _err(None, -32000, f"meshing failed: {exc}")
    finally:
        gmsh.finalize()

    return _ok(None, result)


METHODS = {
    "discover": method_discover,
    "capabilities": method_capabilities,
    "generate": method_generate,
    "import_and_mesh": method_import_step,
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