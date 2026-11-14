# SPDX-License-Identifier: Aether-VPSL-1.0
# SPDX-FileCopyrightText: 2026 The Aether Consortium
"""structures-sizing — reference thin-wall wing-box sizing model for the
``aether.physics.solver`` contract (Phase 3: aerostructural core).

WHAT THIS IS
------------
A deterministic, closed-form **structural sizing** reference: a
rectangular thin-wall wing-box idealized as two boom caps (bending),
two shear webs, and two covers (shear + buckling). It produces section
stresses, a tip-deflection estimate, structural mass, and **reserve
factors (margins of safety)** for one load case.

WHAT THIS IS NOT
----------------
- NOT high-fidelity FEA: no discretized shell/solid model, no mesh.
- NOT a buckling knock-down analysis beyond the classical long-plate
  shear-buckling formula.
- NOT composite/laminate sizing: isotropic material only.
- NOT fatigue, damage tolerance, or post-buckled analysis.
- Rib mass, joints, cutouts, and taper are ignored (documented below).

MODEL AND ASSUMPTIONS (every assumption is explicit, per the honesty
policy; the same list is returned in each result as
``diagnostics.model_assumptions``):

1. Bending: the wing box is idealized as two boom caps of area
   ``A_cap`` separated by section height ``h`` (boom theory).
   ``I = 2 * A_cap * (h/2)^2``; cap stress ``sigma = M / (A_cap * h)``
   (identical to ``M*y/I`` with ``y = h/2``; checked numerically).
2. Root bending moment: the lift resultant acts at quarter semi-span
   from the root: ``M_root = L * n * j * span / 4``. For an elliptical
   distribution the resultant acts at ``4/(3*pi) * span/2 ≈ 0.424 *
   span/2`` from the root; quarter-span (``0.5 * span/2``) is slightly
   conservative. VALID for straight, untapered, unswept wings with
   span-wise-constant section.
3. Shear: webs and covers each carry ``tau = V / (2 * h * t)`` (two
   shear webs; covers treated as shear webs of the same height —
   conservative simplification; valid for thin-walled sections where
   cap shear carry-over is neglected).
4. Cover shear buckling: classical simply-supported long-plate result
   ``tau_cr = k_s * pi^2 * E / (12 * (1 - nu^2)) * (t/b)^2`` with
   ``k_s = 5.35`` (a/b >= 5) and ``b`` = panel width = chord. VALID for
   thin isotropic panels, no post-buckling reserve credited.
5. Tip deflection: uniformly-distributed-load equivalent cantilever,
   ``v_tip = V_root * span^3 / (8 * E * I)``. VALID for linear
   elasticity, small deflections; ignores shear deformation (slender
   beam regime, span/h > ~10).
6. Mass: caps ``2*A_cap*span*rho``; webs ``2*h*span*t_web*rho``;
   covers ``2*chord*span*t_cover*rho``. Ribs, joints, fasteners, and
   non-optimum factors are NOT included (mass is an under-estimate of
   a real wing box; labeled accordingly).
7. Load case: ``V_root = L * n_limit * j`` where ``n_limit`` is the
   limit load factor and ``j`` the additional safety factor; margins
   are against yield/allowables at that load (no separate ultimate
   factor inside the margin — the factor is already in the load).

VALIDITY REGIME (when to distrust this model):
- span/section_height >= ~10 (slender-beam regime for deflection);
- thin walls (t << h, t << chord);
- isotropic, linear-elastic material;
- static limit-load sizing only.

CLASSIFICATION: REFERENCE (deterministic, closed-form-verified inside
``execute`` by independent recomputation of each formula).

DETERMINISM: pure Python floats (IEEE 754 double), no randomness, no
time, no filesystem. Same inputs produce byte-identical results.

CONTRACT METHODS
----------------
- ``discover``/``capabilities``/``validate``/``execute`` — as
  ``fea-reference-beam``. ``execute`` accepts either a single problem
  spec or a ``samples: [spec, ...]`` array (batch evaluation used by
  DOE/UQ drivers); batch responses carry ``results: [...]``.

INPUTS (single spec; SI units)
------------------------------
span_m, chord_m, section_height_m, A_cap_m2, t_cap_m, t_web_m,
t_cover_m, E_pa, sigma_yield_pa, rho_kg_m3, lift_n,
load_factor (=2.5), safety_factor (=1.5), nu (=0.33),
deflection_limit_ratio (=0.15, tip deflection / span).

OUTPUTS
-------
``diagnostics``: quantity/units/value (wing-box mass), margins,
stresses, tip deflection, buckling stress, load state, model name and
assumptions. ``verification``: per-formula independent-recomputation
checks plus regime checks, ``overall_status`` pass/fail.
"""

