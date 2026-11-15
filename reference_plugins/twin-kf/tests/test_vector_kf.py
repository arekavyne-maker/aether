# SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
# SPDX-FileCopyrightText: 2026 Crocell Company
"""Wave 4: vector-state (multivariate) linear Kalman filter tests.

The estimator under test is ``ekf_vector`` in the twin-kf reference
plugin. Every expected value below is computed from the closed-form
Kalman algebra (scalar cases) or from an exact two-sensor identity
(spanwise recovery), NOT from the implementation itself.
"""

from __future__ import annotations

import importlib.util
import json
import math
import subprocess
import sys
from pathlib import Path

import pytest

PLUGIN = (
    Path(__file__).resolve().parents[1] / "python" / "stdio_plugin.py"
)
_spec = importlib.util.spec_from_file_location("twin_kf_stdio_plugin", PLUGIN)
assert _spec is not None and _spec.loader is not None
twin_kf = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(twin_kf)


# ---------------------------------------------------------------------
# 1. Scalar reduction: n=1, H=1 must equal the closed-form scalar KF.
# ---------------------------------------------------------------------

def test_scalar_reduction_matches_closed_form() -> None:
    """One measurement, one state, H=[1]: the multivariate filter must
    reproduce the closed-form scalar Kalman update exactly.

        K = P/(P+R),  x' = x + K (y - x),  P' = (1-K) P
    """
    x0, p0, q, r, y = 1.0, 0.01, 1e-6, 2.5e-5, 1.2
    res = twin_kf.ekf_vector({
        "state": {"x": [x0], "covariance": [[p0]]},
        "process_noise": {"covariance": [[q]]},
        "measurement_noise": {"covariance": [[r]]},
        "measurements": [{"t": 0, "values": [y]}],
    })
    p_pred = p0 + q
    k = p_pred / (p_pred + r)
    x_expected = x0 + k * (y - x0)
    p_expected = (1.0 - k) * p_pred
    assert res["state_length"] == 1
    assert res["mean"]["x0"] == pytest.approx(x_expected, rel=1e-15)
    assert res["variance"]["x0"] == pytest.approx(p_expected, rel=1e-15)
    assert res["estimator"] == "ekf_vector"


# ---------------------------------------------------------------------
# 2. Independent sensors average: with two independent sensors of equal
#    noise observing one true state, the posterior mean must equal the
#    sample mean exactly (a classical KF identity).
# ---------------------------------------------------------------------

def test_equal_variance_sensors_average_exactly() -> None:
    ys = [1.001, 0.999]
    r = 2.5e-5
    res = twin_kf.ekf_vector({
        "state": {"x": [1.0], "covariance": [[0.01]]},
        "process_noise": {"covariance": [[0.0]]},
        "measurement_noise": {"covariance": [[r]]},
        "measurements": [{"t": k, "H": [[1.0]], "values": [y]}
                         for k, y in enumerate(ys)],
    })
    assert res["mean"]["x0"] == pytest.approx(sum(ys) / len(ys), rel=1e-12)


# ---------------------------------------------------------------------
# 3. Zero-gain subset: an unobserved station is represented by a ZERO
#    row of H (H is n x n; the zero row contributes an R-only entry to
#    S, which stays invertible). The station's mean and variance must
#    remain EXACTLY at the prior while the observed station updates.
# ---------------------------------------------------------------------

def test_unobserved_station_keeps_prior_exactly() -> None:
    x0 = [1.0, 1.0]
    p0 = [[0.01, 0.0], [0.0, 0.01]]
    res = twin_kf.ekf_vector({
        "state": {"x": x0, "covariance": p0},
        "process_noise": {"covariance": [[0.0, 0.0], [0.0, 0.0]]},
        "measurement_noise": {"covariance": [[1e-5, 0.0], [0.0, 1e-5]]},
        "measurements": [{"t": 0, "H": [[1.0, 0.0], [0.0, 0.0]], "values": [1.2, 0.0]}],
    })
    assert res["mean"]["x1"] == 1.0
    assert res["variance"]["x1"] == pytest.approx(0.01, rel=1e-15)
    assert res["mean"]["x0"] == pytest.approx(1.0 + (0.01 / (0.01 + 1e-5)) * 0.2, rel=1e-12)
    # A non-conforming H must raise, not degrade silently (I-12).
    with pytest.raises(ValueError, match="H must be n x n"):
        twin_kf.ekf_vector({
            "state": {"x": [1.0, 1.0], "covariance": [[0.01, 0.0], [0.0, 0.01]]},
            "measurements": [{"t": 0, "H": [[1.0, 0.0]], "values": [1.2, 0.0]}],
        })


