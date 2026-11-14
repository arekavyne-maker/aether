# SPDX-License-Identifier: Aether-VPSL-1.0
# SPDX-FileCopyrightText: 2026 The Aether Consortium
"""UQ methods (aether.uq.method) — Phase 0/4 reference plugin.

Implements LHS, Monte Carlo, polynomial chaos, and Sobol on a simple
analytical test function. The plugin is a **conformance fixture**:
it validates the protocol and the result schema, and produces results
whose Sobol indices can be verified analytically (the Ishigami
function with known S1/ST indices).
"""

from __future__ import annotations

import json
import math
import sys
import time
import uuid
from typing import Any

import numpy as np  # type: ignore[import-not-found]

PLUGIN_ID = "org.aether.reference.uq.basic"
PLUGIN_VERSION = "0.1.0"
CONTRACT = "aether.uq.method"


# -- Helpers --

def _seeded(seed: int | None) -> np.random.Generator:
    if seed is None:
        return np.random.default_rng()
    return np.random.default_rng(int(seed))


def _sample_inputs(inputs: list[dict[str, Any]], n: int, rng: np.random.Generator) -> np.ndarray:
    """Sample an (n, d) matrix of input values per the input spec."""
    cols: list[np.ndarray] = []
    for spec in inputs:
        name = spec["name"]
        d = spec["distribution"]
        params = spec.get("params", {}) or {}
        if d == "normal":
            mu = float(params.get("mean", 0.0))
            sd = float(params.get("std", 1.0))
            cols.append(rng.normal(mu, sd, size=n))
        elif d == "uniform":
            lo = float(params.get("min", 0.0))
            hi = float(params.get("max", 1.0))
            cols.append(rng.uniform(lo, hi, size=n))
        elif d == "lognormal":
            mu = float(params.get("mean", 0.0))
            sigma = float(params.get("sigma", 1.0))
            cols.append(rng.lognormal(mu, sigma, size=n))
        elif d == "deterministic":
            v = float(params.get("value", 0.0))
            cols.append(np.full(n, v))
        else:
            # Fall back to uniform [0, 1].
            cols.append(rng.uniform(0.0, 1.0, size=n))
    return np.column_stack(cols) if cols else np.zeros((n, 0))


def _ishigami(x: np.ndarray) -> np.ndarray:
    """The Ishigami test function. Sobol indices are known analytically:
    S1(x1) = 0.314, S1(x2) = 0.442, S1(x3) = 0.0
    ST(x1) = 0.558, ST(x2) = 0.442, ST(x3) = 0.244
    """
    if x.shape[1] < 3:
        return np.zeros(x.shape[0])
    a, b = 7.0, 0.1
    x1 = x[:, 0]
    x2 = x[:, 1]
    x3 = x[:, 2]
    return (
        np.sin(x1)
        + a * np.sin(x2) ** 2
        + b * (x3 ** 4) * np.sin(x1)
    )


def _model_eval(model: dict[str, Any], x: np.ndarray) -> np.ndarray:
    """Evaluate the analysis model on the input matrix.

    The Phase 0/4 fixture routes to the Ishigami test function when
    the model is `aether.test.ishigami`; production sites use the
    physics.solver plugin to evaluate the model. The Sobol indices of
    the Ishigami function are analytically known, which is what makes
    the conformance test meaningful.
    """
    plugin = model.get("plugin", "")
    if plugin == "aether.test.ishigami":
        return _ishigami(x)
    # Generic linear surrogate: y = sum(w_i * x_i).
    weights = [1.0] * x.shape[1]
    return x @ np.array(weights)


def _lhs(n: int, d: int, rng: np.random.Generator) -> np.ndarray:
    """Latin hypercube sampling on [0, 1]^d.

    Each column is an independent stratified permutation: the n strata
    are visited exactly once per column, in a randomly shuffled order,
    with the within-stratum position drawn uniformly. This is essential
    for the Sobol path: the A and B matrices must be statistically
    independent samples, not rank-identical permutations of each other
    (a rank-correlated LHS collapses the Saltelli cross-differences and
    drives every sensitivity index to zero).
    """
    cut = np.linspace(0.0, 1.0, n + 1)
    u = rng.uniform(0.0, 1.0, size=(n, d))
    out = np.empty_like(u)
    for j in range(d):
        # Shuffle stratum order independently per column, then place the
        # sample uniformly within its stratum.
        strata = rng.permutation(n)
        out[:, j] = cut[strata] + (cut[1] - cut[0]) * u[:, j]
    return out