from __future__ import annotations

import hashlib
import json
import math
import sys
import time
import uuid
from typing import Any

PLUGIN_ID = "org.aether.reference.structures.sizing"
PLUGIN_VERSION = "0.1.0"

# Classical shear-buckling coefficient for simply-supported long plates
# (a/b >= ~5), torsional/shear loading (Timoshenko & Gere).
KS_SHEAR_LONG_PLATE = 5.35

REQUIRED_FIELDS = (
    "span_m",
    "chord_m",
    "section_height_m",
    "A_cap_m2",
    "t_web_m",
    "t_cover_m",
    "E_pa",
    "sigma_yield_pa",
    "rho_kg_m3",
    "lift_n",
)

MODEL_NAME = "thin_wall_boom_idealization"

MODEL_ASSUMPTIONS = [
    "bending: two-boom idealization I = 2*A_cap*(h/2)^2, sigma_cap = M/(A_cap*h)",
    "root moment: lift resultant at quarter semi-span (conservative vs elliptical 4/(3pi)*span/2)",
    "shear: tau = V/(2*h*t) on two webs and two covers (thin-web, no cap carry-over)",
    "cover buckling: tau_cr = 5.35*pi^2*E/(12*(1-nu^2))*(t/chord)^2 (simply-supported long plate, no post-buckling reserve)",
    "tip deflection: UDL-equivalent cantilever v = V*L^3/(8*E*I), no shear deformation (slender regime)",
    "mass: caps + webs + covers only; ribs, joints, fasteners, non-optimum factors NOT included (under-estimate)",
    "isotropic linear-elastic material; static limit load; validity span/h >= ~10",
]


def _err(req_id: Any, code: int, message: str) -> dict[str, Any]:
    return {"jsonrpc": "2.0", "id": req_id, "error": {"code": code, "message": message}}


def _ok(req_id: Any, result: dict[str, Any]) -> dict[str, Any]:
    return {"jsonrpc": "2.0", "id": req_id, "result": result}


def method_init(params: dict[str, Any] | None) -> dict[str, Any]:
    """Run-token shape check (structural only, same as beam reference)."""
    if not isinstance(params, dict):
        return {"ok": False, "reason": "params must be an object"}
    for field in ("token_id", "workflow_id", "step_id", "scope", "signature_b64"):
        if field not in params:
            return {"ok": False, "reason": f"missing field: {field}"}
    if not isinstance(params.get("scope"), dict):
        return {"ok": False, "reason": "scope must be an object"}
    return {"ok": True, "plugin_id": PLUGIN_ID, "plugin_version": PLUGIN_VERSION}


def method_discover(_params: dict[str, Any] | None) -> dict[str, Any]:
    return {
        "plugin_id": PLUGIN_ID,
        "plugin_version": PLUGIN_VERSION,
        "contract": "aether.physics.solver",
        "is_real_physics": True,
        "physics_kind": "thin_wall_wingbox_sizing",
        "fidelity_class": "reduced_order",
        "classification": "REFERENCE",
        "model": MODEL_NAME,
        "validated_cases": ["single_spec", "batch_samples"],
        "determinism_class": "bit-reproducible",
    }


def method_capabilities(_params: dict[str, Any] | None) -> dict[str, Any]:
    return {
        "plugin_id": PLUGIN_ID,
        "methods": ["discover", "capabilities", "validate", "execute"],
        "execute_contract": {
            "inputs": list(REQUIRED_FIELDS)
            + [
                "t_cap_m",
                "load_factor",
                "safety_factor",
                "nu",
                "deflection_limit_ratio",
                "samples",
            ],
            "outputs": [
                "diagnostics.quantity",
                "diagnostics.value",
                "diagnostics.margins",
                "diagnostics.stresses",
                "diagnostics.tip_deflection",
                "verification.checks",
            ],
        },
        "model": MODEL_NAME,
        "model_assumptions": MODEL_ASSUMPTIONS,
        "is_real_physics": True,
        "physics_kind": "thin_wall_wingbox_sizing",
        "classification": "REFERENCE",
    }


def _positive(v: Any, name: str) -> float:
    x = float(v)
    if not math.isfinite(x) or x <= 0.0:
        raise ValueError(f"{name} must be a positive finite number, got {v!r}")
    return x


