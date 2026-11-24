# SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
# SPDX-FileCopyrightText: 2026 Crocell Company
"""Production-claim discipline check (A3-I22, §0.5, §22).

Research methods must never be presented as production capabilities. This
check scans current-facing docs for production claims attached to research
topics, and verifies that the research promotion contract exists.
"""

from __future__ import annotations

from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
CURRENT = [
    REPO_ROOT / "README.md",
    REPO_ROOT / "docs" / "ARCHITECTURE.md",
    REPO_ROOT / "docs" / "verification" / "STATUS.md",
    REPO_ROOT / "docs" / "roadmap" / "IMPLEMENTATION-PLAN.md",
]
RESEARCH_TERMS = [
    "neural operator",
    "PINN",
    "reaction-diffusion",
    "morphogenesis",
    "reservoir computing",
    "SINDy",
    "quality-diversity",
]
PRODUCTION_MARKERS = ["production-ready", "production-grade", "certified", "approved for release"]


def check(repo_root: Path = REPO_ROOT):
    from run import CheckResult

    problems: list[str] = []
    for path in CURRENT:
        if not path.is_file():
            continue
        text = path.read_text(encoding="utf-8", errors="ignore").lower()
        for term in RESEARCH_TERMS:
            if term.lower() not in text:
                continue
            for marker in PRODUCTION_MARKERS:
                # Flag only if the marker appears in the same line as a
                # research term.
                for line in text.splitlines():
                    if term.lower() in line and marker in line:
                        problems.append(f"{path.name}: {term!r} marked {marker!r}")
    if not (repo_root / "contracts" / "research.promotion" / "manifest.json").is_file():
        problems.append("missing contracts/research.promotion manifest")
    if problems:
        return CheckResult(
            name="research_claim",
            ok=False,
            summary=f"{len(problems)} research-claim problem(s)",
            details=problems[:10],
        )
    return CheckResult(
        name="research_claim",
        ok=True,
        summary="research methods are not claimed as production; promotion contract present",
        details=[],
    )


if __name__ == "__main__":
    result = check()
    print(result.summary)
    for detail in result.details:
        print(" ", detail)
    raise SystemExit(0 if result.ok else 1)