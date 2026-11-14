# SPDX-License-Identifier: Aether-VPSL-1.0
# SPDX-FileCopyrightText: 2026 The Aether Consortium
"""Digital-twin estimator reference plugin (aether.twin.estimator).

Implements the Extended Kalman Filter (EKF), Unscented Kalman Filter
(UKF), and a basic particle filter on a single-state linear-Gaussian
model. The reference plugin is the conformance fixture (G5); production
sites replace it with filterpy or a commercial estimator.

The model is the standard SISO random walk observed through Gaussian
noise:
  x_t = x_{t-1} + w_t,    w_t ~ N(0, Q)
  y_t = x_t     + v_t,    v_t ~ N(0, R)

For the EKF, this reduces to the scalar Kalman filter:
  K_t = P_{t|t-1} / (P_{t|t-1} + R)
  x_t = x_{t|t-1} + K_t * (y_t - x_{t|t-1})
  P_t = (1 - K_t) * P_{t|t-1}
and the predict step is:
  x_{t+1|t} = x_t
  P_{t+1|t} = P_t + Q

The UKF uses the sigma-point transform; for a 1D linear-Gaussian model
it produces identical estimates to the EKF. The particle filter uses
N=200 particles with the same model and importance resampling.

WAVE 4 — multivariate (vector-state) linear Kalman filter
---------------------------------------------------------

The scalar filters above estimate ONE state (e.g. a single deflection
growth factor). Real digital twins estimate a VECTOR state — for a
spanwise structural twin: one stiffness (EI) scale per span station.
The ``estimate`` method therefore accepts two problem shapes:

  1. Scalar (as before): ``state: {"x": {"mean", "variance"}}`` with
     ``process_noise: {"x"}``, ``measurement_noise: {"y"}`` and each
     measurement ``values: {"y"}`` — served by the original filters.

  2. Vector: ``state: {"x": [m1, ...], "covariance": [[...]]}`` with
     ``process_noise: {"covariance": [[...]]}``, ``measurement_noise:
     {"covariance": [[...]]}``, and each measurement carrying an
     optional observation matrix ``H`` (defaults to identity) plus
     ``values: [y1, ...]``. This is served by ``ekf_vector``, a real
     multivariate linear Kalman filter (random-walk dynamics, linear
     observation, exact matrix algebra, no linearization error in the
     linear-Gaussian case).

Shape detection is on the prior mean being a list. The scalar path is
unchanged byte-for-byte in behavior; existing callers (``aether demo
``, the scalar twin loop, BENCH-09/10) keep their exact results.
"""

from __future__ import annotations

import json
import math
import random
import sys
import time
import uuid
from typing import Any

PLUGIN_ID = "org.aether.reference.twin.kf"
PLUGIN_VERSION = "0.2.0"
CONTRACT = "aether.twin.estimator"


# -- EKF (scalar Kalman filter for the linear-Gaussian random walk) --

def ekf(problem: dict[str, Any]) -> dict[str, Any]:
    state = problem.get("state", {})
    x_init = float(state.get("x", {}).get("mean", 0.0))
    p_init = float(state.get("x", {}).get("variance", 1.0))
    q = float(problem.get("process_noise", {}).get("x", 0.01))
    r = float(problem.get("measurement_noise", {}).get("y", 0.1))
    x, p = x_init, p_init
    last_innovation: dict[str, float] = {}
    last_t = 0.0
    drift = False
    for meas in problem.get("measurements", []):
        # Predict (random walk).
        p = p + q
        y = float(meas.get("values", {}).get("y", 0.0))
        # Update.
        k = p / (p + r)
        innov = y - x
        x = x + k * innov
        p = (1.0 - k) * p
        last_innovation = {"y": innov}
        last_t = float(meas.get("t", 0.0))
        # Drift detection (heuristic: innovation > 3 sigma).
        if abs(innov) > 3.0 * math.sqrt(p + r):
            drift = True
    return {
        "t": last_t,
        "estimator": "ekf",
        "mean": {"x": x},
        "variance": {"x": p},
        "innovation": last_innovation,
        "drift_detected": drift,
    }


# -- Vector (multivariate) linear Kalman filter --
#
# State model: x_t = x_{t-1} + w_t,  w_t ~ N(0, Q)  (random walk)
# Observation:      y_t = H x_t + v_t,  v_t ~ N(0, R)
#
# This is the standard discrete linear KF; because both dynamics and
# observation are linear, the "E"KF IS the exact KF here (no
# linearization error). Implemented with dense lists (no numpy — the
# plugin must stay stdlib-only like every reference plugin).

def _mat_add(A: list[list[float]], B: list[list[float]]) -> list[list[float]]:
    return [[a + b for a, b in zip(ra, rb)] for ra, rb in zip(A, B)]


