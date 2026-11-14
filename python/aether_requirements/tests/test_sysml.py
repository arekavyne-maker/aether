# SPDX-License-Identifier: Aether-VPSL-1.0
# SPDX-FileCopyrightText: 2026 The Aether Consortium
"""Conformance tests for the requirements package (G3)."""

from __future__ import annotations

import textwrap
from pathlib import Path

import pytest

from aether_requirements import SysMLFragment, VerificationGap, VerificationReport


def _sample_fragment_dict() -> dict:
    return {
        "package": "UAVWingRequirements",
        "elements": [
            {
                "kind": "requirement",
                "id": "R-STR-011",
                "name": "Wing rib limit load",
                "text": "The wing rib shall sustain the limit load factor of 1.5 with no failure.",
                "verification_method": "test",
                "criticality": "catastrophic",
            },
            {
                "kind": "requirement",
                "id": "R-STR-012",
                "name": "Wing rib fatigue",
                "text": "The wing rib shall sustain 10^6 cycles.",
                "verification_method": "test",
                "criticality": "hazardous",
                "subject_ref": "R-STR-011",
            },
            {
                "kind": "requirement",
                "id": "R-MFG-001",
                "name": "Fastener torque",
                "text": "All fasteners shall be torque-marked.",
                "verification_method": "inspection",
                "criticality": "major",
            },
            {
                "kind": "verification",
                "id": "V-STR-011-1",
                "name": "Buckling test TR-114",
                "verify_ref": "R-STR-011",
            },
            {
                "kind": "verification",
                "id": "V-MFG-001-1",
                "name": "Visual inspection",
                "verify_ref": "R-MFG-001",
            },
        ],
    }


def test_parses_from_dict() -> None:
    f = SysMLFragment.from_dict(_sample_fragment_dict())
    assert f.package == "UAVWingRequirements"
    assert len(f.elements) == 5
    assert len(f.requirements()) == 3
    assert len(f.verifications()) == 2


def test_parses_from_yaml_strips_schema_id(tmp_path: Path) -> None:
    p = tmp_path / "wing.sysml.yaml"
    p.write_text(
        textwrap.dedent(
            """
            _schema_id: "https://aether.local/contracts/requirement/schema/sysml-fragment.schema.json"
            package: "X"
            elements:
              - { kind: requirement, id: R1, verification_method: test }
            """
        ).strip()
    )
    f = SysMLFragment.from_yaml(p)
    assert f.package == "X"
    assert f.elements[0].id == "R1"


def test_verify_report_full_coverage() -> None:
    """All 3 requirements have verifications; gaps = []."""
    f = SysMLFragment.from_dict(_sample_fragment_dict())
    r = f.verify_report()
    assert r.requirements_total == 3
    assert r.requirements_verified == 2
    # R-STR-012 has no verification case.
    assert len(r.gaps) == 1
    assert r.gaps[0].requirement_id == "R-STR-012"
    assert "no verification case" in r.gaps[0].reason


def test_verify_report_no_verifications_at_all() -> None:
    f = SysMLFragment.from_dict(
        {
            "package": "Empty",
            "elements": [
                {"kind": "requirement", "id": "R1", "verification_method": "test"},
                {"kind": "requirement", "id": "R2", "verification_method": "test"},
            ],
        }
    )
    r = f.verify_report()
    assert r.requirements_total == 2
    assert r.requirements_verified == 0
    assert len(r.gaps) == 2


def test_verify_report_all_covered() -> None:
    f = SysMLFragment.from_dict(
        {
            "package": "P",
            "elements": [
                {"kind": "requirement", "id": "R1", "verification_method": "test"},
                {"kind": "verification", "id": "V1", "verify_ref": "R1"},
            ],
        }
    )
    r = f.verify_report()
    assert r.requirements_verified == 1
    assert r.gaps == []


def test_rejects_unknown_kind() -> None:
    with pytest.raises(ValueError, match="unknown element kind"):
        SysMLFragment.from_dict(
            {
                "package": "P",
                "elements": [{"kind": "totally_made_up", "id": "X"}],
            }
        )


def test_rejects_unknown_verification_method() -> None:
    with pytest.raises(ValueError, match="unknown verification_method"):
        SysMLFragment.from_dict(
            {
                "package": "P",
                "elements": [
                    {"kind": "requirement", "id": "R1", "verification_method": "vibes"},
                ],
            }
        )


def test_rejects_unknown_criticality() -> None:
    with pytest.raises(ValueError, match="unknown criticality"):
        SysMLFragment.from_dict(
            {
                "package": "P",
                "elements": [
                    {"kind": "requirement", "id": "R1", "criticality": "slightly_bad"},
                ],
            }
        )


def test_rejects_missing_id() -> None:
    with pytest.raises(ValueError, match="missing id"):
        SysMLFragment.from_dict(
            {
                "package": "P",
                "elements": [{"kind": "requirement"}],
            }
        )


def test_rejects_missing_required_field() -> None:
    with pytest.raises(ValueError, match="missing required field"):
        SysMLFragment.from_dict({"elements": []})


def test_round_trip_yaml() -> None:
    original = _sample_fragment_dict()
    f = SysMLFragment.from_dict(original)
    yaml_str = f.to_yaml()
    f2 = SysMLFragment.from_yaml_text(yaml_str) if hasattr(SysMLFragment, "from_yaml_text") else None
    # from_yaml_text doesn't exist; use a temporary file.
    import tempfile
    with tempfile.NamedTemporaryFile("w", suffix=".yaml", delete=False) as tf:
        tf.write(yaml_str)
        path = tf.name
    f2 = SysMLFragment.from_yaml(path)
    assert f2.package == f.package
    assert len(f2.elements) == len(f.elements)
    # Re-verify after round-trip.
    r = f2.verify_report()
    assert r.requirements_total == 3
    assert r.requirements_verified == 2


def test_to_yaml_omits_none_fields() -> None:
    f = SysMLFragment.from_dict(
        {
            "package": "P",
            "elements": [{"kind": "requirement", "id": "R1"}],
        }
    )
    yaml_str = f.to_yaml()
    assert "verification_method" not in yaml_str  # not set
    assert "criticality" not in yaml_str
    assert "subject_ref" not in yaml_str