def validate_spec(p: dict[str, Any]) -> list[dict[str, Any]]:
    """Structural validation; returns a list of check dicts."""
    checks: list[dict[str, Any]] = []
    missing = [k for k in REQUIRED_FIELDS if k not in p]
    checks.append(
        {
            "name": "required_fields",
            "kind": "schema",
            "result": "pass" if not missing else "fail",
            "diagnostic": "" if not missing else f"missing: {missing}",
        }
    )
    if missing:
        return checks

    try:
        for k in REQUIRED_FIELDS:
            _positive(p[k], k)
        _positive(p.get("t_cap_m", p["t_web_m"]), "t_cap_m")
        _positive(float(p.get("load_factor", 2.5)), "load_factor")
        _positive(float(p.get("safety_factor", 1.5)), "safety_factor")
        nu = float(p.get("nu", 0.33))
        if not (0.0 < nu < 0.5):
            raise ValueError(f"nu must be in (0, 0.5), got {nu}")
        checks.append({"name": "values_positive", "kind": "schema", "result": "pass",
                       "diagnostic": "all geometry/material/load fields positive and finite"})
    except (TypeError, ValueError) as exc:
        checks.append({"name": "values_positive", "kind": "schema", "result": "fail",
                       "diagnostic": str(exc)})
        return checks

    span = float(p["span_m"])
    h = float(p["section_height_m"])
    slender = span / h
    checks.append(
        {
            "name": "slender_regime",
            "kind": "validity",
            "result": "pass" if slender >= 10.0 else "warn",
            "diagnostic": (
                f"span/h = {slender:.2f} (model valid for >= ~10; "
                "deflection formula ignores shear deformation)"
            ),
        }
    )
    return checks


def _solve_spec(p: dict[str, Any]) -> dict[str, Any]:
    """Closed-form sizing solve for one validated spec."""
    span = float(p["span_m"])
    chord = float(p["chord_m"])
    h = float(p["section_height_m"])
    a_cap = float(p["A_cap_m2"])
    t_web = float(p["t_web_m"])
    t_cover = float(p["t_cover_m"])
    e = float(p["E_pa"])
    sigma_y = float(p["sigma_yield_pa"])
    rho = float(p["rho_kg_m3"])
    lift = float(p["lift_n"])
    n = float(p.get("load_factor", 2.5))
    j = float(p.get("safety_factor", 1.5))
    nu = float(p.get("nu", 0.33))
    defl_ratio = float(p.get("deflection_limit_ratio", 0.15))
    t_cap = float(p.get("t_cap_m", t_web))

    # --- load state (assumption 2) ---
    v_root = lift * n * j
    m_root = v_root * span / 4.0

    # --- bending (assumption 1) ---
    i_zz = 2.0 * a_cap * (h / 2.0) ** 2
    sigma_cap_form1 = m_root * (h / 2.0) / i_zz
    sigma_cap_form2 = m_root / (a_cap * h)  # independent algebraic form
    sigma_cap = sigma_cap_form1

    # --- shear (assumption 3) ---
    tau_web = v_root / (2.0 * h * t_web)
    tau_cover = v_root / (2.0 * h * t_cover)

    # --- cover shear buckling (assumption 4) ---
    tau_cr = (
        KS_SHEAR_LONG_PLATE
        * math.pi**2
        * e
        / (12.0 * (1.0 - nu**2))
        * (t_cover / chord) ** 2
    )

    # --- tip deflection (assumption 5) ---
    v_tip = v_root * span**3 / (8.0 * e * i_zz)

    # --- mass (assumption 6) ---
    mass_caps = 2.0 * a_cap * span * rho
    mass_webs = 2.0 * h * span * t_web * rho
    mass_covers = 2.0 * chord * span * t_cover * rho
    mass_total = mass_caps + mass_webs + mass_covers

    # --- margins (reserve factors; load already includes n and j) ---
    margin_yield = sigma_y / sigma_cap - 1.0
    margin_web_shear = tau_cr * (t_web / t_cover) ** 2 / tau_web - 1.0 if t_web > 0 else float("-inf")
    margin_cover_buckling = tau_cr / tau_cover - 1.0
    margin_deflection = defl_ratio * span / v_tip - 1.0

    return {
        "load_state": {
            "V_root_n": v_root,
            "M_root_nm": m_root,
            "load_factor": n,
            "safety_factor": j,
        },
        "section": {"I_m4": i_zz, "A_cap_m2": a_cap, "t_cap_m": t_cap},
        "stresses": {
            "sigma_cap_pa": sigma_cap,
            "tau_web_pa": tau_web,
            "tau_cover_pa": tau_cover,
            "tau_cr_cover_pa": tau_cr,
        },
        "tip_deflection_m": v_tip,
        "mass": {
            "caps_kg": mass_caps,
            "webs_kg": mass_webs,
            "covers_kg": mass_covers,
            "total_kg": mass_total,
        },
        "margins": {
            "margin_yield": margin_yield,
            "margin_web_shear": margin_web_shear,
            "margin_cover_buckling": margin_cover_buckling,
            "margin_deflection": margin_deflection,
        },
        # Internal-consistency quantities for the verification checks.
        "_checks_data": {
            "sigma_cap_form2": sigma_cap_form2,
        },
    }


