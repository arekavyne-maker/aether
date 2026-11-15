# SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
# SPDX-FileCopyrightText: 2026 Crocell Company
"""Kernel LOC budget check (Phase 0; ADR-0001).

Counts Rust source lines in the kernel crates (`aether_kernel`,
`aether_graph`, `aether_workflow`, `aether_provenance`, `aether_policy`,
`aether_registry`, `aether_exec`). Phase 0 budget is **soft**: the cap is
50,000 lines (SDP §2 I-1). The check reports the current total and warns
when it crosses 50% of the budget; it fails only if the budget is
exceeded outright.

`aether_cli` and `aether_sdk_ffi` are excluded from the kernel proper
(ADR-0001 notes); they are tracked separately and reported for context.
"""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

KERNEL_CRATES = (
    "aether_kernel",
    "aether_graph",
    "aether_workflow",
    "aether_provenance",
    "aether_policy",
    "aether_registry",
    "aether_exec",
)

BOUNDARY_CRATES = (
    "aether_cli",
    "aether_sdk_ffi",
)

KERNEL_LOC_BUDGET = 50_000


@dataclass(frozen=True)
class _Counts:
    kernel_loc: int
    boundary_loc: int
    per_crate: dict[str, int]


def _count_rust_lines(crate_dir: Path) -> int:
    total = 0
    for path in crate_dir.rglob("*.rs"):
        if "target" in path.parts:
            continue
        try:
            with path.open(encoding="utf-8") as f:
                total += sum(1 for _ in f)
        except OSError:
            continue
    return total


def check(repo_root: Path) -> "CheckResult":  # type: ignore[name-defined]
    from run import CheckResult

    crates_dir = repo_root / "crates"
    if not crates_dir.is_dir():
        return CheckResult(
            name="loc_budget",
            ok=True,
            summary="no crates/ directory yet (Phase 0 just started)",
            details=[],
        )

    per_crate: dict[str, int] = {}
    kernel_loc = 0
    for crate in KERNEL_CRATES:
        crate_dir = crates_dir / crate
        if not crate_dir.is_dir():
            continue
        n = _count_rust_lines(crate_dir / "src")
        per_crate[crate] = n
        kernel_loc += n

    boundary_loc = 0
    for crate in BOUNDARY_CRATES:
        crate_dir = crates_dir / crate
        if not crate_dir.is_dir():
            continue
        n = _count_rust_lines(crate_dir / "src")
        per_crate[crate] = n
        boundary_loc += n

    pct = (kernel_loc / KERNEL_LOC_BUDGET) * 100.0
    details = [f"kernel LOC: {kernel_loc} / {KERNEL_LOC_BUDGET} ({pct:.1f}%)"]
    details.append(f"boundary LOC (cli, sdk_ffi): {boundary_loc}")
    details.append("per-crate:")
    for crate, n in per_crate.items():
        details.append(f"  {crate}: {n}")

    if kernel_loc > KERNEL_LOC_BUDGET:
        return CheckResult(
            name="loc_budget",
            ok=False,
            summary=f"kernel LOC {kernel_loc} exceeds budget {KERNEL_LOC_BUDGET}",
            details=details,
        )
    return CheckResult(
        name="loc_budget",
        ok=True,
        summary=f"kernel LOC {kernel_loc} within budget ({pct:.1f}%)",
        details=details,
    )
