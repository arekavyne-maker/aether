# SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
# SPDX-FileCopyrightText: 2026 Crocell Company
"""Conformance tests for the QIF inspection-plan adapter."""

from __future__ import annotations

import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from aether_standards.qif import (  # noqa: E402
    Characteristic,
    InspectionPlan,
    read_plan,
    write_plan,
)


def _plan() -> InspectionPlan:
    return InspectionPlan(
        part_id="rib-07",
        revision=2,
        characteristics=[
            Characteristic("pocket_depth", 3.0, 3.1, 2.9, "mm"),
            Characteristic("skin_thickness", 2.4, 2.5, 2.3, "mm"),
        ],
        resources=["CMM-001"],
    )


def test_round_trip_preserves_characteristics(tmp_path: Path) -> None:
    path = tmp_path / "plan.qif"
    problems = write_plan(_plan(), str(path))
    assert problems == []
    back = read_plan(str(path))
    assert back.part_id == "rib-07"
    assert back.revision == 2
    assert len(back.characteristics) == 2
    assert back.characteristics[0].name == "pocket_depth"
    assert back.characteristics[0].nominal == 3.0
    assert back.resources == ["CMM-001"]


def test_round_trip_declares_semantic_loss(tmp_path: Path) -> None:
    path = tmp_path / "plan.qif"
    write_plan(_plan(), str(path))
    back = read_plan(str(path))
    assert back.semantic_loss


def test_characteristic_accepts_within_limits() -> None:
    characteristic = Characteristic("pocket_depth", 3.0, 3.1, 2.9)
    assert characteristic.accepts(3.0)
    assert characteristic.accepts(2.9)
    assert not characteristic.accepts(2.85)


def test_invalid_plan_is_refused(tmp_path: Path) -> None:
    plan = InspectionPlan(
        part_id="rib-07",
        revision=1,
        characteristics=[Characteristic("bad", 3.0, 2.0, 4.0)],
    )
    problems = write_plan(plan, str(tmp_path / "bad.qif"))
    assert problems
    assert not (tmp_path / "bad.qif").exists()


def test_nominal_outside_limits_is_refused(tmp_path: Path) -> None:
    plan = InspectionPlan(
        part_id="rib-07",
        revision=1,
        characteristics=[Characteristic("bad", 5.0, 3.1, 2.9)],
    )
    assert write_plan(plan, str(tmp_path / "bad.qif"))