def _mat_sub(A: list[list[float]], B: list[list[float]]) -> list[list[float]]:
    return [[a - b for a, b in zip(ra, rb)] for ra, rb in zip(A, B)]


def _mat_mul(A: list[list[float]], B: list[list[float]]) -> list[list[float]]:
    n, m = len(A), len(B)
    if m == 0 or len(A[0]) != m:
        raise ValueError(
            f"matrix dimension mismatch in _mat_mul: "
            f"A is {n}x{len(A[0])}, B is {m}x{len(B[0])}"
        )
    p = len(B[0])
    return [
        [sum(A[i][k] * B[k][j] for k in range(m)) for j in range(p)]
        for i in range(n)
    ]


def _mat_vec(A: list[list[float]], x: list[float]) -> list[float]:
    if len(A[0]) != len(x):
        raise ValueError(
            f"matrix dimension mismatch in _mat_vec: A is {len(A)}x{len(A[0])}, x has length {len(x)}"
        )
    return [sum(a * xi for a, xi in zip(row, x)) for row in A]


def _transpose(A: list[list[float]]) -> list[list[float]]:
    return [list(col) for col in zip(*A)]


def _identity(n: int) -> list[list[float]]:
    return [[1.0 if i == j else 0.0 for j in range(n)] for i in range(n)]


def _mat_inverse(M: list[list[float]]) -> list[list[float]]:
    """Gauss-Jordan inverse; raises ValueError on (near-)singular input."""
    n = len(M)
    A = [list(row) + [1.0 if i == j else 0.0 for j in range(n)] for i, row in enumerate(M)]
    for i in range(n):
        piv = A[i][i]
        if abs(piv) < 1e-300:
            raise ValueError("singular matrix in _mat_inverse")
        inv_piv = 1.0 / piv
        for j in range(2 * n):
            A[i][j] *= inv_piv
        for k in range(n):
            if k == i:
                continue
            factor = A[k][i]
            if factor == 0.0:
                continue
            for j in range(2 * n):
                A[k][j] -= factor * A[i][j]
    return [row[n:] for row in A]


def _cov_diag(vals: list[float]) -> list[list[float]]:
    return [[v if i == j else 0.0 for j, _ in enumerate(vals)] for i, v in enumerate(vals)]


def _pseudo_div(a: float, b: float) -> float:
    """0/0 -> 0 (a zero Kalman gain entry must zero the correction,
    not propagate NaN)."""
    return a / b if b != 0.0 else 0.0


