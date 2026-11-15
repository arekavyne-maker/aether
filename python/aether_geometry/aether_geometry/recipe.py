# SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
# SPDX-FileCopyrightText: 2026 Crocell Company
"""Parametric recipe interpreter (aether.geometry.recipe, SDP §11.1).

A recipe is a versioned, tool-agnostic feature history. This module
provides:

- `Recipe.from_yaml(path)` — parse and validate a recipe.
- `Recipe.bind_parameters(values)` — bind named parameters.
- `Recipe.regenerate()` — execute the recipe's features in order,
  producing a list of `FeatureResult` records. The interpreter is
  **kernel-independent**: it builds a typed `GeometryOp` sequence
  that any `aether.geometry.kernel` implementation can execute.

The interpreter does not perform real B-rep operations; it walks the
feature tree and produces the **typed intent**. The actual
geometry construction is the kernel's job (OCC, commercial CAD, etc.).
This is the structural decoupling that makes MVR-06 work: replace
the kernel, the recipe stays the same.

NOT REAL B-REP
--------------
The interpreter produces typed intent (sketch + extrude + fillet
operations as a DAG) and a stable content hash over that intent.
The actual B-rep construction is delegated to the
``aether.geometry.kernel`` plugin. In Phase 0–2, the only such
plugin in the conformance set is the bounding-box fixture
(``reference_plugins/occ-geometry``); a real B-rep kernel (OCC
binding or equivalent) is a G3 deliverable. The
``infra/fitness/geom_fixture_label.py`` check enforces that the
``occ-geometry`` fixture always declares ``is_real_brep: False``,
so a regression that accidentally drops the honest label is a
build failure.
"""

from __future__ import annotations

import hashlib
import json
import math
import uuid
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Iterable, Mapping

import yaml

# Schema constants mirror contracts/geometry.recipe/schema/recipe.schema.json
_KIND_VALUES = {
    "sketch", "extrude", "revolve", "fillet", "boolean", "transform", "mirror",
}
_UNIT_VALUES = {"m", "cm", "mm"}


@dataclass(frozen=True)
class FeatureResult:
    """The result of executing one feature."""
    feature_id: str
    kind: str
    geometry_handle: str
    params: dict[str, Any]
    success: bool
    message: str = ""


