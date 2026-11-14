# SPDX-License-Identifier: Aether-VPSL-1.0
# SPDX-FileCopyrightText: 2026 The Aether Consortium
"""Cross-plugin consistency test: fea-reference-beam vs calculix-adapter.

Both plugins implement the ``aether.physics.solver`` contract and
both can solve the LE1 cantilever case. The closed-form analytical
tip deflection is identical for both (it is the canonical Euler-
Bernoulli beam formula). This test:

  1. Calls each plugin's ``method_execute`` with the *same* problem
     spec.
  2. Asserts each plugin's numerical tip deflection is within the
     declared relative tolerance of the analytical value.
  3. Asserts the two plugins' numerical tip deflections agree
     with each other to within 5% (the wider of the two tolerances
     is a generous cross-plugin check: any unit-conversion error
     or sign error in either plugin will produce a wildly different
     number here).

This is the explicit "two-impl, mutually checked" claim that the
SDP §5.8 two-implementation rule is satisfied by two REAL,
MUTUALLY-CHECKED implementations of the same physics — not by a
real implementation and a hash-stub shim.

If the ccx binary is not on PATH, the calculix test skips and the
beam test runs alone.
"""

from __future__ import annotations

import importlib.util
import math
import sys
from pathlib import Path

import pytest


REPO_ROOT = Path(__file__).resolve().parents[3]
BEAM_PLUGIN = REPO_ROOT / "reference_plugins" / "fea-reference-beam" / "python" / "stdio_plugin.py"
CCX_PLUGIN = REPO_ROOT / "reference_plugins" / "calculix-adapter" / "python" / "stdio_plugin.py"


def _import(path: Path, name: str):
    spec = importlib.util.spec_from_file_location(name, path)
    assert spec and spec.loader
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


beam = _import(BEAM_PLUGIN, "fea_ref_beam_stdio_plugin")
ccx = _import(CCX_PLUGIN, "calculix_adapter_stdio_plugin")

import shutil

CCX_AVAILABLE = shutil.which(ccx.CCX_BIN) is not None
SKIP_REASON = f"ccx binary not on PATH (CCX_BIN={ccx.CCX_BIN!r})"


# Beam plugin parameters (Euler-Bernoulli).
# The beam plugin uses ``beam_length_m``, ``E_pa``, ``I_m4``; it does
# not have a poisson_ratio or a cross-section directly, only an
# I value. We compute the equivalent I for the same cross section.
W = 0.01  # m
H = 0.01  # m
I = W * H ** 3 / 12.0
E = 200e9
L = 1.0
P = 1000.0
v_analytical = P * L ** 3 / (3.0 * E * I)
# Sanity: 2.0 m.
assert math.isclose(v_analytical, 2.0, rel_tol=1e-12)


def _beam_params() -> dict:
    return {
        "physics_class": "structural",
        "beam_length_m": L,
        "E_pa": E,
        "I_m4": I,
        "case": "cantilever_point",
        "P_n": P,
        "n_elements": 8,
        "problem_id": "cross-impl-1",
    }


def _ccx_params() -> dict:
    return {
        "step_id": "cross-impl-1",
        "problem_id": "cross-impl-1",
        "physics_class": "structural",
        "geometry": {"cas_hash": "0" * 64, "format": "native"},
        "boundary_conditions": [
            {"type": "dirichlet", "region": "root", "values": {"all_dof": 0.0}},
        ],
        "solver_settings": {
            "case": "le1_cantilever_b31",
            "n_elements": 8,
        },
        "inputs": {
            "beam_length_m": L,
            "cross_section_width_m": W,
            "cross_section_height_m": H,
            "E_pa": E,
            "poisson_ratio": 0.3,
            "tip_load_n": P,
            "load_direction": "Z",
            "n_elements": 8,
        },
    }


def test_beam_plugin_alone_matches_analytical() -> None:
    """Sanity: the beam plugin's own analytical check is what the
    cross-plugin consistency test is built on top of. If this fails,
    the cross-plugin test cannot run. The beam plugin's tolerance is
    1e-6 (8 elements) — this is a tight, per-plugin check.
    """
    r = beam.method_execute(_beam_params())
    v = r["diagnostics"]["v_numerical"]
    rel_err = r["diagnostics"]["rel_err"]
    assert math.isclose(r["diagnostics"]["v_analytical"], v_analytical, rel_tol=1e-12)
    assert rel_err < beam.DEFAULT_REL_TOL


@pytest.mark.skipif(not CCX_AVAILABLE, reason=SKIP_REASON)
def test_ccx_plugin_alone_matches_analytical() -> None:
    """Sanity: the ccx plugin's own check is what the cross-plugin
    consistency test is built on top of.
    """
    r = ccx.method_execute(_ccx_params())
    assert r["verification"]["overall_status"] == "pass"
    v = r["diagnostics"]["v_numerical"]
    rel_err = r["diagnostics"]["rel_err"]
    assert math.isclose(r["diagnostics"]["v_analytical"], v_analytical, rel_tol=1e-12)
    assert rel_err < ccx.DEFAULT_REL_TOL


@pytest.mark.skipif(not CCX_AVAILABLE, reason=SKIP_REASON)
def test_two_real_implementations_agree_within_5_percent() -> None:
    """The two-implementation rule for ``aether.physics.solver`` is
    satisfied here by two REAL, MUTUALLY-CHECKED implementations
    of the same physics on the same problem spec.

    The beam plugin uses Euler-Bernoulli beam theory (closed-form
    solution at the canonical load point). The calculix plugin runs
    a real Timoshenko beam FEM (8 B31 elements) through the ccx
    binary and parses the .frd output. Both report a numerical
    tip deflection; the cross-plugin check is that they agree to
    within 5%.
    """
    rb = beam.method_execute(_beam_params())
    rc = ccx.method_execute(_ccx_params())

    v_beam = rb["diagnostics"]["v_numerical"]
    v_ccx = rc["diagnostics"]["v_numerical"]
    rel_err_beam = rb["diagnostics"]["rel_err"]
    rel_err_ccx = rc["diagnostics"]["rel_err"]

    # Each plugin's own check must pass.
    assert rb["verification"]["overall_status"] == "pass", (
        f"beam plugin failed its own check: rel_err={rel_err_beam}"
    )
    assert rc["verification"]["overall_status"] == "pass", (
        f"calculix plugin failed its own check: rel_err={rel_err_ccx}"
    )

    # Cross-plugin agreement: 5% is a generous engineering
    # tolerance (the 8-element B31 mesh gives ~0.5% vs analytical;
    # the beam plugin's own FEM error is < 1e-6). A wider
    # tolerance accommodates any future plugin that uses a
    # coarser mesh or a different formulation, while still
    # catching unit errors and sign errors.
    if v_ccx == 0.0:
        cross_rel = 0.0 if v_beam == 0.0 else float("inf")
    else:
        cross_rel = abs(v_beam - v_ccx) / abs(v_ccx)
    assert cross_rel < 5e-2, (
        f"cross-plugin disagreement: beam={v_beam:.6e} m, "
        f"ccx={v_ccx:.6e} m, rel_err={cross_rel:.3e} (tol 5e-2)"
    )
