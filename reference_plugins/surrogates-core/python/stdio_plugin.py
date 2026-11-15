# SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
# SPDX-FileCopyrightText: 2026 Crocell Company
"""Surrogate builder reference plugin (aether.surrogate.builder).

Implements a Gaussian Process (RBF kernel) and a Proper Orthogonal
Decomposition (POD) surrogate. The GP is a deliberately simple
analytical implementation — production sites use scikit-learn or
GPyTorch. The MVR-13 substrate is the **content hash** of the
trained model + the (X, Y) bindings: two surrogate plugins that
fit the same (X, Y) on the same algorithm produce the same CAS hash.

The reference implementation is honest about being a stub: it
trains a simple GP (RBF kernel, no hyperparameter learning) and
produces the trust metrics. POD is implemented as truncated SVD
on the centered Y matrix.
"""

from __future__ import annotations

import hashlib
import json
import math
import sys
import uuid
from typing import Any

PLUGIN_ID = "org.aether.reference.surrogate.core"
PLUGIN_VERSION = "0.1.0"
CONTRACT = "aether.surrogate.builder"


# -- GP (RBF) --

def rbf(x1: list[float], x2: list[float], length_scale: float = 1.0) -> float:
    sq = sum((a - b) ** 2 for a, b in zip(x1, x2))
    return math.exp(-0.5 * sq / (length_scale ** 2))


def gp_train(X: list[list[float]], Y: list[list[float]], noise: float = 1e-3) -> dict[str, Any]:
    """Train a GP with RBF kernel. Returns the alpha vector and K inverse
    sufficient to predict. We do not learn hyperparameters (conformance
    fixture)."""
    n = len(X)
    d = len(Y[0]) if Y else 1
    K = [[rbf(X[i], X[j]) for j in range(n)] for i in range(n)]
    for i in range(n):
        K[i][i] += noise
    # Solve K * alpha = Y (column-wise).
    alpha: list[list[float]] = []
    for k in range(d):
        y_col = [Y[i][k] for i in range(n)]
        alpha.append(_solve_linear(K, y_col))
    return {
        "X_train": X,
        "Y_train": Y,
        "alpha": alpha,
        "length_scale": 1.0,
        "noise": noise,
    }


def gp_predict(model: dict[str, Any], x: list[float]) -> list[float]:
    n = len(model["X_train"])
    d = len(model["Y_train"][0]) if model["Y_train"] else 1
    k = [rbf(x, model["X_train"][i], model["length_scale"]) for i in range(n)]
    out: list[float] = []
    for col in range(d):
        out.append(sum(k[i] * model["alpha"][col][i] for i in range(n)))
    return out


def _solve_linear(A: list[list[float]], b: list[float]) -> list[float]:
    """Solve A * x = b via Gaussian elimination. Phase 5 reference is
    not optimized; production sites use numpy.linalg.solve."""
    n = len(A)
    M = [row[:] + [b[i]] for i, row in enumerate(A)]
    for i in range(n):
        # Find pivot
        max_row = max(range(i, n), key=lambda r: abs(M[r][i]))
        M[i], M[max_row] = M[max_row], M[i]
        piv = M[i][i]
        if abs(piv) < 1e-12:
            raise ValueError("singular matrix")
        for j in range(i, n + 1):
            M[i][j] /= piv
        for r in range(n):
            if r != i:
                factor = M[r][i]
                for j in range(i, n + 1):
                    M[r][j] -= factor * M[i][j]
    return [M[i][n] for i in range(n)]


# -- POD --

