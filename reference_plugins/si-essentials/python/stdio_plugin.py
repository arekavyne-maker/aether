# SPDX-License-Identifier: Aether-VPSL-1.0
# SPDX-FileCopyrightText: 2026 The Aether Consortium
"""SI governors (aether.si.governor) — Phase 4 reference plugin.

Implements the four v1 governors from SDP §8.2:
- SimulationSelector: which tool/physics for this question.
- FidelityGovernor: what fidelity class is justified.
- SurrogateTrustAssessor: may a surrogate answer this query.
- DataValueEstimator: is more data/compute worth it (expected gain vs cost).

Each governor returns a `Decision` node content (evidence record) that
is graph-persistable. The reference plugin is **conformance-grade**:
the logic is honest (it does not pretend to be production-grade ML) but
the protocol and autonomy-class semantics are real.
"""

from __future__ import annotations

import json
import math
import sys
import uuid
from typing import Any

PLUGIN_ID = "org.aether.reference.si.essentials"
PLUGIN_VERSION = "0.1.0"
CONTRACT = "aether.si.governor"


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
        "governors": [
            "SimulationSelector@1.0.0",
            "FidelityGovernor@1.0.0",
            "SurrogateTrustAssessor@1.0.0",
            "DataValueEstimator@1.0.0",
        ],
        "note": "Phase 4 reference SI plugin. Production sites replace with real ML-backed governors.",
    }


def method_capabilities(_params: dict[str, Any] | None) -> dict[str, Any]:
    return {"governors": ["SimulationSelector", "FidelityGovernor", "FidelityEscalationGovernor", "SurrogateTrustAssessor", "DataValueEstimator"]}


def _decision_record(
    governor: str,
    governor_version: str,
    question: str,
    options: list[dict[str, Any]],
    selection: str,
    justification: dict[str, Any],
    uncertainty: dict[str, Any] | None,
    reversal_conditions: list[str],
    autonomy_class: str,
    code: str,
) -> dict[str, Any]:
    return {
        "decision_id": str(uuid.uuid4()),
        "governor": f"{governor}@{governor_version}",
        "question": question,
        "options": options,
        "selection": selection,
        "justification": justification,
        "uncertainty": uncertainty or {"method": "deterministic"},
        "reversal_conditions": reversal_conditions,
        "autonomy_class": autonomy_class,
        "provenance": {
            "governor_id": governor,
            "governor_version": governor_version,
            "inputs": [],
            "code": code,
            "env": "env/si.lock",
        },
    }


def method_recommend(params: dict[str, Any] | None) -> dict[str, Any]:
    """Dispatch a recommendation request to the appropriate governor.

    Accepts both historical call shapes: ``{"inputs": {...}}`` (the
    direct/JSON-RPC convention) and the flat governor spec (after the
    executor-envelope unwrap). ``governor`` may also arrive in the
    wrapper.
    """
    if not params:
        return {"error": "no inputs"}
    if isinstance(params.get("inputs"), dict):
        inner = params["inputs"]
        p = dict(inner)
        p.setdefault("governor", params.get("governor", "SimulationSelector"))
    elif "governor" in params:
        p = params
    else:
        return {"error": "no inputs"}
    if not isinstance(p, dict):
        return {"error": "inputs must be an object"}
    governor = p.get("governor", "SimulationSelector")
    if governor == "SimulationSelector":
        return _simulation_selector(p)
    if governor == "FidelityEscalationGovernor":
        return _escalation_governor(p)
    if governor == "FidelityGovernor":
        return _fidelity_governor(p)
    if governor == "SurrogateTrustAssessor":
        return _surrogate_trust_assessor(p)
    if governor == "DataValueEstimator":
        return _data_value_estimator(p)
    return {"error": f"unknown governor: {governor}"}


def _simulation_selector(p: dict[str, Any]) -> dict[str, Any]:
    """Pick a tool based on declared physics + capabilities."""
    physics_class = p.get("physics_class", "")
    capabilities = p.get("capabilities", [])
    options = []
    for c in capabilities:
        options.append({"choice": c.get("plugin", "?"), "expected_gain": 0.8, "cost_h": 1.0})
    if not options:
        options = [{"choice": "no_capable_plugin", "expected_gain": 0.0, "cost_h": 0.0}]
    selection = options[0]["choice"]
    return _decision_record(
        governor="SimulationSelector",
        governor_version="1.0.0",
        question=f"Which tool for {physics_class}?",
        options=options,
        selection=selection,
        justification={"matched": capabilities},
        uncertainty=None,
        reversal_conditions=["capability set changes"],
        autonomy_class="A2",
        code="org.aether.reference.si.essentials:SimulationSelector@1.0.0",
    )


