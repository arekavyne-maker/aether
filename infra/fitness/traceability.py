# SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
# SPDX-FileCopyrightText: 2026 Crocell Company
"""Validate the verification traceability matrix structure."""

from __future__ import annotations

from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
MATRIX = REPO_ROOT / "docs" / "verification" / "TRACEABILITY-MATRIX.md"
REQUIRED = ["Requirement", "Source", "Implementation", "Test / evidence", "Status"]
VALID_STATUS = {"planned", "implemented", "verified", "blocked"}


def check(root: Path = REPO_ROOT):
    from run import CheckResult

    if not MATRIX.exists():
        return CheckResult("traceability", False, "matrix missing", [str(MATRIX)])

    lines = MATRIX.read_text(encoding="utf-8").splitlines()
    rows = [line for line in lines if line.startswith("| A3-REQ-")]
    problems: list[str] = []
    seen: set[str] = set()

    for row in rows:
        cells = [c.strip() for c in row.strip().strip("|").split("|")]
        if len(cells) < 5:
            problems.append(f"malformed row: {row[:80]}")
            continue
        req, status = cells[0], cells[-1]
        if req in seen:
            problems.append(f"duplicate requirement: {req}")
        seen.add(req)
        if status not in VALID_STATUS:
            problems.append(f"{req}: invalid status {status!r}")
        if status in {"implemented", "verified"} and not cells[3]:
            problems.append(f"{req}: no test/evidence reference")

    if problems:
        return CheckResult("traceability", False, f"{len(problems)} problem(s)", problems)
    return CheckResult(
        "traceability",
        True,
        f"{len(rows)} requirements, {len(seen)} unique, statuses valid",
        [],
    )


if __name__ == "__main__":
    result = check()
    print(result.summary)
    for detail in result.details:
        print(" ", detail)
    raise SystemExit(0 if result.ok else 1)