# ---------------------------------------------------------------------
# 4. Spanwise recovery (the twin use case): two stations observed
#    simultaneously with H = I and diagonal priors/noise. The posterior
#    is the exact per-station Bayesian precision-weighted mean
#
#        x_i' = (x_i/p0 + y_i/r) / (1/p0 + 1/r)
#
#    the posterior variance is p0*r/(p0+r) per station (strictly
#    smaller than the prior — information was gained), and stations
#    stay exactly uncorrelated (P_post off-diagonals 0).
# ---------------------------------------------------------------------

def test_two_station_spanwise_recovery_closed_form() -> None:
    prior = 1.0
    p0 = 0.01
    r = 2.5e-5
    ys = [1.15, 1.05]
    res = twin_kf.ekf_vector({
        "state": {"x": [prior, prior], "covariance": [[p0, 0.0], [0.0, p0]]},
        "process_noise": {"covariance": [[0.0, 0.0], [0.0, 0.0]]},
        "measurement_noise": {"covariance": [[r, 0.0], [0.0, r]]},
        "measurements": [{"t": 0, "values": ys}],
    })
    for i, y in enumerate(ys):
        expected = (prior / p0 + y / r) / (1.0 / p0 + 1.0 / r)
        assert res["mean"][f"x{i}"] == pytest.approx(expected, rel=1e-12)
        p_post = p0 * r / (p0 + r)
        assert res["variance"][f"x{i}"] == pytest.approx(p_post, rel=1e-12)
        assert res["variance"][f"x{i}"] < p0
    assert res["covariance"][0][1] == pytest.approx(0.0, abs=1e-18)
    assert res["covariance"][1][0] == pytest.approx(0.0, abs=1e-18)


# ---------------------------------------------------------------------
# 5. Correlated measurement noise is EXPLOITED, not ignored: with two
#    span stations whose sensors share the SAME noise (R off-diagonal
#    = r, fully correlated), the difference x0 - x1 is recovered
#    EXACTLY (posterior variance of the difference -> 0 in closed
#    form), while with independent noise it stays finite at
#    2*p0*r/(p0+r). This identity is only representable because the
#    filter carries full covariance matrices — the whole point of the
#    vector filter over per-station scalar filters.
# ---------------------------------------------------------------------

def test_correlated_measurement_noise_is_exploited_not_ignored() -> None:
    p0, r = 0.01, 1e-5
    base = {
        "state": {"x": [1.0, 1.0], "covariance": [[p0, 0.0], [0.0, p0]]},
        "process_noise": {"covariance": [[0.0, 0.0], [0.0, 0.0]]},
        "measurements": [{"t": 0, "values": [1.2, 1.1]}],
    }
    def diff_var(corr: float) -> float:
        res = twin_kf.ekf_vector({**base,
            "measurement_noise": {"covariance": [[r, corr], [corr, r]]}})
        cov = res["covariance"]
        return cov[0][0] + cov[1][1] - 2.0 * cov[0][1]

    var_indep = diff_var(0.0)
    var_shared = diff_var(r)  # fully shared sensor noise
    assert var_indep == pytest.approx(2.0 * p0 * r / (p0 + r), rel=1e-9)
    assert var_shared < 1e-18  # the difference channel is noiseless
    assert var_shared < var_indep


# ---------------------------------------------------------------------
# 6. Drift detection: a >3-sigma normalized innovation must raise the
#    drift flag; well-behaved noise must not.
# ---------------------------------------------------------------------

def test_drift_detection_fires_on_outlier_and_not_on_quiet_noise() -> None:
    quiet = {
        "state": {"x": [1.0, 1.0], "covariance": [[0.01, 0.0], [0.0, 0.01]]},
        "process_noise": {"covariance": [[0.0, 0.0], [0.0, 0.0]]},
        "measurement_noise": {"covariance": [[2.5e-5, 0.0], [0.0, 2.5e-5]]},
        "measurements": [{"t": 0, "values": [1.001, 0.999]}],
    }
    assert twin_kf.ekf_vector(quiet)["drift_detected"] is False
    outlier = {**quiet,
        "measurements": [{"t": 0, "values": [1.5, 0.9]}]}
    assert twin_kf.ekf_vector(outlier)["drift_detected"] is True


# ---------------------------------------------------------------------
# 7. Determinism: identical inputs -> byte-identical results (I-9).
# ---------------------------------------------------------------------

def test_repeated_invocation_is_bit_identical() -> None:
    problem = {
        "state": {"x": [1.0, 1.0, 1.0], "covariance": [[0.01, 0.0, 0.0], [0.0, 0.01, 0.0], [0.0, 0.0, 0.01]]},
        "process_noise": {"covariance": [[1e-6, 0.0, 0.0], [0.0, 1e-6, 0.0], [0.0, 0.0, 1e-6]]},
        "measurement_noise": {"covariance": [[2.5e-5, 0.0, 0.0], [0.0, 2.5e-5, 0.0], [0.0, 0.0, 2.5e-5]]},
        "measurements": [{"t": 0, "values": [1.01, 0.99, 1.005]},
                         {"t": 1, "values": [1.02, 1.00, 1.01]}],
    }
    a = json.dumps(twin_kf.ekf_vector(problem), sort_keys=True)
    b = json.dumps(twin_kf.ekf_vector(problem), sort_keys=True)
    assert a == b