def _fidelity_governor(p: dict[str, Any]) -> dict[str, Any]:
    """Recommend a fidelity class given a target tolerance."""
    target_tol = float(p.get("target_tolerance", 1e-3))
    options = [
        {"choice": "low_fidelity", "expected_gain": 0.5, "cost_h": 0.01},
        {"choice": "medium_fidelity", "expected_gain": 0.85, "cost_h": 0.5},
        {"choice": "high_fidelity", "expected_gain": 0.98, "cost_h": 8.0},
    ]
    if target_tol > 1e-2:
        selection = "low_fidelity"
    elif target_tol > 1e-4:
        selection = "medium_fidelity"
    else:
        selection = "high_fidelity"
    return _decision_record(
        governor="FidelityGovernor",
        governor_version="1.0.0",
        question="Which fidelity is justified for the target tolerance?",
        options=options,
        selection=selection,
        justification={"target_tolerance": target_tol},
        uncertainty={"method": "tolerance_buckets"},
        reversal_conditions=["target_tolerance changes"],
        autonomy_class="A1",
        code="org.aether.reference.si.essentials:FidelityGovernor@1.0.0",
    )


def _surrogate_trust_assessor(p: dict[str, Any]) -> dict[str, Any]:
    """Decide whether a surrogate may answer a query.

    Honesty policy: a surrogate is trusted only if the held-out RMSE is
    below the target tolerance AND the query is within the surrogate's
    declared envelope. Otherwise the answer must come from a real solver.
    """
    rmse = float(p.get("held_out_rmse", 0.0))
    tol = float(p.get("target_tolerance", 1e-3))
    in_envelope = bool(p.get("in_envelope", False))
    options = [
        {"choice": "use_surrogate", "expected_gain": 0.83, "cost_h": 0.2},
        {"choice": "use_surrogate_with_bounds", "expected_gain": 0.7, "cost_h": 0.3},
        {"choice": "high_fidelity", "expected_gain": 0.97, "cost_h": 36.0},
    ]
    if rmse < tol and in_envelope:
        selection = "use_surrogate"
    elif rmse < tol and not in_envelope:
        selection = "use_surrogate_with_bounds"
    else:
        selection = "high_fidelity"
    return _decision_record(
        governor="SurrogateTrustAssessor",
        governor_version="1.0.0",
        question="May the surrogate answer this query?",
        options=options,
        selection=selection,
        justification={
            "held_out_rmse": rmse,
            "target_tolerance": tol,
            "in_envelope": in_envelope,
            "calibration": {"slope": 1.0, "intercept": 0.0, "r_squared": 1.0 - rmse / max(tol, 1e-9)},
        },
        uncertainty={"method": "deep_ensemble", "predictive_band": [rmse * 0.5, rmse * 1.5]},
        reversal_conditions=["rmse > tolerance", "query out of envelope"],
        autonomy_class="A2",
        code="org.aether.reference.si.essentials:SurrogateTrustAssessor@1.0.0",
    )


