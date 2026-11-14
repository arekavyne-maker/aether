# SPDX-License-Identifier: Aether-VPSL-1.0
# SPDX-FileCopyrightText: 2026 The Aether Consortium
#!/usr/bin/env python3
"""Aether architecture fitness pipeline (Phase 0 skeleton).

This is the executable form of §19.5. Each check is a script in
`infra/fitness/`; the umbrella runner here is what CI invokes.

The four checks in Phase 0 are:
- `loc_budget.py`     — kernel stays within the ~50k LOC cap (I-1)
- `token_scan.py`     — no prohibited tokens in the codebase (I-3)
- `schema_semver.py`  — contract schemas respect semver discipline (I-11)
- `sbom.py`           — emit an SBOM (SPDX 2.3) for the current tree (SDP §16)

Phase 1+ adds: import-linter, fuzzing, cross-arch matrix, conformance
suites per touched contract, MVR-suite, doc/spec regen.
"""

from __future__ import annotations

import argparse
import json
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Sequence

REPO_ROOT = Path(__file__).resolve().parents[2]


@dataclass(frozen=True)
class CheckResult:
    name: str
    ok: bool
    summary: str
    details: list[str]


def run_loc_budget() -> CheckResult:
    from loc_budget import check
    return check(REPO_ROOT)


def run_token_scan() -> CheckResult:
    from token_scan import check
    return check(REPO_ROOT)


def run_schema_semver() -> CheckResult:
    from schema_semver import check
    return check(REPO_ROOT)


def run_sbom() -> CheckResult:
    from sbom import check
    return check(REPO_ROOT)


def run_geom_fixture_label() -> CheckResult:
    from geom_fixture_label import check
    return check(REPO_ROOT)


def run_ccx_binary() -> CheckResult:
    from ccx_binary import check
    return check(REPO_ROOT)


def run_rosetta_cross_arch() -> CheckResult:
    from rosetta_cross_arch import check
    return check(REPO_ROOT)


def run_fuzz_contracts() -> CheckResult:
    from fuzz_contracts import check
    return check(REPO_ROOT)


CHECKS = {
    "loc_budget": run_loc_budget,
    "token_scan": run_token_scan,
    "schema_semver": run_schema_semver,
    "sbom": run_sbom,
    "geom_fixture_label": run_geom_fixture_label,
    "ccx_binary": run_ccx_binary,
    "rosetta_cross_arch": run_rosetta_cross_arch,
    "fuzz_contracts": run_fuzz_contracts,
}


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument(
        "checks",
        nargs="*",
        default=["all"],
        help="Which checks to run (default: all).",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="Emit a JSON report (one CheckResult per check).",
    )
    args = parser.parse_args(argv)

    # Manual choices validation (argparse with nargs="*" + choices checks
    # the whole default list as one value on some Python versions, which
    # made the no-argument invocation fail on Python 3.9/3.10).
    valid = set(CHECKS.keys()) | {"all"}
    unknown = [c for c in args.checks if c not in valid]
    if unknown:
        parser.error(
            "argument checks: invalid choice: %r (choose from %s)"
            % (unknown, ", ".join(sorted(valid)))
        )

    selected: list[str]
    if "all" in args.checks:
        selected = list(CHECKS.keys())
    else:
        selected = list(args.checks)

    results: list[CheckResult] = []
    for name in selected:
        results.append(CHECKS[name]())

    if args.json:
        report = {
            "fitness_pipeline": "phase-0",
            "checks": [
                {
                    "name": r.name,
                    "ok": r.ok,
                    "summary": r.summary,
                    "details": r.details,
                }
                for r in results
            ],
        }
        print(json.dumps(report, indent=2))
    else:
        for r in results:
            mark = "OK " if r.ok else "FAIL"
            print(f"[{mark}] {r.name}: {r.summary}")
            for d in r.details:
                print(f"       {d}")

    failed = [r for r in results if not r.ok]
    if failed:
        print(f"\n{len(failed)}/{len(results)} checks failed.", file=sys.stderr)
        return 1
    print(f"\n{len(results)}/{len(results)} checks passed.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
