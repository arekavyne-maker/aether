# SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
# SPDX-FileCopyrightText: 2026 Crocell Company
"""Scientific toolchain check (Aether 3.0 Phase A).

The real-geometry and real-FEA plugins depend on a pinned scientific stack.
This check verifies that the stack is present and at the declared versions,
so a build cannot silently fall back to a fixture while claiming to be real.

Absent *optional* native tools (CalculiX) are reported as a skip, not a
failure: the adapter keeps its honest label and the benchmark reports SKIP.
Absent *required* Python packages are a hard failure.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]

# Packages the reference plugins genuinely import. A missing one means the
# "real" claim is unsupported and the build must fail.
REQUIRED = {
    "numpy": r"^2\.0\.2$",
    "scipy": r"^1\.13\.1$",
    "gmsh": r"^4\.15\.2$",
    "meshio": r"^5\.3\.5$",
    "trimesh": r"^4\.12\.2$",
    "OCP": r"^7\.7\.2",
}

CCX = REPO_ROOT / "vendor" / "bin" / "ccx_2.23"


def _version(module: str) -> str | None:
    try:
        mod = __import__(module)
    except Exception:
        return None
    return str(getattr(mod, "__version__", "unknown"))


def check(repo_root: Path = REPO_ROOT):
    from run import CheckResult

    problems: list[str] = []
    found: list[str] = []

    for module, pattern in REQUIRED.items():
        version = _version(module)
        if version is None:
            problems.append(f"{module}: not importable")
            continue
        if not re.match(pattern, version):
            problems.append(f"{module}: version {version} does not match {pattern}")
            continue
        found.append(f"{module}=={version}")

    if problems:
        return CheckResult(
            name="toolchain",
            ok=False,
            summary=f"{len(problems)} required package problem(s)",
            details=problems
            + ["fix: ./infra/bootstrap_env.sh"],
        )

    # CalculiX is optional: report SKIP when absent, but never a silent pass.
    if not CCX.is_file():
        return CheckResult(
            name="toolchain",
            ok=False,
            skipped=True,
            summary=f"required stack OK ({len(found)} packages); CalculiX absent",
            details=found
            + [
                "calculix: absent; BENCH-08 and the CalculiX adapter keep their honest labels",
                "fix: place a ccx_2.x binary at vendor/bin/ccx_2.23 or set $CCX_BIN",
            ],
        )

    return CheckResult(
        name="toolchain",
        ok=True,
        summary=f"required stack OK ({len(found)} packages) + CalculiX present",
        details=found + ["calculix: vendor/bin/ccx_2.23"],
    )


if __name__ == "__main__":
    result = check()
    print(result.summary)
    for detail in result.details:
        print(" ", detail)
    raise SystemExit(0 if result.ok or result.skipped else 1)