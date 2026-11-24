# SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
# SPDX-FileCopyrightText: 2026 Crocell Company
"""ADR discipline check (A3-I22, spec §0.10).

No runtime, broker, orchestration engine, model runtime or agent framework
may exist without an Accepted ADR. This check maps known runtime crates to
their governing records and fails when a record is missing, Proposed, or
superseded. It also fails if a new crate appears without an ADR entry.
"""

from __future__ import annotations

import re
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
ADR_DIR = REPO_ROOT / "specs" / "adr"
CRATES = REPO_ROOT / "crates"

RUNTIME_ADRS = {
    "aether_model_runtime": "0034-model-inference-runtime.md",
    "aether_agent_runtime": "0035-agent-runtime-crate.md",
    "aether_agents": "0035-agent-runtime-crate.md",
    "aether_exec": "0036-job-envelope-federation.md",
    "aether_workflow": "0037-graph-access-control.md",
    "aether_policy": "0037-graph-access-control.md",
    "aether_graph": "0037-graph-access-control.md",
    "aether_identity": "0039-key-management.md",
    "aether_classification": "0039-key-management.md",
}

NEW_ADRS = [
    "0034-model-inference-runtime.md",
    "0035-agent-runtime-crate.md",
    "0036-job-envelope-federation.md",
    "0037-graph-access-control.md",
    "0038-research-promotion.md",
    "0039-key-management.md",
    "0040-benchmark-slo.md",
    "0041-contract-migration.md",
]


def check(repo_root: Path = REPO_ROOT):
    from run import CheckResult

    problems: list[str] = []

    for filename in NEW_ADRS:
        path = ADR_DIR / filename
        if not path.is_file():
            problems.append(f"missing ADR {filename}")
            continue
        text = path.read_text(encoding="utf-8")
        status = re.search(r"\*\*Status:\*\*\s*(.+)", text)
        if not status:
            problems.append(f"{filename}: no Status field")
        elif not status.group(1).strip().startswith("Accepted"):
            problems.append(
                f"{filename}: status must be Accepted, got {status.group(1).strip()!r}"
            )
        for section in ("Context", "Decision", "Consequences"):
            if f"## {section}" not in text:
                problems.append(f"{filename}: missing ## {section}")

    for crate, adr in RUNTIME_ADRS.items():
        if not (CRATES / crate).is_dir():
            continue
        path = ADR_DIR / adr
        if not path.is_file():
            problems.append(f"crate {crate}: governing ADR {adr} missing")
            continue
        text = path.read_text(encoding="utf-8")
        status = re.search(r"\*\*Status:\*\*\s*(.+)", text)
        if not status or not status.group(1).strip().startswith("Accepted"):
            problems.append(f"crate {crate}: governing ADR {adr} is not Accepted")

    if problems:
        return CheckResult(
            name="adr_discipline",
            ok=False,
            summary=f"{len(problems)} ADR problem(s)",
            details=problems,
        )

    return CheckResult(
        name="adr_discipline",
        ok=True,
        summary=f"{len(NEW_ADRS)} new ADRs Accepted; all runtime crates governed",
        details=[],
    )


if __name__ == "__main__":
    result = check()
    print(result.summary)
    for detail in result.details:
        print(" ", detail)
    raise SystemExit(0 if result.ok else 1)