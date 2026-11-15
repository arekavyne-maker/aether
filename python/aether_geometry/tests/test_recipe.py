# SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
# SPDX-FileCopyrightText: 2026 Crocell Company
"""Conformance tests for the parametric recipe interpreter (G3)."""

from __future__ import annotations

import json
import textwrap
from pathlib import Path

import pytest

from aether_geometry import FeatureResult, Recipe, substitute_params, topo_sort


def _sample_recipe_yaml(tmp_path: Path) -> Path:
    p = tmp_path / "rib.yaml"
    p.write_text(
        textwrap.dedent(
            """
            recipe_id: wing-rib-014
            version: "1.0.0"
            frame: "aether://proj/frames/wing-root"
            units:
              length: "mm"
              angle: "deg"
            parameters:
              chord_mm: 240.0
              thickness_mm: 4.0
              height_mm: 80.0
            features:
              - id: base_sketch
                kind: sketch
                params:
                  plane: XY
              - id: extrude
                kind: extrude
                depends_on: [base_sketch]
                params:
                  direction: [0, 0, 1]
                  distance_mm: "{{ thickness_mm }}"
              - id: rounds
                kind: fillet
                depends_on: [extrude]
                params:
                  edges: ["*"]
                  radius_mm: 1.0
            """
        ).strip()
    )
    return p


def test_parses_yaml(tmp_path: Path) -> None:
    r = Recipe.from_yaml(_sample_recipe_yaml(tmp_path))
    assert r.recipe_id == "wing-rib-014"
    assert r.version == "1.0.0"
    assert r.units == {"length": "mm", "angle": "deg"}
    assert r.parameters["chord_mm"] == 240.0
    assert len(r.features) == 3


def test_rejects_unknown_feature_kind(tmp_path: Path) -> None:
    bad = tmp_path / "bad.yaml"
    bad.write_text(
        textwrap.dedent(
            """
            recipe_id: r
            version: "1.0.0"
            frame: f
            units: { length: mm }
            parameters: {}
            features:
              - { id: f1, kind: unknown_kind }
            """
        ).strip()
    )
    with pytest.raises(ValueError, match="unknown kind"):
        Recipe.from_yaml(bad)


def test_rejects_missing_required_field(tmp_path: Path) -> None:
    bad = tmp_path / "bad.yaml"
    bad.write_text(
        textwrap.dedent(
            """
            recipe_id: r
            version: "1.0.0"
            frame: f
            units: { length: mm }
            parameters: {}
            """
        ).strip()
    )
    with pytest.raises(ValueError, match="missing required field: features"):
        Recipe.from_yaml(bad)


def test_rejects_bad_unit(tmp_path: Path) -> None:
    bad = tmp_path / "bad.yaml"
    bad.write_text(
        textwrap.dedent(
            """
            recipe_id: r
            version: "1.0.0"
            frame: f
            units: { length: km }
            parameters: {}
            features: []
            """
        ).strip()
    )
    with pytest.raises(ValueError, match="units.length"):
        Recipe.from_yaml(bad)


def test_bind_parameters_validates_against_declared(tmp_path: Path) -> None:
    r = Recipe.from_yaml(_sample_recipe_yaml(tmp_path))
    with pytest.raises(ValueError, match="unknown parameters"):
        r.bind_parameters({"unknown_param": 1.0})


def test_regenerate_resolves_template(tmp_path: Path) -> None:
    r = Recipe.from_yaml(_sample_recipe_yaml(tmp_path))
    bound = r.bind_parameters({"chord_mm": 250.0, "thickness_mm": 5.0})
    results = bound.regenerate()
    assert len(results) == 3
    assert all(r.success for r in results)
    # The extrude step's distance must equal the bound thickness.
    extrude = next(r for r in results if r.feature_id == "extrude")
    assert extrude.params["distance_mm"] == 5.0


def test_regenerate_handles_missing_dependencies() -> None:
    # An extrude feature with a non-existent dependency should fail cleanly.
    r = Recipe.from_dict(
        {
            "recipe_id": "x",
            "version": "1.0.0",
            "frame": "f",
            "units": {"length": "mm"},
            "parameters": {},
            "features": [
                {"id": "f1", "kind": "sketch", "params": {}},
                {"id": "f2", "kind": "extrude", "params": {}, "depends_on": ["missing"]},
            ],
        }
    )
    results = r.regenerate()
    assert len(results) == 2
    # The first feature succeeds; the second fails because of the missing dep.
    assert results[0].success
    assert not results[1].success
    assert "missing dependencies" in results[1].message


def test_geometry_handles_stable_across_runs() -> None:
    """MVR-06 substrate: the same recipe + parameters must produce the
    same feature handles across runs. The handle is the content hash of
    the typed intent; real kernels that honor the intent produce
    bodies that hash to this handle."""
    r1 = Recipe.from_dict(
        {
            "recipe_id": "x",
            "version": "1.0.0",
            "frame": "f",
            "units": {"length": "mm"},
            "parameters": {"a": 1.0},
            "features": [{"id": "f1", "kind": "sketch", "params": {"plane": "XY"}}],
        }
    )
    r2 = Recipe.from_dict(
        {
            "recipe_id": "x",
            "version": "1.0.0",
            "frame": "f",
            "units": {"length": "mm"},
            "parameters": {"a": 1.0},
            "features": [{"id": "f1", "kind": "sketch", "params": {"plane": "XY"}}],
        }
    )
    h1 = r1.bind_parameters({"a": 1.0}).regenerate()[0].geometry_handle
    h2 = r2.bind_parameters({"a": 1.0}).regenerate()[0].geometry_handle
    assert h1 == h2


def test_geometry_handles_differ_when_parameters_differ() -> None:
    r = Recipe.from_dict(
        {
            "recipe_id": "x",
            "version": "1.0.0",
            "frame": "f",
            "units": {"length": "mm"},
            "parameters": {"a": 1.0},
            "features": [{"id": "f1", "kind": "sketch", "params": {"a": "{{ a }}"}}],
        }
    )
    h_a = r.bind_parameters({"a": 1.0}).regenerate()[0].geometry_handle
    h_b = r.bind_parameters({"a": 2.0}).regenerate()[0].geometry_handle
    assert h_a != h_b


def test_topo_sort_handles_cycle() -> None:
    with pytest.raises(ValueError, match="cycle"):
        topo_sort(
            [
                {"id": "a", "kind": "sketch", "params": {}, "depends_on": ["b"]},
                {"id": "b", "kind": "sketch", "params": {}, "depends_on": ["a"]},
            ]
        )


def test_to_kernel_problem_has_recipe_block() -> None:
    r = Recipe.from_dict(
        {
            "recipe_id": "x",
            "version": "1.0.0",
            "frame": "f",
            "units": {"length": "mm"},
            "parameters": {},
            "features": [],
        }
    )
    p = r.to_kernel_problem()
    assert p["operation"] == "import"
    assert p["geometry_ref"]["format"] == "native"
    assert p["recipe"]["recipe_id"] == "x"


def test_substitute_params_nested_dict() -> None:
    out = substitute_params(
        {"plane": "XY", "origin": {"x": "{{ chord }}", "y": 0.0}},
        {"chord": 1.5},
    )
    assert out["origin"]["x"] == 1.5
    assert out["origin"]["y"] == 0.0


def test_substitute_params_undefined_raises() -> None:
    with pytest.raises(ValueError, match="undefined parameter"):
        substitute_params({"a": "{{ missing }}"}, {})
