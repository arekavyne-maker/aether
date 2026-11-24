# SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
# SPDX-FileCopyrightText: 2026 Crocell Company
"""Conformance tests for the contract validation harness.

These tests are the executable form of the two-implementation rule
(ADR-0022). The Python implementation in `aether_sdk.validation` is
the first half; a Rust implementation is added in Phase 2 and must
agree on every test here.
"""

from __future__ import annotations

from pathlib import Path

import pytest
from aether_sdk.contracts import list_contracts
from aether_sdk.validation import (
    REPO_ROOT,
    ContractValidator,
    ValidationResult,
    load_contract_package,
)

# All contract ids must be present and their package manifests must load.
ALL_CONTRACTS = [
    "aether.plugin.manifest",
    "aether.physics.solver",
    "aether.compute.backend",
    "aether.io.format",
    "aether.graph.model",
    "aether.geometry.kernel",
    "aether.geometry.recipe",
    "aether.mesh.engine",
    "aether.field.model",
    "aether.requirement",
    "aether.physics.composable",
    "aether.uq.method",
    "aether.si.governor",
    "aether.optimizer",
    "aether.twin.estimator",
    "aether.surrogate.builder",
    "aether.manufacturing.process",
    "aether.inspection.device",
    "aether.test.rig",
    "aether.quantities",
    "aether.frames",
    "aether.asm",
    "aether.evidence",
    "aether.model-card",
    "aether.vv",
    "aether.research.promotion",
]


def test_all_contracts_are_listed() -> None:
    assert set(ALL_CONTRACTS).issubset(set(list_contracts()))


@pytest.mark.parametrize("contract_id", ALL_CONTRACTS)
def test_contract_package_manifest_loads(contract_id: str) -> None:
    package = load_contract_package(contract_id)
    assert package["contract_id"] == contract_id
    assert "version" in package
    # `schemas` may be declared in the package manifest or discovered from
    # the schema/ directory; the validator handles both (validation.py).
    assert "schemas" in package


@pytest.mark.parametrize("contract_id", ALL_CONTRACTS)
def test_contract_validator_constructs(contract_id: str) -> None:
    validator = ContractValidator(contract_id)
    schemas = validator.schemas()
    assert schemas, f"{contract_id} must declare or expose at least one schema"
    # Each schema entry must have a usable id and a path that resolves.
    for entry in schemas:
        assert entry["id"], f"Schema in {contract_id} missing id: {entry}"
        assert (validator.package_root / entry["path"]).exists(), (
            f"Schema path {entry['path']!r} for {contract_id} does not exist"
        )


# Plugin manifest — happy path (the OpenFOAM example)
def test_plugin_manifest_openfoam_example_validates() -> None:
    validator = ContractValidator("aether.plugin.manifest")
    schema_id = next(s["id"] for s in validator.schemas() if s["id"].endswith("manifest.schema.json"))
    example = REPO_ROOT / "contracts/plugin.manifest/examples/openfoam-adapter.manifest.yaml"
    result = validator.validate_file(example, schema_id=schema_id)
    assert result.valid, f"OpenFOAM example must validate: {result.errors}"


# Plugin manifest — negative case (missing required fields)
def test_plugin_manifest_rejects_minimal_invalid() -> None:
    validator = ContractValidator("aether.plugin.manifest")
    schema_id = next(s["id"] for s in validator.schemas() if s["id"].endswith("manifest.schema.json"))
    bad_doc: dict = {"plugin": {"id": "x", "name": "x", "version": "1.0.0", "vendor": "x"}}
    result = validator.validate(bad_doc, schema_id=schema_id, document_path="<test>")
    assert not result.valid
    assert result.errors, "Invalid document should produce at least one error"


# Compute backend — I-6: hardware-specific fields are rejected
def test_compute_backend_rejects_gpu_model_field() -> None:
    validator = ContractValidator("aether.compute.backend")
    schema_id = "https://aether.local/contracts/compute.backend/schema/resource-requirement.schema.json"
    bad_doc: dict = {
        "memory_gb": 8,
        "cores": 4,
        "parallelism": "multi_process",
        "accelerator": "optional",
        "accelerator_class": "any",
        "architecture": ["x86_64"],
        "gpu_model": "H100",  # I-6 violation
    }
    result = validator.validate(bad_doc, schema_id=schema_id, document_path="<test>")
    assert not result.valid, f"Hardware-specific field must be rejected (I-6): {result.errors}"


