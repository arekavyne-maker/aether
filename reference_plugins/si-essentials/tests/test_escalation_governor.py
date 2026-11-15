# SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
# SPDX-FileCopyrightText: 2026 Crocell Company
"""Conformance tests for the FidelityEscalationGovernor (Phase 4: H1).

The escalation governor is the decision logic behind adaptive fidelity:
it decides, from physically meaningful signals, whether the current
model level is sufficient or the analysis must escalate one rung on the
ladder (analytical -> reduced_order -> surrogate -> high_fidelity).

These tests pin the decision table:

- healthy margins + low uncertainty         -> stay
- constraint near violation                 -> escalate one rung
- high uncertainty + sensitive decision     -> escalate one rung
- high uncertainty + insensitive decision   -> stay (cost discipline)
- top of ladder                             -> never escalates past it
- the decision record always carries explicit reasons (explainability)

and the JSON-RPC surface (recommend dispatch, executor envelope).
"""

from __future__ import annotations

import importlib.util
import json
import subprocess
import sys
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parents[3]
PLUGIN = REPO_ROOT / "reference_plugins" / "si-essentials" / "python" / "stdio_plugin.py"

_spec = importlib.util.spec_from_file_location("si_essentials_stdio_plugin", PLUGIN)
assert _spec and _spec.loader
si = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(si)


def gov(**kw):
    base = dict(
        current_fidelity="analytical",
        relative_uncertainty=0.02,
        margin_state=0.4,
        decision_sensitivity=0.1,
    )
    base.update(kw)
    return si.method_recommend({"inputs": {"governor": "FidelityEscalationGovernor", **base}})


def rpc_rec(inputs: dict):
    req = {"jsonrpc": "2.0", "id": "t", "method": "recommend", "params": {"inputs": inputs}}
    proc = subprocess.run(
        [sys.executable, str(PLUGIN)],
        input=json.dumps(req) + "\n",
        capture_output=True,
        text=True,
        timeout=15,
        check=False,
    )
    assert proc.returncode == 0, proc.stderr
    for raw in proc.stdout.splitlines():
        try:
            r = json.loads(raw)
        except json.JSONDecodeError:
            continue
        if r.get("id") == "t":
            return r["result"]
    raise AssertionError("no response")


# ---------------------------------------------------------------------------
# Decision table
# ---------------------------------------------------------------------------

def test_stays_when_margins_healthy_and_uncertainty_low() -> None:
    d = gov()
    assert d["selection"] == "analytical"
    assert d["governor"] == "FidelityEscalationGovernor@1.0.0"


def test_escalates_one_rung_on_near_violation() -> None:
    d = gov(current_fidelity="reduced_order", margin_state=0.01)
    assert d["selection"] == "surrogate"
    assert any("margin_floor" in r for r in d["justification"]["reasons"])


def test_escalates_on_uncertainty_with_sensitivity() -> None:
    d = gov(relative_uncertainty=0.20, decision_sensitivity=0.8)
    assert d["selection"] == "reduced_order"
    assert any("sensitivity" in r for r in d["justification"]["reasons"])


def test_does_not_escalate_when_insensitive() -> None:
    """Cost discipline: uncertainty that cannot change the decision does
    not justify compute."""
    d = gov(current_fidelity="surrogate", relative_uncertainty=0.30, decision_sensitivity=0.2, margin_state=0.5)
    assert d["selection"] == "surrogate"


def test_violated_margin_escalates() -> None:
    d = gov(margin_state=-0.05)
    assert d["selection"] == "reduced_order"


def test_never_escalates_past_ladder_top() -> None:
    d = gov(current_fidelity="high_fidelity", margin_state=-0.5, relative_uncertainty=0.5, decision_sensitivity=1.0)
    assert d["selection"] == "high_fidelity"
    assert any("top of fidelity ladder" in r for r in d["justification"]["reasons"])


def test_escalation_is_one_rung_at_a_time() -> None:
    ladder = ["analytical", "reduced_order", "surrogate", "high_fidelity"]
    d = gov(current_fidelity="analytical", margin_state=-1.0, relative_uncertainty=0.9, decision_sensitivity=1.0)
    assert ladder.index(d["selection"]) == ladder.index("analytical") + 1


def test_decision_record_is_complete() -> None:
    d = gov(margin_state=0.01)
    for field in ("question", "options", "selection", "justification", "autonomy_class", "provenance", "reversal_conditions"):
        assert field in d, field
    assert d["autonomy_class"] in ("A0", "A1", "A2", "A3")  # A4 forbidden by ADR-0024
    assert d["justification"]["current_fidelity"] == "analytical"
    assert d["justification"]["margin_state"] == pytest.approx(0.01)
    assert len(d["reversal_conditions"]) >= 2


def test_deterministic() -> None:
    a = gov(relative_uncertainty=0.1, margin_state=0.01, decision_sensitivity=0.7)
    b = gov(relative_uncertainty=0.1, margin_state=0.01, decision_sensitivity=0.7)
    a["decision_id"] = b["decision_id"] = ""
    assert json.dumps(a, sort_keys=True) == json.dumps(b, sort_keys=True)


# ---------------------------------------------------------------------------
# Contract surface
# ---------------------------------------------------------------------------

def test_rpc_dispatch_and_envelope() -> None:
    d = rpc_rec({"governor": "FidelityEscalationGovernor", "current_fidelity": "reduced_order", "margin_state": 0.0})
    assert d["selection"] == "surrogate"
    # Executor envelope form must behave identically.
    d2 = rpc_rec({"step_id": "s", "inputs": {"governor": "FidelityEscalationGovernor", "current_fidelity": "reduced_order", "margin_state": 0.0}})
    assert d2["selection"] == "surrogate"


def test_capabilities_list_governor() -> None:
    caps = si.method_capabilities(None)
    assert "FidelityEscalationGovernor" in caps["governors"]
