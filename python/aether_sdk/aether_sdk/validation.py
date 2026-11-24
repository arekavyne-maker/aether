# SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
# SPDX-FileCopyrightText: 2026 Crocell Company
"""Aether contract validation harness.

This module is the **Python implementation** of the two-implementation rule
(ADR-0022) for contract schemas. Every contract in `contracts/` is validated
here using `jsonschema` (draft 2020-12). A second, independent implementation
in Rust is added in Phase 2.

The validation harness is the foundation of contract conformance — and the
first executable artifact in the two-impl rule.
"""

from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any

import yaml
from jsonschema import Draft202012Validator
from jsonschema.exceptions import ValidationError

# Initial contract catalog. Each entry maps contract id to the path of its
# package manifest. G4: 13 contracts (Phase 0/G1: 5; G3: 10).
CONTRACT_MANIFESTS: dict[str, Path] = {
    "aether.plugin.manifest": Path("contracts/plugin.manifest/manifest.json"),
    "aether.physics.solver": Path("contracts/physics.solver/manifest.json"),
    "aether.compute.backend": Path("contracts/compute.backend/manifest.json"),
    "aether.io.format": Path("contracts/io.format/manifest.json"),
    "aether.graph.model": Path("contracts/graph.model/manifest.json"),
    "aether.geometry.kernel": Path("contracts/geometry.kernel/manifest.json"),
    "aether.geometry.recipe": Path("contracts/geometry.recipe/manifest.json"),
    "aether.mesh.engine": Path("contracts/mesh.engine/manifest.json"),
    "aether.field.model": Path("contracts/field.model/manifest.json"),
    "aether.requirement": Path("contracts/requirement/manifest.json"),
    "aether.physics.composable": Path("contracts/physics.composable/manifest.json"),
    "aether.uq.method": Path("contracts/uq.method/manifest.json"),
    "aether.si.governor": Path("contracts/si.governor/manifest.json"),
    "aether.optimizer": Path("contracts/optimizer/manifest.json"),
    "aether.twin.estimator": Path("contracts/twin.estimator/manifest.json"),
    "aether.surrogate.builder": Path("contracts/surrogate/manifest.json"),
    "aether.manufacturing.process": Path("contracts/manufacturing.process/manifest.json"),
    "aether.inspection.device": Path("contracts/inspection.device/manifest.json"),
    "aether.test.rig": Path("contracts/test.rig/manifest.json"),
    "aether.quantities": Path("contracts/quantities/manifest.json"),
    "aether.frames": Path("contracts/frames/manifest.json"),
    "aether.asm": Path("contracts/asm/manifest.json"),
    "aether.evidence": Path("contracts/evidence/manifest.json"),
    "aether.model-card": Path("contracts/model-card/manifest.json"),
    "aether.vv": Path("contracts/vv/manifest.json"),
}

REPO_ROOT = Path(__file__).resolve().parents[3]


def _abs(path: Path) -> Path:
    """Resolve a path relative to the monorepo root."""
    if path.is_absolute():
        return path
    return REPO_ROOT / path


def contract_path(contract_id: str) -> Path:
    """Return the package root for a given contract id."""
    if contract_id not in CONTRACT_MANIFESTS:
        raise KeyError(f"Unknown contract: {contract_id!r}")
    return _abs(CONTRACT_MANIFESTS[contract_id]).parent


def load_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def load_yaml(path: Path) -> Any:
    return yaml.safe_load(path.read_text(encoding="utf-8"))


def load_contract_package(contract_id: str) -> dict[str, Any]:
    """Load the top-level package manifest for a contract."""
    manifest_path = _abs(CONTRACT_MANIFESTS[contract_id])
    return load_json(manifest_path)


def _load_schema(schema_rel_path: str, package_root: Path) -> dict[str, Any]:
    return load_json(package_root / schema_rel_path)


@dataclass(frozen=True)
class ValidationResult:
    """Result of validating one document against one schema."""

    contract_id: str
    schema_id: str
    document_path: str
    valid: bool
    errors: tuple[str, ...]


class ContractValidator:
    """Validates documents against a contract's schemas.

    This is the Python half of the two-implementation rule. A Rust
    implementation is added in Phase 2; both must accept or reject the
    same set of documents (MVR-01/02/06/07).
    """

    def __init__(self, contract_id: str) -> None:
        if contract_id not in CONTRACT_MANIFESTS:
            raise KeyError(f"Unknown contract: {contract_id!r}")
        self.contract_id = contract_id
        self.package_root = contract_path(contract_id)
        self.package_manifest = load_contract_package(contract_id)

    def schemas(self) -> list[dict[str, Any]]:
        """Return the list of schemas declared by the package manifest.

        If the package manifest's `schemas` array is empty, the validator
        falls back to discovering `*.schema.json` files in the package's
        `schema/` subdirectory. This keeps the schema list discoverable
        without requiring manifest edits for every new schema.
        """
        declared = list(self.package_manifest.get("schemas", []))
        if declared:
            return declared
        schema_dir = self.package_root / "schema"
        if not schema_dir.is_dir():
            return []
        discovered: list[dict[str, Any]] = []
        for path in sorted(schema_dir.glob("*.schema.json")):
            try:
                data = load_json(path)
            except (OSError, ValueError):
                continue
            discovered.append(
                {
                    "id": data.get("$id", str(path)),
                    "path": str(path.relative_to(self.package_root)),
                    "schema_version": data.get("$schema", ""),
                }
            )
        return discovered

    def _validator_for(self, schema_id: str) -> Draft202012Validator:
        for entry in self.schemas():
            if entry.get("id") == schema_id:
                schema = _load_schema(entry["path"], self.package_root)
                return Draft202012Validator(schema)
        raise KeyError(f"Schema {schema_id!r} not declared in {self.contract_id}")

    def validate(
        self, document: Any, *, schema_id: str, document_path: str = "<memory>"
    ) -> ValidationResult:
        validator = self._validator_for(schema_id)
        errors = sorted(validator.iter_errors(document), key=lambda e: e.path)
        if not errors:
            return ValidationResult(
                contract_id=self.contract_id,
                schema_id=schema_id,
                document_path=document_path,
                valid=True,
                errors=(),
            )
        return ValidationResult(
            contract_id=self.contract_id,
            schema_id=schema_id,
            document_path=document_path,
            valid=False,
            errors=tuple(_format_error(e) for e in errors),
        )

    def validate_file(
        self, file_path: Path, *, schema_id: str
    ) -> ValidationResult:
        document = load_yaml(file_path) if file_path.suffix in {".yaml", ".yml"} else load_json(file_path)
        # Strip the harness-only `_schema_id` field before validating
        # against the contract schema (matches the Rust harness).
        if isinstance(document, dict) and "_schema_id" in document:
            document = {k: v for k, v in document.items() if k != "_schema_id"}
        return self.validate(document, schema_id=schema_id, document_path=str(file_path))


def _format_error(error: ValidationError) -> str:
    path = "/".join(str(p) for p in error.absolute_path) or "<root>"
    return f"{path}: {error.message}"


__all__ = [
    "CONTRACT_MANIFESTS",
    "ContractValidator",
    "REPO_ROOT",
    "ValidationResult",
    "contract_path",
    "load_contract_package",
    "load_json",
    "load_yaml",
]