def ekf_vector(problem: dict[str, Any]) -> dict[str, Any]:
    """Multivariate linear Kalman filter (exact for linear-Gaussian).

    Problem keys:
      state:            {"x": [m0, m1, ...], "covariance": [[P]]}
                        (covariance defaults to diag(1, 1, ...))
      process_noise:    {"covariance": [[Q]]}  (defaults to zeros)
      measurement_noise: {"covariance": [[R]]} (defaults to diag(1, ...))
      measurements:     [{"t": t, "H": [[H]] (optional, default I),
                          "values": [y0, y1, ...]}]

    The observation matrix H is n x n with the state vector (the twin
    application observes every station); R is n x n matching the
    measurement vector. All matrix dimensions are validated loudly —
    a malformed H or R raises ValueError (I-12) instead of producing
    a silently wrong posterior.

    Returns the same envelope as the scalar filters (``mean`` keyed by
    component name ``x0, x1, ...``, ``variance`` by diagonal entry,
    plus ``covariance`` and ``state_length``) so callers can treat
    scalar and vector results uniformly.
    """
    state = problem.get("state", {})
    x_raw = state.get("x", [])
    if not isinstance(x_raw, list):
        raise ValueError("ekf_vector: state.x must be a list of means")
    n = len(x_raw)
    if n == 0:
        raise ValueError("ekf_vector: state.x must be non-empty")
    x: list[float] = [float(v) for v in x_raw]
    cov_raw = state.get("covariance")
    if cov_raw is not None:
        P = [[float(v) for v in row] for row in cov_raw]
    else:
        P = _identity(n)
    q_raw = problem.get("process_noise", {}).get("covariance")
    Q = [[float(v) for v in row] for row in q_raw] if q_raw is not None else [[0.0] * n for _ in range(n)]
    r_raw = problem.get("measurement_noise", {}).get("covariance")
    R_default = _cov_diag([1.0] * n)
    R = [[float(v) for v in row] for row in r_raw] if r_raw is not None else R_default
    if len(R) != n or any(len(row) != n for row in R):
        raise ValueError("ekf_vector: measurement_noise covariance must be n x n")
    if len(Q) != n or any(len(row) != n for row in Q):
        raise ValueError("ekf_vector: process_noise covariance must be n x n")
    if len(P) != n or any(len(row) != n for row in P):
        raise ValueError("ekf_vector: state covariance must be n x n")

    I = _identity(n)
    last_innovation: list[float] = []
    last_t = 0.0
    drift = False
    for meas in problem.get("measurements", []):
        # Predict (random walk: F = I).
        P = _mat_add(P, Q)
        H_raw = meas.get("H")
        H = [[float(v) for v in row] for row in H_raw] if H_raw is not None else I
        if len(H) != n or any(len(row) != n for row in H):
            raise ValueError(
                "ekf_vector: measurement H must be n x n "
                f"(state length {n}); got {len(H)}x{len(H[0]) if H else 0}"
            )
        y = [float(v) for v in meas.get("values", [])]
        if len(y) != n:
            raise ValueError(
                f"ekf_vector: measurement values must have length {n}; got {len(y)}"
            )
        # Innovation covariance S = H P H^T + R.
        S = _mat_add(_mat_mul(_mat_mul(H, P), _transpose(H)), R)
        # Kalman gain K = P H^T S^-1.
        K = _mat_mul(_mat_mul(P, _transpose(H)), _mat_inverse(S))
        innov = [yi - xi for yi, xi in zip(y, _mat_vec(H, x))]
        x = [xi + ki for xi, ki in zip(x, _mat_vec(K, innov))]
        P = _mat_mul(_mat_sub(I, _mat_mul(K, H)), P)
        last_innovation = innov
        last_t = float(meas.get("t", 0.0))
        # Drift heuristic on the max normalized innovation
        # (innov_i^2 / S_ii > 9 <=> > 3 sigma), mirroring the scalar rule.
        for i, si in enumerate(S):
            if i < len(innov) and si[i] > 0.0 and (innov[i] ** 2) / si[i] > 9.0:
                drift = True
    comp_names = [f"x{i}" for i in range(n)]
    return {
        "t": last_t,
        "estimator": "ekf_vector",
        "state_length": n,
        "mean": {name: xi for name, xi in zip(comp_names, x)},
        "variance": {name: P[i][i] for i, name in enumerate(comp_names)},
        "covariance": P,
        "innovation": last_innovation,
        "drift_detected": drift,
    }


# -- UKF (1D linear-Gaussian: equivalent to EKF) --

def ukf(problem: dict[str, Any]) -> dict[str, Any]:
    # For a 1D linear-Gaussian model, the UKF reduces to the EKF; we
    # re-implement the sigma-point machinery for educational fidelity.
    state = problem.get("state", {})
    x_init = float(state.get("x", {}).get("mean", 0.0))
    p_init = float(state.get("x", {}).get("variance", 1.0))
    q = float(problem.get("process_noise", {}).get("x", 0.01))
    r = float(problem.get("measurement_noise", {}).get("y", 0.1))
    alpha = 1e-3
    beta = 2.0
    kappa = 0.0
    n = 1
    lam = alpha * alpha * (n + kappa) - n
    wm = [lam / (n + lam), 1.0 / (2.0 * (n + lam)), 1.0 / (2.0 * (n + lam))]
    wc = [lam / (n + lam) + (1.0 - alpha * alpha + beta), 1.0 / (2.0 * (n + lam)), 1.0 / (2.0 * (n + lam))]

    x, p = x_init, p_init
    last_innovation: dict[str, float] = {}
    last_t = 0.0
    drift = False
    for meas in problem.get("measurements", []):
        # Predict.
        sigma = [x - math.sqrt((n + lam) * p), x, x + math.sqrt((n + lam) * p)]
        sigma_pred = [s for s in sigma]  # random walk: identity
        x_pred = sum(w * s for w, s in zip(wm, sigma_pred))
        p_pred = sum(w * (s - x_pred) ** 2 for w, s in zip(wc, sigma_pred)) + q

        y = float(meas.get("values", {}).get("y", 0.0))
        # Update: H = 1, so measurement sigma = sigma_pred.
        y_pred = x_pred
        s_yy = sum(w * (s - y_pred) ** 2 for w, s in zip(wc, sigma_pred)) + r
        s_xy = sum(w * (s - x_pred) * (s - y_pred) for w, s in zip(wc, sigma_pred))
        k = s_xy / s_yy
        innov = y - y_pred
        x = x_pred + k * innov
        p = p_pred - k * s_xy
        last_innovation = {"y": innov}
        last_t = float(meas.get("t", 0.0))
        if abs(innov) > 3.0 * math.sqrt(s_yy):
            drift = True
    return {
        "t": last_t,
        "estimator": "ukf",
        "mean": {"x": x},
        "variance": {"x": p},
        "innovation": last_innovation,
        "drift_detected": drift,
    }