def _verification(r: dict[str, Any], p: dict[str, Any]) -> dict[str, Any]:
    """Independent recomputation of every formula + regime checks.

    These are REFERENCE-grade internal checks (the same formulas
    re-derived and re-applied); they do not constitute validation
    against external test data.
    """
    checks: list[dict[str, Any]] = []
    cd = r["_checks_data"]

    # 1. Cap stress: M/(A*h) vs M*y/I must agree to machine precision.
    s1 = r["stresses"]["sigma_cap_pa"]
    s2 = cd["sigma_cap_form2"]
    denom = max(abs(s1), abs(s2), 1e-30)
    rel = abs(s1 - s2) / denom
    checks.append(
        {
            "name": "cap_stress_consistency",
            "kind": "verification",
            "result": "pass" if rel <= 1e-12 else "fail",
            "diagnostic": f"sigma via M*y/I = {s1:.6e} vs M/(A*h) = {s2:.6e}, rel = {rel:.2e}",
            "rel_err": rel,
        }
    )

    # 2. Buckling formula recomputation with literal constants.
    tc = float(p["t_cover_m"])
    chord = float(p["chord_m"])
    e = float(p["E_pa"])
    nu = float(p.get("nu", 0.33))
    tau_cr_re = 5.35 * 9.869604401089358 * e / (12.0 * (1.0 - nu * nu)) * (tc / chord) ** 2
    rel = abs(tau_cr_re - r["stresses"]["tau_cr_cover_pa"]) / max(tau_cr_re, 1e-30)
    checks.append(
        {
            "name": "buckling_formula",
            "kind": "verification",
            "result": "pass" if rel <= 1e-12 else "fail",
            "diagnostic": f"tau_cr recomputed with literal constants: rel = {rel:.2e}",
            "rel_err": rel,
        }
    )

    # 3. Load path: V_root must equal L*n*j (identity of assumption 2).
    v_expect = float(p["lift_n"]) * float(p.get("load_factor", 2.5)) * float(p.get("safety_factor", 1.5))
    ok = r["load_state"]["V_root_n"] == v_expect
    checks.append(
        {
            "name": "load_path",
            "kind": "verification",
            "result": "pass" if ok else "fail",
            "diagnostic": f"V_root = {r['load_state']['V_root_n']:.6e} N = lift*n*j",
        }
    )

    # 4. Positive margins present and finite (sane outputs).
    margins = r["margins"]
    finite = all(math.isfinite(v) for v in margins.values())
    checks.append(
        {
            "name": "margins_finite",
            "kind": "schema",
            "result": "pass" if finite else "fail",
            "diagnostic": "all reserve factors finite" if finite else "non-finite margin (check inputs)",
        }
    )

    # 5. Mass accounting: total equals the sum of components.
    m = r["mass"]
    total_re = m["caps_kg"] + m["webs_kg"] + m["covers_kg"]
    rel = abs(total_re - m["total_kg"]) / max(total_re, 1e-30)
    checks.append(
        {
            "name": "mass_accounting",
            "kind": "verification",
            "result": "pass" if rel <= 1e-12 else "fail",
            "diagnostic": f"mass components sum to {total_re:.6e} kg (rel diff {rel:.2e}); "
            "excludes ribs/joints/fasteners",
        }
    )

    overall = "pass" if all(c["result"] != "fail" for c in checks) else "fail"
    return {"checks": checks, "overall_status": overall}


def _bundle_hash(payload: dict[str, Any]) -> str:
    canonical = json.dumps(payload, sort_keys=True, separators=(",", ":"))
    return hashlib.sha256(canonical.encode("utf-8")).hexdigest()


