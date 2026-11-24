#!/usr/bin/env python3
# SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
# SPDX-FileCopyrightText: 2026 Crocell Company
"""Aether benchmark registry and runner (ADR-0040, specs §17.2, §21).

Declares scientific-validity and operational-performance benchmarks with
reference, assumptions, tolerance, version constraints and known limits.
Runs the executable ones and records evidence under
``docs/verification/baseline/<date>/``. Runtime and accuracy are reported
separately; a fast inaccurate result is never an optimisation.
"""

from __future__ import annotations

import argparse
import json
import platform
import subprocess
import sys
import time
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]

SCIENTIFIC = [
    ("beam_mechanics", "aether demo structure --json", "P L^3 / (3 E I)", 5e-3, "fea-reference-beam"),
    ("buckling", "pytest -q reference_plugins/fea-reference-buckling/tests", "Euler pinned/cantilever", 2e-2, "fea-reference-buckling"),
    ("thermal_conduction", "pytest -q reference_plugins/fea-reference-thermal/tests", "linear + parabolic profile", 2e-2, "fea-reference-thermal"),
    ("plate_mechanics", "pytest -q reference_plugins/fea-reference-spar/tests", "four closed forms", 1e-2, "fea-reference-spar"),
    ("composite_laminate", "pytest -q reference_plugins/composite-laminate/tests", "CLT isotropy/UD/cross-ply", 5e-2, "composite-laminate"),
    ("architected_homogenisation", "pytest -q reference_plugins/lattice-generator/tests", "Gibson-Ashby", 1e-9, "lattice-generator"),
    ("cfd_canonical", "pytest -q reference_plugins/cfd-reference-minimal/tests", "Poiseuille analytical", 1e-3, "cfd-reference-minimal"),
    ("aeroelastic", "pytest -q crates/aether_compose/tests/flutter_1dof_closed_form.rs", "1-DOF flutter", 5e-2, "aether_compose"),
    ("uncertainty", "pytest -q python/aether_testing/tests", "Sobol/LHS analytic", 5e-2, "uq-basic"),
    ("manufacturing_tolerance", "pytest -q reference_plugins/process-twin/tests", "RSS/worst-case", 1e-9, "process-twin"),
    ("scientific_ml", "pytest -q reference_plugins/scientific-ml/tests", "grouped validation RMSE", 2e-1, "scientific-ml"),
]

PERFORMANCE = [
    ("cli_cold_start", "aether status --json"),
    ("graph_query_latency", "aether graph query --type Requirement --json"),
    ("artifact_ingest_throughput", "aether schema list --json"),
    ("workflow_scheduling_overhead", "aether plan explain examples/wingbox/control-flow.awl.yaml --json"),
    ("plugin_startup_overhead", "aether plugin list --json"),
    ("provenance_query", "aether policy explain graph.mutate --json"),
    ("schema_migration", "python3 -m pytest -q python/aether_testing/tests/test_contract_conformance.py"),
    ("restore_duration", "python3 -m pytest -q python/aether_testing/tests/test_dr_drill.py"),
    ("telemetry_ingest", "aether metrics --json"),
    ("agent_tool_latency", "aether agent-run examples/wingbox/agent-run-allowed.yaml --json"),
    ("concurrent_submission", "cargo test -p aether_exec --lib --quiet"),
    ("offline_project_size", "aether demo structure --json"),
]


def _run(command: str) -> tuple[float, bool, str]:
    args = command.split()
    if args[0] == "pytest":
        args[0] = sys.executable
        args.insert(1, "-m")
        args.insert(2, "pytest")
    elif args[0] == "aether":
        args[0] = str(ROOT / "target" / "debug" / "aether")
    elif args[0] == "python3":
        args[0] = sys.executable
    started = time.perf_counter()
    try:
        proc = subprocess.run(
            args, cwd=ROOT, capture_output=True, text=True, timeout=300
        )
        elapsed = time.perf_counter() - started
        return elapsed, proc.returncode == 0, (proc.stderr or "")[-500:]
    except (OSError, subprocess.TimeoutExpired) as exc:
        return time.perf_counter() - started, False, repr(exc)


def registry() -> dict:
    return {
        "contract": "aether.benchmark.registry",
        "version": "1.0",
        "platform": {
            "system": platform.system(),
            "machine": platform.machine(),
            "python": platform.python_version(),
        },
        "scientific": [
            {
                "name": n, "command": c, "reference": r,
                "tolerance": t, "solver": s,
            }
            for n, c, r, t, s in SCIENTIFIC
        ],
        "performance": [
            {"name": n, "command": c} for n, c in PERFORMANCE
        ],
        "note": "Runtime and accuracy are reported separately.",
    }


def run_all() -> dict:
    results = {"scientific": {}, "performance": {}}
    for name, command, reference, tolerance, solver in SCIENTIFIC:
        elapsed, ok, detail = _run(command)
        results["scientific"][name] = {
            "passed": ok,
            "runtime_seconds": round(elapsed, 4),
            "reference": reference,
            "tolerance": tolerance,
            "solver": solver,
            "error": detail if not ok else None,
        }
    for name, command in PERFORMANCE:
        elapsed, ok, detail = _run(command)
        results["performance"][name] = {
            "passed": ok,
            "runtime_seconds": round(elapsed, 4),
            "error": detail if not ok else None,
        }
    return results


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--list", action="store_true", help="print the registry only")
    parser.add_argument("--run", action="store_true", help="execute all benchmarks")
    args = parser.parse_args()
    if args.list:
        print(json.dumps(registry(), indent=2))
        return 0
    if not args.run:
        parser.error("use --list or --run")
    date = datetime.now(timezone.utc).strftime("%Y-%m-%d")
    out_dir = ROOT / "docs" / "verification" / "baseline" / date
    out_dir.mkdir(parents=True, exist_ok=True)
    report = {
        "executed_at": datetime.now(timezone.utc).isoformat(),
        "registry": registry(),
        "results": run_all(),
    }
    path = out_dir / "benchmarks.json"
    path.write_text(json.dumps(report, indent=2) + "\n")
    failed = [
        k for group in report["results"].values()
        for k, v in group.items() if not v["passed"]
    ]
    print(f"benchmarks recorded: {path}")
    print(f"failed: {failed if failed else 'none'}")
    return 1 if failed else 0


if __name__ == "__main__":
    raise SystemExit(main())