# ---------------------------------------------------------------------
# 8. Malformed inputs fail loudly, never silently (I-12).
# ---------------------------------------------------------------------

def test_malformed_vector_inputs_raise() -> None:
    with pytest.raises(ValueError):
        twin_kf.ekf_vector({"state": {"x": []}})
    with pytest.raises(ValueError):
        twin_kf.ekf_vector({"state": {"x": 1.0}})  # scalar mean is not a vector


# ---------------------------------------------------------------------
# 9. Scalar path unchanged: the legacy scalar estimator must return the
#    same envelope as before Wave 4 (mean/variance keyed "x").
# ---------------------------------------------------------------------

def test_scalar_path_unchanged_envelope() -> None:
    res = twin_kf.ekf({
        "state": {"x": {"mean": 1.0, "variance": 0.01}},
        "process_noise": {"x": 1e-6},
        "measurement_noise": {"y": 2.5e-5},
        "measurements": [{"t": 0, "values": {"y": 1.2}}],
    })
    assert set(res["mean"].keys()) == {"x"}
    assert set(res["variance"].keys()) == {"x"}
    p_pred = 0.01 + 1e-6
    k = p_pred / (p_pred + 2.5e-5)
    assert res["mean"]["x"] == pytest.approx(1.0 + k * 0.2, rel=1e-12)


# ---------------------------------------------------------------------
# 10. Routing: method_estimate dispatches on the shape of the prior
#     mean; "ekf_vector" on a scalar state is an error.
# ---------------------------------------------------------------------

def test_estimate_routes_by_state_shape() -> None:
    vector = twin_kf.method_estimate({"inputs": {
        "estimator": "ekf_vector",
        "state": {"x": [1.0], "covariance": [[0.01]]},
        "measurement_noise": {"covariance": [[1e-5]]},
        "measurements": [{"t": 0, "values": [1.1]}],
    }})
    assert vector["estimator"] == "ekf_vector"
    scalar = twin_kf.method_estimate({"inputs": {
        "estimator": "ekf",
        "state": {"x": {"mean": 1.0, "variance": 0.01}},
        "measurements": [{"t": 0, "values": {"y": 1.1}}],
    }})
    assert scalar["estimator"] == "ekf"
    mismatch = twin_kf.method_estimate({"inputs": {
        "estimator": "particle",
        "state": {"x": [1.0]},
        "measurements": [{"t": 0, "values": [1.1]}],
    }})
    assert "error" in mismatch


# ---------------------------------------------------------------------
# 11. Full JSON-RPC round-trip over stdio (the contract surface).
# ---------------------------------------------------------------------

def _rpc(method: str, params: dict) -> dict:
    proc = subprocess.run(
        [sys.executable, str(PLUGIN)],
        input=json.dumps({"jsonrpc": "2.0", "id": "t1", "method": method, "params": params}) + "\n",
        capture_output=True, text=True, timeout=30,
    )
    assert proc.returncode == 0, proc.stderr
    lines = [ln for ln in proc.stdout.splitlines() if ln.strip()]
    resp = json.loads(lines[-1])
    assert "result" in resp, resp
    return resp["result"]


def test_stdio_jsonrpc_estimate_vector() -> None:
    # The executor envelope is {"step_id", "inputs"}; call_plugin sends
    # that shape, so test it exactly as the platform calls it. Expectations
    # are the per-station precision-weighted closed form (test 4).
    p0, r, prior = 0.01, 2.5e-5, 1.0
    result = _rpc("estimate", {"step_id": "t", "inputs": {
        "estimator": "ekf_vector",
        "state": {"x": [prior, prior], "covariance": [[p0, 0.0], [0.0, p0]]},
        "process_noise": {"covariance": [[0.0, 0.0], [0.0, 0.0]]},
        "measurement_noise": {"covariance": [[r, 0.0], [0.0, r]]},
        "measurements": [{"t": 0, "values": [1.15, 1.05]}],
    }})
    assert result["state_length"] == 2
    for i, y in enumerate([1.15, 1.05]):
        expected = (prior / p0 + y / r) / (1.0 / p0 + 1.0 / r)
        assert result["mean"][f"x{i}"] == pytest.approx(expected, rel=1e-10)


def test_stdio_jsonrpc_discover_advertises_vector_estimator() -> None:
    d = _rpc("discover", {"step_id": "t", "inputs": {}})
    assert "ekf_vector" in d["estimators"]
    assert "vector" in d["state_kinds"]