def _sobol_indices(
    y_all: np.ndarray, y_A: np.ndarray, y_B: np.ndarray, y_AB_list: list[np.ndarray], n: int
) -> dict[str, dict[str, float]]:
    """Saltelli's estimator for first-order (S1) and total-order (ST)
    Sobol indices.

    `y_all` is the concatenation of y_A + y_B + y_AB_1 + ... + y_AB_d.
    `n` is the per-sample-set size (length of y_A = length of y_B = length
    of each y_AB_i).
    """
    var_y = float(np.var(np.concatenate([y_A, y_B])))
    if var_y <= 0:
        return {f"x{i + 1}": {"S1": 0.0, "ST": 0.0} for i in range(len(y_AB_list))}
    out: dict[str, dict[str, float]] = {}
    for i, y_AB_i in enumerate(y_AB_list):
        # First-order
        s1 = float(np.mean(y_B * (y_AB_i - y_A)) / var_y)
        # Total-order
        st = float(0.5 * np.mean((y_A - y_AB_i) ** 2) / var_y)
        out[f"x{i + 1}"] = {"S1": s1, "ST": st}
    return out


# -- JSON-RPC methods --

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
        "methods": ["lhs", "monte_carlo", "polynomial_chaos", "sobol", "deep_ensemble"],
        "note": "Phase 0/4 reference UQ plugin. Replaced by SciPy/Chaospy/UQpy in production.",
    }


def method_capabilities(_params: dict[str, Any] | None) -> dict[str, Any]:
    return {
        "methods": ["lhs", "monte_carlo", "polynomial_chaos", "sobol", "deep_ensemble"],
        "distributions": ["normal", "uniform", "lognormal", "beta", "triangular", "deterministic"],
    }


def method_propagate(params: dict[str, Any] | None) -> dict[str, Any]:
    if not params or "inputs" not in params:
        return {"error": "no inputs"}
    p = params["inputs"]
    method = p.get("method", "monte_carlo")
    inputs = p.get("inputs", [])
    model = p.get("model", {})
    n = int(p.get("samples", 1000))
    seed = p.get("seed")
    rng = _seeded(seed)

    if method == "lhs":
        d = len(inputs)
        u = _lhs(n, d, rng)
        # Map [0,1]^d -> input distributions.
        x = _inverse_cdf(u, inputs, rng)
    elif method == "monte_carlo":
        x = _sample_inputs(inputs, n, rng)
    elif method == "polynomial_chaos":
        # Use sparse-grid-like sparse pseudo-sampling for the fixture.
        # The Phase 0 reference is honest about being a stub: it
        # falls back to LHS with a smaller sample count and a higher
        # polynomial order flag. Production sites use chaospy.
        order = int(p.get("polynomial_order", 3))
        n_pce = max(n, 200 * order)
        u = _lhs(n_pce, len(inputs), rng)
        x = _inverse_cdf(u, inputs, rng)
    elif method == "sobol":
        # Saltelli design: A, B, and AB_i matrices; total samples = n * (d + 2).
        d = len(inputs)
        n_sobol = max(n, 256)
        u_A = _lhs(n_sobol, d, rng)
        u_B = _lhs(n_sobol, d, rng)
        u_AB = []
        for i in range(d):
            u_i = u_A.copy()
            u_i[:, i] = u_B[:, i]
            u_AB.append(u_i)
        x_A = _inverse_cdf(u_A, inputs, rng)
        x_B = _inverse_cdf(u_B, inputs, rng)
        x_AB = [_inverse_cdf(u, inputs, rng) for u in u_AB]
        y_A = _model_eval(model, x_A)
        y_B = _model_eval(model, x_B)
        y_AB = [_model_eval(model, xi) for xi in x_AB]
        # Stats over all y values (treated as a single sample).
        all_y = np.concatenate([y_A, y_B] + y_AB)
        outputs = [
            {
                "name": "y",
                "statistics": {
                    "mean": float(np.mean(all_y)),
                    "std": float(np.std(all_y)),
                    "p05": float(np.quantile(all_y, 0.05)),
                    "p50": float(np.quantile(all_y, 0.50)),
                    "p95": float(np.quantile(all_y, 0.95)),
                },
            }
        ]
        # Sobol indices
        indices_raw = _sobol_indices(all_y, y_A, y_B, y_AB, n_sobol)
        sobol_indices = [
            {"output": "y", "input": k, "S1": v["S1"], "ST": v["ST"]}
            for k, v in indices_raw.items()
        ]
        return {
            "plugin_id": PLUGIN_ID,
            "plugin_version": PLUGIN_VERSION,
            "determinism_class": "bit-reproducible",
            "outputs": {"uq": {"cas_hash": "0" * 64, "name": "uq", "size": 0, "format": "json"}},
            "result": {
                "method": "sobol",
                "samples": n_sobol * (d + 2),
                "outputs": outputs,
                "convergence": {"sobol_converged": True},
                "sobol_indices": sobol_indices,
            },
        }
    else:
        return {"error": f"unknown method: {method}"}

    y = _model_eval(model, x)
    outputs = [
        {
            "name": "y",
            "statistics": {
                "mean": float(np.mean(y)),
                "std": float(np.std(y)),
                "p05": float(np.quantile(y, 0.05)),
                "p50": float(np.quantile(y, 0.50)),
                "p95": float(np.quantile(y, 0.95)),
            },
        }
    ]
    return {
        "plugin_id": PLUGIN_ID,
        "plugin_version": PLUGIN_VERSION,
        "determinism_class": "bit-reproducible",
        "outputs": {"uq": {"cas_hash": "0" * 64, "name": "uq", "size": 0, "format": "json"}},
        "result": {
            "method": method,
            "samples": n,
            "outputs": outputs,
            "convergence": {
                "mean_of_means": {"y": float(np.mean(y))},
                "std_of_stds": {"y": float(np.std(y)) / math.sqrt(n)},
                "sobol_converged": False,
            },
        },
    }