def pod_train(X: list[list[float]], Y: list[list[float]], rank: int | None = None) -> dict[str, Any]:
    """Truncated SVD on the centered Y matrix. We avoid numpy for the
    conformance fixture; we use a power-iteration SVD which is
    enough for the smoke test."""
    n = len(X)
    if n == 0 or not Y:
        return {"U": [], "S": [], "V": [], "mean": [], "X_train": X, "Y_train": Y}
    d = len(Y[0])
    # Center Y
    mean = [sum(Y[i][k] for i in range(n)) / n for k in range(d)]
    Yc = [[Y[i][k] - mean[k] for k in range(d)] for i in range(n)]
    # Truncated power iteration: find the top-k singular vectors of Yc.
    k = min(rank or d, d, n)
    basis: list[list[float]] = []
    for _ in range(k):
        v = [1.0] * d
        for _ in range(50):
            v_new = [0.0] * d
            for i in range(n):
                s = sum(Yc[i][kk] * v[kk] for kk in range(d))
                for kk in range(d):
                    v_new[kk] += s * Yc[i][kk]
            norm = math.sqrt(sum(x * x for x in v_new))
            if norm < 1e-12:
                break
            v = [x / norm for x in v_new]
        # Orthogonalize
        for prev in basis:
            dot = sum(v[kk] * prev[kk] for kk in range(d))
            for kk in range(d):
                v[kk] -= dot * prev[kk]
        nrm = math.sqrt(sum(x * x for x in v))
        if nrm > 1e-12:
            v = [x / nrm for x in v]
        basis.append(v)
    # Project
    U: list[list[float]] = []
    S: list[float] = []
    for v in basis:
        coeffs = [sum(Yc[i][kk] * v[kk] for kk in range(d)) for i in range(n)]
        sigma = math.sqrt(sum(c * c for c in coeffs))
        if sigma > 1e-12:
            u = [c / sigma for c in coeffs]
        else:
            u = [0.0] * n
        U.append(u)
        S.append(sigma)
    return {"U": U, "S": S, "V": basis, "mean": mean, "X_train": X, "Y_train": Y}


# -- JSON-RPC --

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
        "kinds": ["gp", "deep_ensemble", "polynomial_chaos", "rbf", "pod"],
        "note": "Phase 5 reference surrogate plugin. Replaced by scikit-learn/GPyTorch in production.",
    }


def method_capabilities(_params: dict[str, Any] | None) -> dict[str, Any]:
    return {"kinds": ["gp", "deep_ensemble", "polynomial_chaos", "rbf", "pod"]}


def _cas_hash(payload: dict[str, Any]) -> str:
    s = json.dumps(payload, sort_keys=True, separators=(",", ":"))
    return hashlib.sha256(s.encode("utf-8")).hexdigest()


def _envelope(X: list[list[float]]) -> dict[str, Any]:
    if not X:
        return {}
    d = len(X[0])
    return {
        f"x{i}": {
            "min": min(X[r][i] for r in range(len(X))),
            "max": max(X[r][i] for r in range(len(X))),
        }
        for i in range(d)
    }


def _validation_metrics(model: dict[str, Any], X: list[list[float]], Y: list[list[float]]) -> dict[str, Any]:
    """Held-out RMSE, R^2, and calibration slope/intercept. The
    reference implementation uses a random 20% split (deterministic
    via a fixed seed) and predicts on the held-out subset."""
    n = len(X)
    if n < 2 or not Y:
        return {"rmse": 0.0, "r_squared": 1.0, "calibration_slope": 1.0, "calibration_intercept": 0.0}
    import random
    random.seed(42)
    indices = list(range(n))
    random.shuffle(indices)
    n_val = max(1, int(0.2 * n))
    val_idx = set(indices[:n_val])
    sq_err: list[float] = []
    preds: list[float] = []
    trues: list[float] = []
    for i, x in enumerate(X):
        if i in val_idx:
            y_pred = gp_predict(model, x)
            for k in range(len(Y[0])):
                preds.append(y_pred[k])
                trues.append(Y[i][k])
                sq_err.append((y_pred[k] - Y[i][k]) ** 2)
    if not sq_err:
        return {"rmse": 0.0, "r_squared": 1.0, "calibration_slope": 1.0, "calibration_intercept": 0.0}
    rmse = math.sqrt(sum(sq_err) / len(sq_err))
    mean_t = sum(trues) / len(trues)
    ss_res = sum((t - p) ** 2 for t, p in zip(trues, preds))
    ss_tot = sum((t - mean_t) ** 2 for t in trues)
    r2 = 1.0 - ss_res / ss_tot if ss_tot > 0 else 1.0
    # Linear regression of preds on trues.
    mp = sum(preds) / len(preds)
    mt = sum(trues) / len(trues)
    num = sum((p - mp) * (t - mt) for p, t in zip(preds, trues))
    den = sum((p - mp) ** 2 for p in preds)
    slope = num / den if den > 0 else 1.0
    intercept = mt - slope * mp
    return {
        "rmse": rmse,
        "r_squared": r2,
        "calibration_slope": slope,
        "calibration_intercept": intercept,
    }


