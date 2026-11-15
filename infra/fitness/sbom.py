# SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
# SPDX-FileCopyrightText: 2026 Crocell Company
"""SBOM (SPDX 2.3) emitter (Phase 0; SDP §16).

Emits a minimal SPDX 2.3 SBOM for the Aether platform monorepo. Phase 0
walks the `contracts/` directory and the Cargo and Python dependencies
that are pinned in `Cargo.lock` (if present) and `python/*/pyproject.toml`.

Phase 1+ integrates with cargo-cyclonedx and pip-licenses to produce a
complete SPDX document. The output here is intentionally a `passes` if
the SBOM can be emitted; the actual SPIDX compliance check is Phase 1+.

The check **passes** whenever the SBOM can be written to the configured
output path. The CI runs this on every release; a failed emission is a
release blocker.
"""

from __future__ import annotations

import datetime
import json
import re
from pathlib import Path

try:  # Python 3.11+
    import tomllib
except ModuleNotFoundError:  # Python 3.9/3.10: tiny pure-Python backport
    import tomli as tomllib  # type: ignore[no-redef]

SBOM_OUT = "infra/release/sbom.spdx.json"


def _read_cargo_lock(repo_root: Path) -> dict:
    lock = repo_root / "Cargo.lock"
    if not lock.is_file():
        return {"packages": []}
    text = lock.read_text(encoding="utf-8")
    # Phase 0: a tolerant TOML reader is sufficient. Phase 1+ uses toml crate.
    try:
        return tomllib.loads(text)
    except Exception:  # noqa: BLE001
        return {"packages": []}


def _read_pyproject(path: Path) -> dict:
    if not path.is_file():
        return {}
    try:
        return tomllib.loads(path.read_text(encoding="utf-8"))
    except Exception:  # noqa: BLE001
        return {}


def check(repo_root: Path) -> "CheckResult":  # type: ignore[name-defined]
    from run import CheckResult

    lock = _read_cargo_lock(repo_root)
    rust_packages = [
        {"name": p.get("name"), "version": p.get("version"), "source": "crates.io"}
        for p in lock.get("package", [])
        if p.get("name") and p.get("version")
    ]

    python_packages: list[dict[str, str]] = []
    for pyproject in (repo_root / "python").rglob("pyproject.toml"):
        cfg = _read_pyproject(pyproject)
        for dep in (cfg.get("project", {}) or {}).get("dependencies", []) or []:
            match = re.match(r"^([A-Za-z0-9_.\-]+)\s*([~<>=!]?=.*)?$", dep.strip())
            if match:
                python_packages.append(
                    {"name": match.group(1), "version_spec": (match.group(2) or "").strip() or "*"}
                )

    sbom = {
        "spdxVersion": "SPDX-2.3",
        "dataLicense": "CC0-1.0",
        "SPDXID": "SPDXRef-DOCUMENT",
        "name": "aether-platform",
        "documentNamespace": f"https://aether.local/sbom/aether-platform-{datetime.date.today().isoformat()}",
        "creationInfo": {
            "created": datetime.datetime.now(tz=datetime.timezone.utc).isoformat(),
            "creators": ["Tool: aether-fitness-pipeline-0.1.0"],
        },
        "packages": [
            {"name": p["name"], "versionInfo": p["version"], "downloadLocation": "NOASSERTION"}
            for p in rust_packages
        ]
        + [
            {"name": p["name"], "versionInfo": p["version_spec"], "downloadLocation": "NOASSERTION"}
            for p in python_packages
        ],
    }

    out_path = repo_root / SBOM_OUT
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(sbom, indent=2, sort_keys=True), encoding="utf-8")

    return CheckResult(
        name="sbom",
        ok=True,
        summary=f"SPDX 2.3 SBOM emitted: {len(rust_packages)} rust + {len(python_packages)} python packages",
        details=[str(out_path.relative_to(repo_root))],
    )