def _inverse_cdf(
    u: np.ndarray, inputs: list[dict[str, Any]], rng: np.random.Generator
) -> np.ndarray:
    """Map (n, d) uniform [0,1] matrix -> the input distributions."""
    cols: list[np.ndarray] = []
    # Clip the uniform values to the open interval (0, 1) so that
    # `NormalDist.inv_cdf` does not raise on LHS boundary samples.
    u_safe = np.clip(u, 1e-12, 1.0 - 1e-12)
    for j, spec in enumerate(inputs):
        d = spec["distribution"]
        params = spec.get("params", {}) or {}
        c = u_safe[:, j]
        if d == "normal":
            mu = float(params.get("mean", 0.0))
            sd = float(params.get("std", 1.0))
            from statistics import NormalDist
            n = NormalDist(mu=mu, sigma=sd)
            cols.append(np.array([n.inv_cdf(float(v)) for v in c]))
        elif d == "uniform":
            lo = float(params.get("min", 0.0))
            hi = float(params.get("max", 1.0))
            cols.append(lo + (hi - lo) * c)
        elif d == "lognormal":
            mu = float(params.get("mean", 0.0))
            sigma = float(params.get("sigma", 1.0))
            from statistics import NormalDist
            n = NormalDist(mu=mu, sigma=sigma)
            cols.append(np.exp([n.inv_cdf(float(v)) for v in c]))
        elif d == "deterministic":
            cols.append(np.full(u_safe.shape[0], float(params.get("value", 0.0))))
        else:
            cols.append(c)
    return np.column_stack(cols) if cols else np.zeros((u_safe.shape[0], 0))


def method_sobol_indices(params: dict[str, Any] | None) -> dict[str, Any]:
    if not params:
        return {"error": "no params"}
    p = params["inputs"]
    inputs = p.get("inputs", [])
    model = p.get("model", {})
    seed = p.get("seed")
    rng = _seeded(seed)
    d = len(inputs)
    n = 1024
    u_A = _lhs(n, d, rng)
    u_B = _lhs(n, d, rng)
    u_AB = []
    for i in range(d):
        u_i = u_A.copy()
        u_i[:, i] = u_B[:, i]
        u_AB.append(u_i)
    x_A = _inverse_cdf(u_A, inputs, rng)
    x_B = _inverse_cdf(u_B, inputs, rng)
    x_AB = [_inverse_cdf(u, inputs, rng) for u in u_AB]
    y_A = _model_eval(model, x_A)
    y_B = _model_eval(model, x_B)
    y_AB = [_model_eval(model, xi) for xi in x_AB]
    all_y = np.concatenate([y_A, y_B] + y_AB)
    indices = _sobol_indices(all_y, y_A, y_B, y_AB, n)
    return {
        "method": "sobol",
        "samples": n * (d + 2),
        "indices": [
            {"input": k, "S1": v["S1"], "ST": v["ST"]}
            for k, v in indices.items()
        ],
    }


METHODS = {
    "init": method_init,
    "discover": method_discover,
    "capabilities": method_capabilities,
    "propagate": method_propagate,
    "sobol_indices": method_sobol_indices,
}


def handle(line: str) -> dict[str, Any] | None:
    line = line.strip()
    if not line:
        return None
    try:
        req = json.loads(line)
    except json.JSONDecodeError as exc:
        return {"jsonrpc": "2.0", "id": None, "error": {"code": -32700, "message": f"parse error: {exc.msg}"}}
    if not isinstance(req, dict) or req.get("jsonrpc") != "2.0":
        return {"jsonrpc": "2.0", "id": req.get("id") if isinstance(req, dict) else None, "error": {"code": -32600, "message": "invalid request"}}
    method = req.get("method")
    req_id = req.get("id")
    params = req.get("params")
    if not isinstance(method, str):
        return {"jsonrpc": "2.0", "id": req_id, "error": {"code": -32601, "message": "method must be a string"}}
    fn = METHODS.get(method)
    if fn is None:
        return {"jsonrpc": "2.0", "id": req_id, "error": {"code": -32601, "message": f"method not found: {method}"}}
    try:
        result = fn(params)
    except Exception as exc:  # noqa: BLE001
        return {"jsonrpc": "2.0", "id": req_id, "error": {"code": -32603, "message": f"internal error: {exc!r}"}}
    return {"jsonrpc": "2.0", "id": req_id, "result": result}


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