def test_compute_backend_accepts_vendor_neutral_requirement() -> None:
    validator = ContractValidator("aether.compute.backend")
    schema_id = "https://aether.local/contracts/compute.backend/schema/resource-requirement.schema.json"
    good_doc: dict = {
        "memory_gb": 64,
        "disk_gb": 500,
        "cores": 32,
        "parallelism": "distributed",
        "accelerator": "optional",
        "accelerator_class": "any",
        "architecture": ["x86_64", "aarch64"],
        "max_runtime_h": 48.0,
    }
    result = validator.validate(good_doc, schema_id=schema_id, document_path="<test>")
    assert result.valid, f"Vendor-neutral requirement must validate: {result.errors}"


# Physics solver — I-7: result bundle requires fidelity_class, validation_status, uncertainty
def test_physics_solver_result_requires_fidelity_metadata() -> None:
    validator = ContractValidator("aether.physics.solver")
    schema_id = "https://aether.local/contracts/physics.solver/schema/result-bundle.schema.json"
    bad_doc: dict = {
        "run_id": "01900000-0000-0000-0000-000000000000",
        "produced_at": "2026-08-30T00:00:00Z",
        "outputs": {"scalars": [], "fields": []},
        "provenance_ref": "sha256:abc",
    }
    result = validator.validate(bad_doc, schema_id=schema_id, document_path="<test>")
    assert not result.valid, "I-7: result bundle missing fidelity_class, validation_status, uncertainty must be rejected"
    # All three I-7 fields must appear in the errors.
    error_blob = " ".join(result.errors)
    assert "fidelity_class" in error_blob
    assert "validation_status" in error_blob
    assert "uncertainty" in error_blob


def test_physics_solver_result_accepts_complete_bundle() -> None:
    validator = ContractValidator("aether.physics.solver")
    schema_id = "https://aether.local/contracts/physics.solver/schema/result-bundle.schema.json"
    good_doc: dict = {
        "run_id": "01900000-0000-0000-0000-000000000000",
        "produced_at": "2026-08-30T00:00:00Z",
        "fidelity_class": "high_fidelity",
        "validation_status": "verified",
        "uncertainty": {"method": "convergence_residual", "bands": {}},
        "outputs": {"scalars": [], "fields": []},
        "provenance_ref": "sha256:abc",
    }
    result = validator.validate(good_doc, schema_id=schema_id, document_path="<test>")
    assert result.valid, f"Complete result bundle must validate: {result.errors}"


# Graph model — node types
def test_graph_model_requirement_node_validates() -> None:
    validator = ContractValidator("aether.graph.model")
    schema_id = "https://aether.local/contracts/graph.model/schema/node-type.schema.json"
    node = REPO_ROOT / "contracts/graph.model/catalog/nodes/requirement.node.json"
    result = validator.validate_file(node, schema_id=schema_id)
    assert result.valid, f"Requirement node-type must validate: {result.errors}"


# Graph model — decision node (autonomy class is required)
def test_graph_model_decision_node_requires_autonomy_class() -> None:
    validator = ContractValidator("aether.graph.model")
    schema_id = "https://aether.local/contracts/graph.model/schema/node-type.schema.json"
    decision_node = REPO_ROOT / "contracts/graph.model/catalog/nodes/decision.node.json"
    result = validator.validate_file(decision_node, schema_id=schema_id)
    assert result.valid, f"Decision node must validate: {result.errors}"


# IO format descriptors
@pytest.mark.parametrize(
    "descriptor_file",
    [
        "step-ap242.descriptor.yaml",
        "cgns.descriptor.yaml",
        "hdf5.descriptor.yaml",
    ],
)
def test_io_format_descriptor_validates(descriptor_file: str) -> None:
    validator = ContractValidator("aether.io.format")
    schema_id = "https://aether.local/contracts/io.format/schema/format-descriptor.schema.json"
    path = REPO_ROOT / f"contracts/io.format/examples/{descriptor_file}"
    result = validator.validate_file(path, schema_id=schema_id)
    assert result.valid, f"{descriptor_file} must validate: {result.errors}"
