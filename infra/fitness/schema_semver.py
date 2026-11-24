# SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
# SPDX-FileCopyrightText: 2026 Crocell Company
"""Schema semver discipline check (Phase 0; ADR-0011).

Each contract package has a `manifest.json` declaring its `version`. Phase 0
checks:

- the version is a valid semver string;
- the contract id is reverse-DNS / kebab-style;
- every declared schema file exists and parses as JSON.

Phase 1 extends this with: schema evolution rules (no breaking changes
within a major), $id discipline, and contract freeze window enforcement.
"""

from __future__ import annotations

import json
import re
from pathlib import Path

SEMVER = re.compile(
    r"^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$"
)
CONTRACT_ID = re.compile(r"^aether(\.[a-z0-9](?:[a-z0-9-]*[a-z0-9])?)+$")


def check(repo_root: Path) -> "CheckResult":  # type: ignore[name-defined]
    from run import CheckResult

    contracts_dir = repo_root / "contracts"
    if not contracts_dir.is_dir():
        return CheckResult(
            name="schema_semver",
            ok=True,
            summary="no contracts/ directory yet",
            details=[],
        )

    errors: list[str] = []
    contract_count = 0
    schema_count = 0
    for package_dir in sorted(p for p in contracts_dir.iterdir() if p.is_dir()):
        manifest_path = package_dir / "manifest.json"
        if not manifest_path.is_file():
            continue
        contract_count += 1
        try:
            manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        except json.JSONDecodeError as e:
            errors.append(f"{package_dir.name}/manifest.json: invalid JSON: {e}")
            continue
        cid = manifest.get("contract_id", "")
        if not CONTRACT_ID.match(cid):
            errors.append(
                f"{package_dir.name}/manifest.json: contract_id {cid!r} does not match {CONTRACT_ID.pattern}"
            )
        version = manifest.get("version", "")
        if not isinstance(version, str) or not SEMVER.match(version):
            errors.append(
                f"{package_dir.name}/manifest.json: version {version!r} is not a valid semver"
            )
        for entry in manifest.get("schemas", []) or []:
            rel_path = entry.get("path")
            if not rel_path:
                errors.append(
                    f"{package_dir.name}/manifest.json: schema entry missing path: {entry}"
                )
                continue
            schema_path = package_dir / rel_path
            if not schema_path.is_file():
                errors.append(
                    f"{package_dir.name}/manifest.json: schema path {rel_path!r} does not exist"
                )
                continue
            try:
                schema = json.loads(schema_path.read_text(encoding="utf-8"))
            except json.JSONDecodeError as e:
                errors.append(f"{schema_path}: invalid JSON: {e}")
                continue
            if not isinstance(schema, dict) or "$schema" not in schema:
                errors.append(f"{schema_path}: missing $schema declaration")
                continue
            schema_count += 1

    if errors:
        return CheckResult(
            name="schema_semver",
            ok=False,
            summary=f"{len(errors)} schema/contract error(s) across {contract_count} contract(s)",
            details=errors,
        )
    return CheckResult(
        name="schema_semver",
        ok=True,
        summary=f"{contract_count} contract(s), {schema_count} schema(s) — all valid",
        details=[],
    )
