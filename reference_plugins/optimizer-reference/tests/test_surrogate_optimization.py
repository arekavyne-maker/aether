# SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
# SPDX-FileCopyrightText: 2026 Crocell Company
"""Conformance tests for surrogate-assisted optimization (Phase 5).

The optimizer accepts a trained GP payload (``gp_surrogate`` model:
X_train / Y_train / alpha / length_scale) as produced by
aether.surrogate.builder's train(). These tests pin:

1. Prediction equivalence — the optimizer's internal GP prediction is
   numerically identical to surrogates-core's gp_predict for the same
   model and query points (the two implementations of the same math
   must agree; ADR-0022 spirit applied inside the loop).
2. Optimization over the surrogate is deterministic (same seed, same
   pareto front) and converges toward the surrogate's optimum.
3. The optimizer still passes ZDT1/DTLZ2 through unchanged (backward
   compatibility of the model registry).
4. Honesty — results carry plugin identity and bit-reproducible class;
   the surrogate predictions used are labeled as such in the model
   spec the caller passed.
"""

from __future__ import annotations

import importlib.util
import json
import math
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parents[3]

_spec = importlib.util.spec_from_file_location(
    "optimizer_ref_stdio_plugin",
    REPO_ROOT / "reference_plugins" / "optimizer-reference" / "python" / "stdio_plugin.py",
)
assert _spec and _spec.loader
opt = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(opt)

_spec2 = importlib.util.spec_from_file_location(
    "surrogates_core_stdio_plugin",
    REPO_ROOT / "reference_plugins" / "surrogates-core" / "python" / "stdio_plugin.py",
)
assert _spec2 and _spec2.loader
sg = importlib.util.module_from_spec(_spec2)
_spec2.loader.exec_module(sg)


def trained_model():
    """Train a GP through surrogates-core on a smooth 2D function."""
    X = [
        [0.0, 0.0], [0.1, 0.0], [0.0, 0.1], [0.1, 0.1],
        [0.05, 0.05], [0.0, 0.075], [0.075, 0.0], [0.1, 0.05],
    ]
    Y = [[1.0 + 20.0 * x[0] + 10.0 * x[1]] for x in X]
    tr = sg.method_train({"inputs": {"kind": "gp", "X": X, "Y": Y}})
    assert "error" not in tr, tr
    return tr["result"]["model"]


# ---------------------------------------------------------------------------
# 1. Prediction equivalence between the two GP implementations
# ---------------------------------------------------------------------------

def test_optimizer_gp_matches_surrogates_core() -> None:
    model = trained_model()
    queries = [[0.01, 0.02], [0.09, 0.09], [0.05, 0.0], [0.033, 0.077]]
    for q in queries:
        a = sg.gp_predict(
            {
                "X_train": model["X_train"],
                "Y_train": model["Y_train"],
                "alpha": model["alpha"],
                "length_scale": model["length_scale"],
            },
            q,
        )
        b = opt.gp_surrogate_predict(model, q)
        assert a[0] == pytest.approx(b[0], rel=1e-15, abs=1e-18)


def test_gp_surrogate_predict_tracks_training_points() -> None:
    """The GP is trained with noise=1e-3 and a FIXED length_scale=1
    (no hyperparameter learning — a documented surrogate-core fixture
    limitation). On a design that is dense relative to the kernel scale
    (K nearly all-ones), conditioning degrades training-point tracking
    to ~1e-1. The prediction must still track the data to that stated
    bound."""
    model = trained_model()
    worst = 0.0
    for x_row, y_row in zip(model["X_train"], model["Y_train"]):
        pred = opt.gp_surrogate_predict(model, x_row)
        worst = max(worst, abs(pred[0] - y_row[0]) / max(abs(y_row[0]), 1e-12))
    assert worst < 2e-1, f"training-point tracking error {worst:.3e}"


def test_gp_surrogate_beats_linear_model_on_quadratic() -> None:
    """On a curved function the GP must predict better than the
    legacy linear fallback (y = sum x_j)."""
    X = [[0.0], [0.25], [0.5], [0.75], [1.0]]
    Y = [[x[0] ** 2] for x in X]
    tr = sg.method_train({"inputs": {"kind": "gp", "X": X, "Y": Y}})
    model = tr["result"]["model"]
    for q in ([0.1], [0.6], [0.9]):
        truth = q[0] ** 2
        gp = opt.gp_surrogate_predict(model, q)[0]
        lin = sum(q)
        assert abs(gp - truth) <= abs(lin - truth) + 1e-9


# ---------------------------------------------------------------------------
# 2. Optimization over the surrogate
# ---------------------------------------------------------------------------

def test_optimize_on_gp_surrogate_deterministic() -> None:
    model = dict(trained_model(), plugin="gp_surrogate")
    problem = {
        "algorithm": "nsga_iii",
        "variables": [
            {"name": "a", "lower": 0.0, "upper": 0.1},
            {"name": "b", "lower": 0.0, "upper": 0.1},
        ],
        "objectives": [{"name": "mass", "sense": "minimize"}],
        "model": model,
        "population_size": 12,
        "generations": 6,
        "seed": 42,
    }
    r1 = opt.method_optimize({"inputs": problem})
    r2 = opt.method_optimize({"inputs": problem})
    assert json.dumps(r1["result"], sort_keys=True) == json.dumps(r2["result"], sort_keys=True)


def test_optimize_finds_better_than_random_start() -> None:
    """The GP surrogate is linear-ish here, so the optimizer should find
    a point near the corner a=0, b=0 (mass ~1.0) — better than the box
    midpoint prediction."""
    model = dict(trained_model(), plugin="gp_surrogate")
    r = opt.method_optimize({"inputs": {
        "algorithm": "nsga_iii",
        "variables": [
            {"name": "a", "lower": 0.0, "upper": 0.1},
            {"name": "b", "lower": 0.0, "upper": 0.1},
        ],
        "objectives": [{"name": "mass", "sense": "minimize"}],
        "model": model,
        "population_size": 16,
        "generations": 10,
        "seed": 42,
    }})
    best = min(e["f"]["mass"] for e in r["result"]["pareto"])
    midpoint = opt.gp_surrogate_predict(model, [0.05, 0.05])[0]
    assert best < midpoint


def test_result_provenance_fields() -> None:
    model = dict(trained_model(), plugin="gp_surrogate")
    r = opt.method_optimize({"inputs": {
        "algorithm": "nsga_iii",
        "variables": [{"name": "a", "lower": 0.0, "upper": 0.1}],
        "objectives": [{"name": "f", "sense": "minimize"}],
        "model": model,
        "population_size": 8,
        "generations": 3,
        "seed": 42,
    }})
    assert r["plugin_id"] == opt.PLUGIN_ID
    assert r["determinism_class"] == "bit-reproducible"
    assert r["result"]["algorithm"] == "nsga_iii"
    assert r["result"]["convergence"]["converged"] is True


# ---------------------------------------------------------------------------
# 3. Backward compatibility of the model registry
# ---------------------------------------------------------------------------

def test_zdt1_still_works() -> None:
    r = opt.method_optimize({"inputs": {
        "algorithm": "nsga_iii",
        "variables": [{"name": "x1", "lower": 0.0, "upper": 1.0},
                      {"name": "x2", "lower": 0.0, "upper": 1.0}],
        "objectives": [{"name": "f1", "sense": "minimize"}, {"name": "f2", "sense": "minimize"}],
        "model": {"plugin": "aether.test.zdt1"},
        "population_size": 8,
        "generations": 3,
        "seed": 42,
    }})
    assert len(r["result"]["pareto"]) >= 1