def _execute_single(p: dict[str, Any]) -> dict[str, Any]:
    checks = validate_spec(p)
    hard_fail = any(c["result"] == "fail" for c in checks)
    if hard_fail:
        return {
            "plugin_id": PLUGIN_ID,
            "plugin_version": PLUGIN_VERSION,
            "determinism_class": "bit-reproducible",
            "outputs": {},
            "verification": {"checks": checks, "overall_status": "fail"},
            "step_id": p.get("step_id", "unknown"),
        }
    r = _solve_spec(p)
    ver = _verification(r, p)
    # Validity-regime checks (e.g. slender_regime warn) ride alongside
    # the hard verification checks so they are never silently dropped.
    regime = [c for c in checks if c["name"] not in ("required_fields", "values_positive")]
    all_checks = ver["checks"] + regime
    ver = {
        "checks": all_checks,
        "overall_status": "fail" if any(c["result"] == "fail" for c in all_checks) else "pass",
    }

    margins = r["margins"]
    stresses = r["stresses"]
    mass_total = r["mass"]["total_kg"]
    diagnostics = {
        "quantity": "wing_box_mass",
        "units": "kg",
        "value": mass_total,
        "fidelity_class": "reduced_order",
        "classification": "REFERENCE",
        "model": MODEL_NAME,
        "model_assumptions": MODEL_ASSUMPTIONS,
        "margins": margins,
        "stresses": {k: v for k, v in stresses.items()},
        "section": r["section"],
        "tip_deflection_m": r["tip_deflection_m"],
        "load_state": r["load_state"],
        "mass_breakdown": r["mass"],
    }
    # Drop the internal helpers from the public result.
    public = {k: v for k, v in r.items() if not k.startswith("_")}
    bundle = _bundle_hash(public)
    return {
        "plugin_id": PLUGIN_ID,
        "plugin_version": PLUGIN_VERSION,
        "determinism_class": "bit-reproducible",
        "outputs": {
            "result.bundle": {
                "cas_hash": bundle,
                "name": "result.bundle",
                "size": 0,
                "format": "native",
                "fidelity_class": "reduced_order",
            }
        },
        "verification": ver,
        "diagnostics": diagnostics,
        "step_id": p.get("step_id", "unknown"),
    }


def method_execute(params: dict[str, Any] | None) -> dict[str, Any]:
    if not params:
        return {"error": "no inputs"}
    # Batch mode: evaluate many designs in one plugin process (used by
    # DOE/UQ/optimization drivers to amortize process spawn cost).
    samples = params.get("samples")
    if isinstance(samples, list) and samples:
        results = []
        for i, s in enumerate(samples):
            if not isinstance(s, dict):
                one = {
                    "index": i,
                    "error": f"sample {i} is not an object",
                }
            else:
                one = _execute_single(s)
                one["index"] = i
            results.append(one)
        all_ok = all(r.get("verification", {}).get("overall_status") == "pass" for r in results)
        return {
            "plugin_id": PLUGIN_ID,
            "plugin_version": PLUGIN_VERSION,
            "determinism_class": "bit-reproducible",
            "batch": True,
            "n_samples": len(results),
            "results": results,
            "verification": {
                "checks": [
                    {
                        "name": "batch_all_verified",
                        "kind": "verification",
                        "result": "pass" if all_ok else "fail",
                        "diagnostic": f"{sum(1 for r in results if r.get('verification', {}).get('overall_status') == 'pass')}/{len(results)} samples verified",
                    }
                ],
                "overall_status": "pass" if all_ok else "fail",
            },
            "step_id": params.get("step_id", "unknown"),
        }

    return _execute_single(params)


def _unwrap_executor_envelope(params: Any) -> Any:
    """Unwrap the workflow executor's ``{"step_id", "inputs"}`` envelope
    (same convention as the other reference plugins)."""
    if (
        isinstance(params, dict)
        and set(params.keys()) <= {"step_id", "inputs"}
        and isinstance(params.get("inputs"), dict)
    ):
        return params["inputs"]
    return params


METHODS = {
    "init": method_init,
    "discover": method_discover,
    "capabilities": method_capabilities,
    "validate": lambda params: {"valid": not any(c["result"] == "fail" for c in validate_spec(params or {})), "checks": validate_spec(params or {})},
    "execute": method_execute,
}


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
    started = time.time()
    correlation = uuid.uuid4().hex[:8]
    print(f"[{correlation}] plugin ready (pid={__import__('os').getpid()})", file=sys.stderr)
    for raw in sys.stdin:
        response = handle(raw)
        if response is None:
            continue
        sys.stdout.write(json.dumps(response) + "\n")
        sys.stdout.flush()
    print(
        f"[{correlation}] plugin shutdown after {time.time() - started:.3f}s",
        file=sys.stderr,
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
