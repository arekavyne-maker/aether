# SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
# SPDX-FileCopyrightText: 2026 Crocell Company
"""Benchmark coverage check (ADR-0040, spec §17.2, §21).

Every declared benchmark must have a runnable command. Runtime and accuracy
must be reported separately. A missing category is a failure, not a skip.
"""

from __future__ import annotations

import importlib.util
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]


def check(repo_root: Path = REPO_ROOT):
    from run import CheckResult

    spec = importlib.util.spec_from_file_location(
        "benchmarks_registry", repo_root / "infra" / "benchmarks_registry.py"
    )
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    reg = module.registry()
    problems: list[str] = []
    science = {b["name"] for b in reg["scientific"]}
    perf = {b["name"] for b in reg["performance"]}
    for name in [
        "beam_mechanics", "buckling", "thermal_conduction", "plate_mechanics",
        "composite_laminate", "architected_homogenisation", "cfd_canonical",
        "aeroelastic", "uncertainty", "manufacturing_tolerance", "scientific_ml",
    ]:
        if name not in science:
            problems.append(f"missing scientific benchmark {name}")
    for name in [
        "cli_cold_start", "graph_query_latency", "artifact_ingest_throughput",
        "workflow_scheduling_overhead", "plugin_startup_overhead", "provenance_query",
        "schema_migration", "restore_duration", "telemetry_ingest",
        "agent_tool_latency", "concurrent_submission", "offline_project_size",
    ]:
        if name not in perf:
            problems.append(f"missing performance benchmark {name}")
    for group in ("scientific", "performance"):
        for entry in reg[group]:
            if not entry.get("command"):
                problems.append(f"{group}/{entry['name']}: no command")
    if reg.get("note", "").find("separately") < 0:
        problems.append("registry must state runtime and accuracy are separate")
    if problems:
        return CheckResult(
            name="benchmark_coverage", ok=False,
            summary=f"{len(problems)} benchmark coverage problem(s)",
            details=problems,
        )
    return CheckResult(
        name="benchmark_coverage", ok=True,
        summary=f"{len(science)} scientific + {len(perf)} performance benchmarks declared",
        details=[],
    )


if __name__ == "__main__":
    result = check()
    print(result.summary)
    for d in result.details:
        print(" ", d)
    raise SystemExit(0 if result.ok else 1)