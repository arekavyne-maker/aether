# SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
# SPDX-FileCopyrightText: 2026 Crocell Company
"""Check that relative Markdown links resolve inside the repository."""

from __future__ import annotations

import re
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
LINK_RE = re.compile(r"\[[^\]]*\]\(([^)\s]+)(?:\s+\"[^\"]*\")?\)")
SKIP_PREFIXES = ("http://", "https://", "mailto:", "artifact://", "#")


def _iter_markdown() -> list[Path]:
    return [
        p
        for p in REPO_ROOT.rglob("*.md")
        if ".git" not in p.parts and "target" not in p.parts
    ]


def check(root: Path = REPO_ROOT):
    from run import CheckResult

    broken: list[str] = []
    checked = 0
    for path in _iter_markdown():
        text = path.read_text(encoding="utf-8", errors="ignore")
        for match in LINK_RE.finditer(text):
            target = match.group(1).split("#", 1)[0]
            if not target or target.startswith(SKIP_PREFIXES):
                continue
            checked += 1
            resolved = (path.parent / target).resolve()
            try:
                resolved.relative_to(root.resolve())
            except ValueError:
                broken.append(f"{path.relative_to(root)} -> {target} (outside repo)")
                continue
            if not resolved.exists():
                broken.append(f"{path.relative_to(root)} -> {target}")
    if broken:
        return CheckResult(
            "docs_links",
            False,
            f"{len(broken)} broken link(s) across {checked} checked",
            broken[:40],
        )
    return CheckResult(
        "docs_links",
        True,
        f"{checked} relative links resolve",
        [],
    )


if __name__ == "__main__":
    result = check()
    print(result.summary)
    for detail in result.details:
        print(" ", detail)
    raise SystemExit(0 if result.ok else 1)
