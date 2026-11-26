# SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
# SPDX-FileCopyrightText: 2026 Crocell Company
"""Fitness check for the cleaned repository layout."""

from __future__ import annotations

import subprocess
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]

REQUIRED = (
    "experimental/README.md",
    "experimental/web/app/index.html",
    "experimental/web/app/app.css",
    "experimental/desktop/Cargo.toml",
    "experimental/desktop/src/main.rs",
    "experimental/desktop/AppIcon.png",
    "experimental/macos/macos-app.sh",
    "experimental/windows/windows-app.ps1",
    "experimental/windows/make-ico.py",
    "experimental/linux/linux-app.sh",
    "scripts/README.md",
    "docs/research/AETHER-3.0-ADAPTIVE-AEROSTRUCTURES.md",
)

FORBIDDEN = (
    "site/app",
    "apps/desktop",
    "scripts/package/macos-app.sh",
    "scripts/package/windows-app.ps1",
    "scripts/package/linux-app.sh",
    "scripts/package/make-ico.py",
)


def check(repo_root: Path = REPO_ROOT):
    from run import CheckResult

    problems: list[str] = []
    details: list[str] = []

    for relative in REQUIRED:
        if not (repo_root / relative).is_file():
            problems.append(f"missing {relative}")

    if (repo_root / "new age.md").exists():
        problems.append("root research brief was not moved into docs/research")

    try:
        tracked = subprocess.run(
            ["git", "ls-files"], cwd=repo_root, check=True,
            capture_output=True, text=True, timeout=60,
        ).stdout.splitlines()
    except Exception as exc:  # pragma: no cover - git is always present in CI
        return CheckResult(name="repo_layout", ok=False,
                           summary=f"cannot list tracked files: {exc}", details=[])

    offenders: list[str] = []
    for relative in tracked:
        path = repo_root / relative
        if path.suffix in (".png", ".woff2", ".icns", ".jpg", ".lock"):
            continue
        try:
            text = path.read_text()
        except (UnicodeDecodeError, OSError):
            continue
        for token in FORBIDDEN:
            if token in text:
                offenders.append(f"{relative} references {token}")
    if offenders:
        problems.extend(sorted(set(offenders)))

    if problems:
        return CheckResult(name="repo_layout", ok=False,
                           summary=f"{len(problems)} layout problem(s)",
                           details=problems)
    return CheckResult(
        name="repo_layout",
        ok=True,
        summary="applications separated under experimental/; no stale paths",
        details=[f"{len(REQUIRED)} required paths present",
                 f"{len(tracked)} tracked files scanned for stale paths"],
    )