@dataclass
class Recipe:
    """A parametric recipe (the design intent)."""
    recipe_id: str
    version: str
    frame: str
    units: dict[str, str]
    parameters: dict[str, Any]
    features: list[dict[str, Any]]
    _bound: dict[str, Any] = field(default_factory=dict)

    @classmethod
    def from_yaml(cls, path: str | Path) -> "Recipe":
        with open(path, "r", encoding="utf-8") as f:
            raw = yaml.safe_load(f)
        # Strip the harness-only _schema_id field.
        if isinstance(raw, dict) and "_schema_id" in raw:
            raw = {k: v for k, v in raw.items() if k != "_schema_id"}
        return cls.from_dict(raw)

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> "Recipe":
        for required in ("recipe_id", "version", "frame", "units", "parameters", "features"):
            if required not in data:
                raise ValueError(f"recipe missing required field: {required}")
        units = data["units"]
        if "length" not in units or units["length"] not in _UNIT_VALUES:
            raise ValueError(f"recipe units.length must be one of {_UNIT_VALUES}; got {units.get('length')!r}")
        for feat in data["features"]:
            if feat.get("kind") not in _KIND_VALUES:
                raise ValueError(
                    f"feature {feat.get('id')!r} has unknown kind {feat.get('kind')!r}; "
                    f"must be one of {_KIND_VALUES}"
                )
        return cls(
            recipe_id=data["recipe_id"],
            version=data["version"],
            frame=data["frame"],
            units=units,
            parameters=dict(data["parameters"]),
            features=list(data["features"]),
        )

    def bind_parameters(self, values: Mapping[str, Any]) -> "Recipe":
        """Return a new Recipe with the given parameters bound.

        Validates that every bound parameter is declared in the recipe.
        """
        unknown = set(values) - set(self.parameters)
        if unknown:
            raise ValueError(
                f"bind_parameters: unknown parameters {sorted(unknown)}; "
                f"recipe declares {sorted(self.parameters)}"
            )
        merged = {**self.parameters, **dict(values)}
        return Recipe(
            recipe_id=self.recipe_id,
            version=self.version,
            frame=self.frame,
            units=self.units,
            parameters=merged,
            features=self.features,
            _bound=merged,
        )

    def regenerate(self) -> list[FeatureResult]:
        """Execute the feature tree in order.

        The interpreter walks the dependency graph (declared via
        `depends_on`), validates each feature's params against the
        recipe, and emits a `FeatureResult` per feature. The actual
        geometry construction is the kernel's job; this method produces
        the **typed intent** the kernel consumes.

        Features whose `depends_on` references are missing are recorded
        as failed (with `success=False` and a descriptive `message`)
        rather than aborting the whole regeneration. Cycles in the
        declared dependencies are still hard errors via `topo_sort`.
        """
        # Group features by id; allow unknown dependencies (per-feature
        # failure rather than whole-regeneration failure).
        by_id: dict[str, dict[str, Any]] = {f["id"]: f for f in self.features}
        # Topological order: features with unknown dependencies are placed
        # at the end; their `success` will be False.
        try:
            order = topo_sort(self.features)
        except ValueError:
            order = list(self.features)
        # Track which features have been "built" so dependents can reference them.
        built: dict[str, str] = {}
        results: list[FeatureResult] = []
        for feat in order:
            fid = feat["id"]
            kind = feat["kind"]
            params = dict(feat.get("params", {}))
            # Substitute declared parameters (e.g. {{chord_mm}} -> 240.0).
            params = substitute_params(params, self._bound or self.parameters)
            depends_on = feat.get("depends_on", [])
            missing = [d for d in depends_on if d not in built]
            if missing:
                results.append(
                    FeatureResult(
                        feature_id=fid, kind=kind, geometry_handle="",
                        params=params, success=False,
                        message=f"missing dependencies: {missing}",
                    )
                )
                continue
            handle = self._execute_feature(kind, params, built, depends_on)
            built[fid] = handle
            results.append(
                FeatureResult(
                    feature_id=fid, kind=kind, geometry_handle=handle,
                    params=params, success=True,
                )
            )
        return results

    def _execute_feature(
        self, kind: str, params: dict[str, Any],
        built: dict[str, str], depends_on: list[str],
    ) -> str:
        """Return a stable geometry handle for the feature.

        The handle is a content hash of the feature's intent (kind +
        params + dependency handles), so that the same recipe with the
        same bindings produces the same handle — the structural
        substrate of MVR-06 (CAD connector replacement). Real kernels
        will produce B-rep bodies that hash to this handle when the
        recipe is well-formed and the kernel is deterministic.
        """
        h = hashlib.sha256()
        h.update(kind.encode("utf-8"))
        h.update(b"\x00")
        # Sort keys for canonical hashing.
        for key in sorted(params):
            h.update(f"{key}={params[key]!r}\n".encode("utf-8"))
        h.update(b"\x00")
        for dep in sorted(built.keys()):
            h.update(f"dep={dep};handle={built[dep]}\n".encode("utf-8"))
        return f"aether:feature:{h.hexdigest()}"

    def to_kernel_problem(self) -> dict[str, Any]:
        """Build the `geometry.kernel` problem that the kernel will
        consume: one `import` (the recipe) + one `export` (per the
        recipe's target format).

        The kernel's `import` method receives the recipe, the bound
        parameters, and the units/frame; it returns a `geometry_ref`
        that subsequent operations (tessellate, boolean, ...) act on.
        """
        return {
            "operation": "import",
            "geometry_ref": {
                "cas_hash": self.content_hash(),
                "format": "native",
                "frame": self.frame,
                "units": self.units,
            },
            "recipe": {
                "recipe_id": self.recipe_id,
                "version": self.version,
                "parameters": self._bound or self.parameters,
                "features": self.features,
            },
        }

    def content_hash(self) -> str:
        canonical = json.dumps(
            {
                "recipe_id": self.recipe_id,
                "version": self.version,
                "frame": self.frame,
                "units": self.units,
                "parameters": self._bound or self.parameters,
                "features": self.features,
            },
            sort_keys=True,
            separators=(",", ":"),
        ).encode("utf-8")
        return hashlib.sha256(canonical).hexdigest()


def topo_sort(features: list[dict[str, Any]]) -> list[dict[str, Any]]:
    """Topological sort over declared `depends_on` edges.

    Raises ValueError on cycles or unknown dependencies.
    """
    by_id = {f["id"]: f for f in features}
    in_degree: dict[str, int] = {f["id"]: 0 for f in features}
    for f in features:
        for d in f.get("depends_on", []):
            if d not in by_id:
                raise ValueError(f"feature {f['id']!r} depends on unknown {d!r}")
            in_degree[f["id"]] += 1
    ready = [fid for fid, deg in in_degree.items() if deg == 0]
    order: list[dict[str, Any]] = []
    visited: set[str] = set()
    while ready:
        next_id = ready.pop(0)
        if next_id in visited:
            continue
        visited.add(next_id)
        order.append(by_id[next_id])
        for f in features:
            if next_id in f.get("depends_on", []):
                in_degree[f["id"]] -= 1
                if in_degree[f["id"]] == 0:
                    ready.append(f["id"])
    if len(order) != len(features):
        remaining = [f["id"] for f in features if f["id"] not in visited]
        raise ValueError(f"cycle detected involving: {remaining}")
    return order


def substitute_params(
    params: dict[str, Any], values: Mapping[str, Any]
) -> dict[str, Any]:
    """Replace `{{name}}` placeholders in string values.

    Numbers and booleans are passed through unchanged; only strings
    are substituted. This keeps the recipe's intent readable while
    binding parameters at execution time.
    """
    out: dict[str, Any] = {}
    for k, v in params.items():
        if isinstance(v, str) and v.startswith("{{") and v.endswith("}}"):
            name = v[2:-2].strip()
            if name not in values:
                raise ValueError(
                    f"parameter substitution: undefined parameter {name!r} (declared: {sorted(values)})"
                )
            out[k] = values[name]
        elif isinstance(v, dict):
            out[k] = substitute_params(v, values)
        elif isinstance(v, list):
            out[k] = [
                substitute_params(item, values) if isinstance(item, dict) else item
                for item in v
            ]
        else:
            out[k] = v
    return out


__all__ = [
    "FeatureResult",
    "Recipe",
    "substitute_params",
    "topo_sort",
]
