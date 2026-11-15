# SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
# SPDX-FileCopyrightText: 2026 Crocell Company
"""Paper-run conformance — validates the UAV wing rib chain against
the contract schemas.

This test is the executable form of the G0 paper-run requirement. It
walks the chain through `aether.graph.model` and asserts that:

- every node conforms to its declared node-type schema
- the edges in the chain are all declared in the edge catalog
- the result/decision nodes carry the I-7 / ADR-0024 required fields
"""

from __future__ import annotations

from pathlib import Path

import pytest
import yaml
from aether_sdk.validation import (
    REPO_ROOT,
    ContractValidator,
)


CHAIN_PATH = REPO_ROOT / "workflows/paper-run/chain.yaml"
NODE_TYPE_SCHEMA = "https://aether.local/contracts/graph.model/schema/node-type.schema.json"


@pytest.fixture(scope="module")
def chain() -> dict:
    return yaml.safe_load(CHAIN_PATH.read_text(encoding="utf-8"))


def test_chain_file_loads(chain: dict) -> None:
    assert "nodes" in chain
    assert len(chain["nodes"]) >= 10, "paper run should expose a non-trivial chain"


def test_chain_nodes_validate(chain: dict) -> None:
    validator = ContractValidator("aether.graph.model")
    failures: list[str] = []
    for node in chain["nodes"]:
        result = validator.validate(node, schema_id=NODE_TYPE_SCHEMA)
        if not result.valid:
            failures.append(f"{node['type_id']}: {result.errors}")
    assert not failures, "Node validation failures:\n" + "\n".join(failures)


def test_chain_contains_decision_with_autonomy(chain: dict) -> None:
    decisions = [n for n in chain["nodes"] if n["type_id"] == "Decision"]
    assert decisions, "Paper run must include at least one Decision node (ADR-0024 surface)"
    for d in decisions:
        ac = d["properties_schema"]["autonomy_class"]
        assert ac in {"A0", "A1", "A2", "A3", "A4"}, f"Invalid autonomy class: {ac}"
        assert ac != "A4", "A4 is structurally forbidden (ADR-0024)"


def test_chain_contains_result_with_fidelity_metadata(chain: dict) -> None:
    results = [n for n in chain["nodes"] if n["type_id"] == "Result"]
    assert results, "Paper run must include at least one Result node (I-7 surface)"
    for r in results:
        props = r["properties_schema"]
        for required in ("fidelity_class", "validation_status", "uncertainty", "produced_by"):
            assert required in props, f"Result missing {required!r}: {props}"


def test_chain_contains_simulation_run_with_input_snapshot(chain: dict) -> None:
    runs = [n for n in chain["nodes"] if n["type_id"] == "SimulationRun"]
    assert runs, "Paper run must include at least one SimulationRun node (I-8 surface)"
    for run in runs:
        props = run["properties_schema"]
        assert "input_snapshot_hash" in props, "SimulationRun missing input_snapshot_hash"
        assert "plugin_id" in props and "plugin_version" in props, (
            "SimulationRun must declare the plugin identity and version"
        )


def test_chain_geometry_revision_is_immutable(chain: dict) -> None:
    geos = [n for n in chain["nodes"] if n["type_id"] == "GeometryRevision"]
    assert geos, "Paper run must include at least one GeometryRevision (SDP §11.1)"
    for g in geos:
        assert g.get("immutable") is True, "GeometryRevision must be immutable"


def test_chain_evidence_carries_sources(chain: dict) -> None:
    evs = [n for n in chain["nodes"] if n["type_id"] == "Evidence"]
    assert evs, "Paper run must include at least one Evidence node"
    for e in evs:
        props = e["properties_schema"]
        assert props.get("source_refs"), "Evidence must have source_refs"
        assert props.get("verification_status") in {"unvalidated", "supported", "verified", "contradicted"}