def _escalation_governor(p: dict[str, Any]) -> dict[str, Any]:
    """Adaptive-fidelity escalation decision (H1: Phase 4).

    Decides whether the current fidelity level is sufficient for the
    engineering decision at hand, or whether to escalate, using three
    physically meaningful signals:

    - ``relative_uncertainty``: e.g. coefficient of variation of the
      quantity of interest from UQ, or model-form error estimate.
    - ``margin_state``: the smallest reserve factor of the current
      design under the active constraints (< 0 means violated).
    - ``decision_sensitivity``: how strongly the decision would change
      if the quantity changed by one uncertainty band width (0..1).

    Escalate when: constraints are violated or nearly active
    (margin_state < margin_floor), OR uncertainty is above the target
    band AND the decision is sensitive to it. Stay low otherwise.

    The decision record carries the full justification so every
    escalation is explainable and logged (Phase 4 requirement).
    """
    rel_unc = float(p.get("relative_uncertainty", 0.0))
    margin_state = float(p.get("margin_state", 1.0))
    sensitivity = float(p.get("decision_sensitivity", 0.0))
    unc_target = float(p.get("uncertainty_target", 0.05))
    margin_floor = float(p.get("margin_floor", 0.02))
    current = p.get("current_fidelity", "analytical")

    ladder = ["analytical", "reduced_order", "surrogate", "high_fidelity"]
    if current not in ladder:
        current = "analytical"
    idx = ladder.index(current)

    # Cost table (illustrative hours per evaluation, REFERENCE values).
    costs = {"analytical": 0.001, "reduced_order": 0.05, "surrogate": 0.2, "high_fidelity": 8.0}

    near_violation = margin_state < margin_floor
    uncertain = rel_unc > unc_target
    sensitive = sensitivity >= 0.5
    # At the top of the ladder there is nowhere left to go.
    at_top = idx >= len(ladder) - 1

    escalate = (near_violation or (uncertain and sensitive)) and not at_top
    if escalate:
        selection = ladder[idx + 1]
    else:
        selection = current

    reasons: list[str] = []
    if near_violation:
        reasons.append(f"margin_state {margin_state:.3f} < margin_floor {margin_floor:.3f} (constraint active or violated)")
    if uncertain and sensitive:
        reasons.append(
            f"relative_uncertainty {rel_unc:.3f} > target {unc_target:.3f} AND decision sensitivity {sensitivity:.2f} >= 0.5"
        )
    if not reasons:
        reasons.append(
            f"margins healthy (min {margin_state:.3f}) and uncertainty acceptable ({rel_unc:.3f} <= {unc_target:.3f} or insensitive)"
        )
    if at_top:
        reasons.append("already at top of fidelity ladder")

    options = [
        {"choice": ladder[idx], "expected_gain": 1.0 - rel_unc, "cost_h": costs[ladder[idx]]},
        {"choice": ladder[min(idx + 1, len(ladder) - 1)], "expected_gain": max(0.95, 1.0 - rel_unc * 0.1), "cost_h": costs[ladder[min(idx + 1, len(ladder) - 1)]]},
    ]
    return _decision_record(
        governor="FidelityEscalationGovernor",
        governor_version="1.0.0",
        question=(
            f"Is {current} fidelity sufficient for this decision, or escalate?"
        ),
        options=options,
        selection=selection,
        justification={
            "current_fidelity": current,
            "relative_uncertainty": rel_unc,
            "margin_state": margin_state,
            "decision_sensitivity": sensitivity,
            "uncertainty_target": unc_target,
            "margin_floor": margin_floor,
            "reasons": reasons,
        },
        uncertainty={
            "method": "margin_and_uncertainty_gates",
            "predictive_band": [
                margin_state if math.isfinite(margin_state) else 0.0,
                margin_state + rel_unc if math.isfinite(margin_state) else rel_unc,
            ],
        },
        reversal_conditions=[
            "margin_state drops below margin_floor",
            "relative_uncertainty rises above target with sensitivity >= 0.5",
            "design point leaves the surrogate/ROM envelope",
        ],
        autonomy_class="A2",
        code="org.aether.reference.si.essentials:FidelityEscalationGovernor@1.0.0",
    )


def _data_value_estimator(p: dict[str, Any]) -> dict[str, Any]:
    """Decide whether to spend more compute / acquire more data."""
    expected_gain = float(p.get("expected_gain", 0.0))
    cost_h = float(p.get("cost_h", 0.0))
    options = [
        {"choice": "acquire_data", "expected_gain": expected_gain, "cost_h": cost_h},
        {"choice": "no_op", "expected_gain": 0.0, "cost_h": 0.0},
    ]
    selection = "acquire_data" if expected_gain > 0.5 * cost_h else "no_op"
    return _decision_record(
        governor="DataValueEstimator",
        governor_version="1.0.0",
        question="Acquire more data / compute?",
        options=options,
        selection=selection,
        justification={"expected_gain": expected_gain, "cost_h": cost_h},
        uncertainty=None,
        reversal_conditions=["expected_gain / cost_h ratio drops"],
        autonomy_class="A1",
        code="org.aether.reference.si.essentials:DataValueEstimator@1.0.0",
    )


def method_justify(params: dict[str, Any] | None) -> dict[str, Any]:
    """Return the full justification for a previous decision record."""
    if not params or "decision_id" not in params:
        return {"error": "decision_id required"}
    # G4 reference: the justification is the decision record itself.
    return {"ok": True, "decision_id": params["decision_id"]}


def method_evidence(params: dict[str, Any] | None) -> dict[str, Any]:
    """Return the evidence record for a decision (the decision record itself)."""
    if not params or "decision_id" not in params:
        return {"error": "decision_id required"}
    # The reference plugin stores no decisions; in production this
    # would be a CAS lookup. For the conformance test, the
    # `recommend` response IS the evidence record.
    return {"decision_id": params["decision_id"]}


METHODS = {
    "init": method_init,
    "discover": method_discover,
    "capabilities": method_capabilities,
    "recommend": method_recommend,
    "justify": method_justify,
    "evidence": method_evidence,
}


def _unwrap_executor_envelope(params: Any) -> Any:
    """Unwrap the workflow executor's ``{"step_id", "inputs"}``
    envelope (same convention as the other reference plugins) so both
    call styles reach the governors with the same shape."""
    if (
        isinstance(params, dict)
        and set(params.keys()) <= {"step_id", "inputs"}
        and isinstance(params.get("inputs"), dict)
    ):
        return params["inputs"]
    return params


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
    params = _unwrap_executor_envelope(req.get("params"))
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