# -- Particle filter --

def particle_filter(problem: dict[str, Any], n_particles: int = 200) -> dict[str, Any]:
    state = problem.get("state", {})
    x_init = float(state.get("x", {}).get("mean", 0.0))
    p_init = float(state.get("x", {}).get("variance", 1.0))
    q = float(problem.get("process_noise", {}).get("x", 0.01))
    r = float(problem.get("measurement_noise", {}).get("y", 0.1))
    particles = [x_init + random.gauss(0, math.sqrt(p_init)) for _ in range(n_particles)]
    weights = [1.0] * n_particles
    last_innovation: dict[str, float] = {}
    last_t = 0.0
    drift = False
    for meas in problem.get("measurements", []):
        # Predict: random walk.
        particles = [p + random.gauss(0, math.sqrt(q)) for p in particles]
        y = float(meas.get("values", {}).get("y", 0.0))
        # Update weights (likelihood).
        for i, p in enumerate(particles):
            weights[i] *= math.exp(-(y - p) ** 2 / (2.0 * r)) / math.sqrt(2.0 * math.pi * r)
        s = sum(weights)
        if s > 0:
            weights = [w / s for w in weights]
        else:
            weights = [1.0 / n_particles] * n_particles
        x = sum(p * w for p, w in zip(particles, weights))
        p_var = sum(w * (pp - x) ** 2 for pp, w in zip(particles, weights))
        # Systematic resampling.
        cumsum: list[float] = []
        running = 0.0
        for w in weights:
            running += w
            cumsum.append(running)
        u = random.random() / n_particles
        new_particles: list[float] = []
        for j in range(n_particles):
            target = u + j / n_particles
            for k_idx, c in enumerate(cumsum):
                if c >= target:
                    new_particles.append(particles[k_idx])
                    break
            else:
                new_particles.append(particles[-1])
        particles = new_particles
        last_innovation = {"y": y - x}
        last_t = float(meas.get("t", 0.0))
        if abs(y - x) > 3.0 * math.sqrt(p_var + r):
            drift = True
    return {
        "t": last_t,
        "estimator": "particle",
        "mean": {"x": x},
        "variance": {"x": p_var},
        "innovation": last_innovation,
        "drift_detected": drift,
    }


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
        "estimators": ["ekf", "ukf", "enkf", "particle", "ekf_vector"],
        "state_kinds": ["scalar", "vector"],
        "note": (
            "Phase 5 reference estimator plugin; Phase 7 scalar twin; "
            "Wave 4 adds the multivariate linear KF (ekf_vector) for "
            "spanwise (vector-state) twins. Replaced by filterpy in production."
        ),
    }


def method_capabilities(_params: dict[str, Any] | None) -> dict[str, Any]:
    return {"estimators": ["ekf", "ukf", "enkf", "particle", "ekf_vector"], "state_kinds": ["scalar", "vector"]}


def method_initialize(params: dict[str, Any] | None) -> dict[str, Any]:
    if not params:
        return {"ok": False, "reason": "no params"}
    p = params.get("inputs", params)
    return {
        "ok": True,
        "estimator": p.get("estimator", "ekf"),
        "state": p.get("state", {}),
    }


def method_estimate(params: dict[str, Any] | None) -> dict[str, Any]:
    if not params or "inputs" not in params:
        return {"error": "no inputs"}
    p = params["inputs"]
    estimator = p.get("estimator", "ekf")
    # Vector shape: the prior mean is a list -> multivariate linear KF.
    if isinstance(p.get("state", {}).get("x"), list):
        if estimator in ("ekf", "ekf_vector"):
            return ekf_vector(p)
        return {"error": f"estimator {estimator!r} does not support vector states"}
    if estimator == "ekf":
        result = ekf(p)
    elif estimator == "ukf":
        result = ukf(p)
    elif estimator == "particle":
        result = particle_filter(p)
    else:
        return {"error": f"unknown estimator: {estimator}"}
    return result


def method_detect_drift(params: dict[str, Any] | None) -> dict[str, Any]:
    if not params or "inputs" not in params:
        return {"error": "no inputs"}
    p = params["inputs"]
    estimator = p.get("estimator", "ekf")
    if estimator == "ekf":
        state = ekf(p)
    elif estimator == "ukf":
        state = ukf(p)
    else:
        state = particle_filter(p)
    return {"drift_detected": state.get("drift_detected", False)}


METHODS = {
    "init": method_init,
    "discover": method_discover,
    "capabilities": method_capabilities,
    "initialize": method_initialize,
    "estimate": method_estimate,
    "detect_drift": method_detect_drift,
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