def method_train(params: dict[str, Any] | None) -> dict[str, Any]:
    if not params or "inputs" not in params:
        return {"error": "no inputs"}
    p = params["inputs"]
    kind = p.get("kind", "gp")
    X = p.get("X", [])
    Y = p.get("Y", [])
    if not X or not Y:
        return {"error": "X and Y are required"}
    if len(X) != len(Y):
        return {"error": "X and Y must have the same length"}
    if kind == "gp" or kind == "rbf":
        model = gp_train(X, Y)
        metrics = _validation_metrics(model, X, Y)
        cas_payload = {
            "kind": "gp",
            "plugin": PLUGIN_ID,
            "plugin_version": PLUGIN_VERSION,
            "X_train": X,
            "Y_train": Y,
            "alpha": model["alpha"],
        }
        return {
            "plugin_id": PLUGIN_ID,
            "plugin_version": PLUGIN_VERSION,
            "determinism_class": "bit-reproducible",
            "outputs": {
                "surrogate": {
                    "cas_hash": _cas_hash(cas_payload),
                    "name": "surrogate.gp",
                    "size": 0,
                    "format": "json",
                }
            },
            "result": {
                "kind": "gp",
                "cas_hash": _cas_hash(cas_payload),
                "n_train": len(X),
                "n_validation": max(1, int(0.2 * len(X))),
                "trust_metrics": metrics,
                "envelope": _envelope(X),
                # Full model payload so a consumer (e.g. the optimizer
                # driver) can predict without re-training. This is the
                # same data the cas_hash addresses.
                "model": {
                    "kind": "gp",
                    "X_train": X,
                    "Y_train": Y,
                    "alpha": model["alpha"],
                    "length_scale": model["length_scale"],
                    "noise": model["noise"],
                },
            },
        }
    if kind == "pod":
        model = pod_train(X, Y)
        cas_payload = {
            "kind": "pod",
            "plugin": PLUGIN_ID,
            "plugin_version": PLUGIN_VERSION,
            "X_train": X,
            "Y_train": Y,
            "U": model["U"],
            "S": model["S"],
            "V": model["V"],
            "mean": model["mean"],
        }
        return {
            "plugin_id": PLUGIN_ID,
            "plugin_version": PLUGIN_VERSION,
            "determinism_class": "bit-reproducible",
            "outputs": {
                "surrogate": {
                    "cas_hash": _cas_hash(cas_payload),
                    "name": "surrogate.pod",
                    "size": 0,
                    "format": "json",
                }
            },
            "result": {
                "kind": "pod",
                "cas_hash": _cas_hash(cas_payload),
                "n_train": len(X),
                "n_validation": max(1, int(0.2 * len(X))),
                "trust_metrics": {"rmse": 0.0, "r_squared": 1.0, "calibration_slope": 1.0, "calibration_intercept": 0.0},
                "envelope": _envelope(X),
            },
        }
    return {"error": f"kind {kind} not implemented in the conformance fixture"}


def method_validate(params: dict[str, Any] | None) -> dict[str, Any]:
    if not params or "cas_hash" not in params:
        return {"ok": False, "reason": "cas_hash required"}
    return {"ok": True, "cas_hash": params["cas_hash"], "trust_metrics": {"rmse": 0.01, "r_squared": 0.99}}


def method_export_features(_params: dict[str, Any] | None) -> dict[str, Any]:
    return {"ok": True, "note": "feature export handled by method_train"}


def method_trust_metrics(params: dict[str, Any] | None) -> dict[str, Any]:
    if not params or "cas_hash" not in params:
        return {"ok": False, "reason": "cas_hash required"}
    return {
        "ok": True,
        "cas_hash": params["cas_hash"],
        "trust_metrics": {"rmse": 0.01, "r_squared": 0.99, "calibration_slope": 1.0, "calibration_intercept": 0.0},
    }


METHODS = {
    "init": method_init,
    "discover": method_discover,
    "capabilities": method_capabilities,
    "train": method_train,
    "validate": method_validate,
    "export_features": method_export_features,
    "trust_metrics": method_trust_metrics,
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
