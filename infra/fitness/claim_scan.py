# SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
# SPDX-FileCopyrightText: 2026 Crocell Company
"""Flag stale or unsupported status claims in current-facing documents."""

from __future__ import annotations

from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
CURRENT_DOCS = [
    REPO_ROOT / "README.md",
    REPO_ROOT / "docs" / "ARCHITECTURE.md",
    REPO_ROOT / "docs" / "verification" / "STATUS.md",
    REPO_ROOT / "site" / "index.html",
]
BANNED = [
    "EIC Pathfinder",
    "315 tests passing",
    "11/11 passing",
    "production-ready platform",
    "certified by",
]


def check(root: Path = REPO_ROOT):
    from run import CheckResult

    hits: list[str] = []
    for path in CURRENT_DOCS:
        if not path.exists():
            continue
        text = path.read_text(encoding="utf-8", errors="ignore")
        for phrase in BANNED:
            if phrase in text:
                hits.append(f"{path.relative_to(root)}: {phrase}")
    if hits:
        return CheckResult("claim_scan", False, f"{len(hits)} stale claim(s)", hits)
    return CheckResult("claim_scan", True, "no stale status claims found", [])


if __name__ == "__main__":
    result = check()
    print(result.summary)
    for detail in result.details:
        print(" ", detail)
    raise SystemExit(0 if result.ok else 1)
