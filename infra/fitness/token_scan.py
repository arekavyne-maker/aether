# SPDX-License-Identifier: Aether-VPSL-1.0
# SPDX-FileCopyrightText: 2026 The Aether Consortium
"""Forbidden-token scan (Phase 0; ADR-0003, §2 I-3).

The scan looks for branch-by-vendor and other prohibited patterns. Phase 0
keeps the list small and focused on the most common drift cases; the list
is expanded in Phase 1 as we observe new failure modes.

Tokens are scanned over the entire `crates/` and `python/` trees, excluding
`target/`, `.venv/`, and `node_modules/`.
"""

from __future__ import annotations

import re
from dataclasses import dataclass
from pathlib import Path

# Prohibited tokens (Phase 0 v0). Each entry is (pattern, reason, suggested_fix).
PROHIBITED: list[tuple[re.Pattern[str], str, str]] = [
    (re.compile(r"\bif\s+solver\s*=="), "branch by solver identity (I-3)", "use capability matching via the registry"),
    (re.compile(r"\bif\s+solver\s*!="), "branch by solver identity (I-3)", "use capability matching via the registry"),
    (re.compile(r"\bis_nvidia\b"), "vendor-specific branch (I-3 / I-6)", "declare accelerator_class in the resource requirement"),
    (re.compile(r"\bis_amd\b"), "vendor-specific branch (I-3 / I-6)", "declare accelerator_class in the resource requirement"),
    (re.compile(r"\bis_intel\b"), "vendor-specific branch (I-3 / I-6)", "declare architecture in the resource requirement"),
    (re.compile(r"\bcad_vendor\b"), "vendor-specific branch (I-3)", "use the geometry.kernel contract"),
    (re.compile(r"\bif\s+vendor\s*=="), "branch by vendor (I-3)", "use capability matching via the registry"),
    (re.compile(r"\bif\s+database_engine\s*=="), "branch by database engine (I-3)", "use the storage SPI"),
    (re.compile(r"\bgpu_model\s*=\s*['\"]"), "hardware-specific field in code (I-6)", "remove; declare accelerator_class instead"),
    (re.compile(r"\bcpu_vendor\s*=\s*['\"]"), "hardware-specific field in code (I-6)", "remove; declare architecture instead"),
]

EXCLUDE_DIRS = {"target", ".venv", "venv", "env", "node_modules", ".git", "__pycache__", "dist", "build"}
SCAN_EXTS = {".rs", ".py", ".toml", ".yaml", ".yml", ".sh", ".md"}


def _is_scannable(path: Path) -> bool:
    if any(part in EXCLUDE_DIRS for part in path.parts):
        return False
    return path.suffix in SCAN_EXTS


@dataclass
class _Hit:
    path: Path
    line: int
    snippet: str
    pattern_name: str
    reason: str
    fix: str


def check(repo_root: Path) -> "CheckResult":  # type: ignore[name-defined]
    from run import CheckResult

    hits: list[_Hit] = []
    for root in (repo_root / "crates", repo_root / "python", repo_root / "scripts"):
        if not root.is_dir():
            continue
        for path in root.rglob("*"):
            if not path.is_file() or not _is_scannable(path):
                continue
            try:
                text = path.read_text(encoding="utf-8")
            except (OSError, UnicodeDecodeError):
                continue
            for ln, line in enumerate(text.splitlines(), 1):
                for pat, reason, fix in PROHIBITED:
                    if pat.search(line):
                        # Skip line if it's inside a comment that names the
                        # prohibited token by way of explanation. Phase 0
                        # # noqa: AETHER token-scan.
                        if "# noqa: AETHER token-scan" in line:
                            continue
                        hits.append(
                            _Hit(
                                path=path,
                                line=ln,
                                snippet=line.strip()[:200],
                                pattern_name=pat.pattern,
                                reason=reason,
                                fix=fix,
                            )
                        )

    if not hits:
        return CheckResult(
            name="token_scan",
            ok=True,
            summary="no prohibited tokens found",
            details=[],
        )

    details = [
        f"{h.path.relative_to(repo_root)}:{h.line} [{h.reason}] {h.snippet}"
        for h in hits[:50]
    ]
    if len(hits) > 50:
        details.append(f"... and {len(hits) - 50} more (truncated)")
    return CheckResult(
        name="token_scan",
        ok=False,
        summary=f"{len(hits)} prohibited token(s) found",
        details=details,
    )
