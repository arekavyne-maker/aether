# SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
# SPDX-FileCopyrightText: 2026 Crocell Company
"""CalculiX binary presence check (alignment pass, I-7 / honesty discipline).

The `reference_plugins/calculix-adapter/` plugin claims to be a
real native-binary adapter that shells out to the `ccx_2.23`
binary. If the binary is not on PATH (or `$CCX_BIN` does not
resolve), the adapter is a fixture, not a production adapter, and
the project cannot claim Phase 2 production-track status for the
structural-solver side.

This check:

1. Calls the `calculix-adapter` plugin's `discover()` method.
2. Asserts the response's ``solver_binary_resolved`` is True.
3. Asserts the response's ``is_real_physics`` is True.
4. If the binary is on PATH: spawns the plugin's `execute` against
   the LE1 case and asserts the response's
   `verification.overall_status` is "pass".

A regression here means the project has lost the real CalculiX
integration; the production-track claim must be retracted.
"""

from __future__ import annotations

import json
import shutil
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

REPO_ROOT_DEFAULT = Path(__file__).resolve().parents[2]
PLUGIN = REPO_ROOT_DEFAULT / "reference_plugins" / "calculix-adapter" / "python" / "stdio_plugin.py"


@dataclass
class _Result:
    name: str
    ok: bool
    summary: str
    details: list[str]


def _spawn(method: str, params: dict | None, repo_root: Path) -> dict | None:
    request = {"jsonrpc": "2.0", "id": f"fitness-{method}", "method": method, "params": params}
    try:
        proc = subprocess.run(
            [sys.executable, str(repo_root / "reference_plugins" / "calculix-adapter" / "python" / "stdio_plugin.py")],
            input=json.dumps(request) + "\n",
            capture_output=True,
            text=True,
            timeout=30,
            check=False,
        )
    except (OSError, subprocess.TimeoutExpired) as exc:
        print(f"ccx_binary: spawn failed: {exc!r}", file=sys.stderr)
        return None
    for raw in (proc.stdout or "").splitlines():
        raw = raw.strip()
        if not raw:
            continue
        try:
            r = json.loads(raw)
        except json.JSONDecodeError:
            continue
        if r.get("id") == f"fitness-{method}":
            return r
    return None


def check(repo_root: Path) -> _Result:  # type: ignore[name-defined]
    from run import CheckResult  # local import; avoid circular at module import

    if not PLUGIN.is_file():
        return CheckResult(
            name="ccx_binary",
            ok=False,
            summary=f"calculix-adapter plugin not found at {PLUGIN}",
            details=[],
        )

    disc = _spawn("discover", None, repo_root)
    if disc is None:
        return CheckResult(
            name="ccx_binary",
            ok=False,
            summary="calculix-adapter discover() did not return a JSON-RPC response",
            details=[],
        )
    if "error" in disc:
        return CheckResult(
            name="ccx_binary",
            ok=False,
            summary=f"calculix-adapter discover() returned an error: {disc['error']!r}",
            details=[],
        )
    desc = disc.get("result", {})
    binary_path = desc.get("solver_binary", "")
    binary_resolved = bool(desc.get("solver_binary_resolved"))
    is_real = bool(desc.get("is_real_physics"))

    issues: list[str] = []
    if not is_real:
        issues.append(
            f"calculix-adapter discover() returned is_real_physics={is_real}; "
            f"a real native-binary adapter must report is_real_physics=True"
        )
    if not binary_resolved:
        issues.append(
            f"calculix-adapter cannot resolve ccx binary at {binary_path!r}; "
            f"PATH lookups: {shutil.which(binary_path)!r}; "
            f"install with `brew install calculix-ccx` or set $CCX_BIN"
        )

    if issues:
        return CheckResult(
            name="ccx_binary",
            ok=False,
            summary="calculix-adapter is not production-grade: ccx binary unresolved or honest label dropped",
            details=issues + [
                "fix: install calculix-ccx (Homebrew: `brew install calculix-ccx`) "
                "or set $CCX_BIN to a valid ccx_2.x executable"
            ],
        )

    # Binary is on PATH: run the LE1 case end-to-end and assert
    # the adapter's own verification passes. This is the live
    # check that the binary is actually wired and not merely
    # present.
    params = {
        "step_id": "fitness-ccx-le1",
        "problem_id": "01900000-0000-0000-0000-000000000010",
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
            "beam_length_m": 1.0,
            "cross_section_width_m": 0.01,
            "cross_section_height_m": 0.01,
            "E_pa": 200e9,
            "poisson_ratio": 0.3,
            "tip_load_n": 1000.0,
            "load_direction": "Z",
            "n_elements": 8,
        },
    }
    exec_resp = _spawn("execute", params, repo_root)
    if exec_resp is None or "error" in exec_resp:
        return CheckResult(
            name="ccx_binary",
            ok=False,
            summary="calculix-adapter execute() did not return a JSON-RPC response",
            details=[f"response: {exec_resp!r}"],
        )
    inner = exec_resp.get("result", {})
    if inner.get("verification", {}).get("overall_status") != "pass":
        return CheckResult(
            name="ccx_binary",
            ok=False,
            summary="calculix-adapter LE1 execute() did not pass verification",
            details=[
                f"verification: {inner.get('verification', {})!r}",
                "fix: check ccx binary version, deck generator, or frd parser",
            ],
        )

    diag = inner.get("diagnostics", {})
    rel_err = diag.get("rel_err", 1.0)
    return CheckResult(
        name="ccx_binary",
        ok=True,
        summary=(
            f"calculix-adapter is real: ccx={shutil.which(binary_path)}; "
            f"LE1 rel_err={rel_err:.3e} (tol 5e-2)"
        ),
        details=[
            f"solver_binary: {shutil.which(binary_path)}",
            f"v_numerical: {diag.get('v_numerical', '?')} m",
            f"v_analytical: {diag.get('v_analytical', '?')} m",
        ],
    )


if __name__ == "__main__":
    import argparse
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--repo-root", type=Path, default=REPO_ROOT_DEFAULT)
    args = parser.parse_args()
    r = check(args.repo_root)
    print(f"[{'OK ' if r.ok else 'FAIL'}] {r.name}: {r.summary}")
    for d in r.details:
        print(f"       {d}")
    sys.exit(0 if r.ok else 1)
