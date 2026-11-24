# SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
# SPDX-FileCopyrightText: 2026 Crocell Company
"""QIF (Quality Information Framework) inspection-plan adapter.

QIF is an XML standard for metrology. This adapter reads and writes a
small, well-defined subset sufficient for an inspection plan: part
identity, characteristics with nominal and tolerance, and the measurement
resources. It declares its semantic loss explicitly: QIF's full
measurement-result and statistics model is not implemented.

The adapter is deliberately conservative. It refuses to write a plan that
declares a characteristic without a nominal and a tolerance, because an
inspection plan that cannot be judged is not an inspection plan.
"""

from __future__ import annotations

import xml.etree.ElementTree as ET
from dataclasses import dataclass, field
from typing import Any

QIF_NAMESPACE = "http://qifstandards.org/QIF2"


@dataclass(frozen=True)
class Characteristic:
    """One measurable characteristic with its acceptance limits."""

    name: str
    nominal: float
    upper_limit: float
    lower_limit: float
    unit: str = "mm"

    def to_dict(self) -> dict[str, Any]:
        return {
            "name": self.name,
            "nominal": self.nominal,
            "upper_limit": self.upper_limit,
            "lower_limit": self.lower_limit,
            "unit": self.unit,
        }

    def accepts(self, measured: float) -> bool:
        return self.lower_limit <= measured <= self.upper_limit


@dataclass
class InspectionPlan:
    """A QIF-subset inspection plan."""

    part_id: str
    revision: int
    characteristics: list[Characteristic] = field(default_factory=list)
    resources: list[str] = field(default_factory=list)
    semantic_loss: list[str] = field(default_factory=list)

    def validate(self) -> list[str]:
        problems: list[str] = []
        if not self.part_id.strip():
            problems.append("part_id is empty")
        if self.revision < 1:
            problems.append("revision must be >= 1")
        for characteristic in self.characteristics:
            if not characteristic.name.strip():
                problems.append("characteristic name is empty")
            if characteristic.lower_limit > characteristic.upper_limit:
                problems.append(
                    f"{characteristic.name}: lower limit exceeds upper limit"
                )
            if not (
                characteristic.lower_limit
                <= characteristic.nominal
                <= characteristic.upper_limit
            ):
                problems.append(
                    f"{characteristic.name}: nominal is outside the limits"
                )
        return problems

    def to_dict(self) -> dict[str, Any]:
        return {
            "part_id": self.part_id,
            "revision": self.revision,
            "characteristics": [c.to_dict() for c in self.characteristics],
            "resources": list(self.resources),
            "semantic_loss": list(self.semantic_loss),
        }


def _q(tag: str) -> str:
    return f"{{{QIF_NAMESPACE}}}{tag}"


def write_plan(plan: InspectionPlan, path: str) -> list[str]:
    """Write an inspection plan as QIF XML. Returns validation problems."""
    problems = plan.validate()
    if problems:
        return problems

    root = ET.Element(_q("QIFDocument"), {"version": "2.1"})
    header = ET.SubElement(root, _q("Header"))
    ET.SubElement(header, _q("FileVersion")).text = "1.0"

    plan_el = ET.SubElement(root, _q("InspectionPlan"))
    ET.SubElement(plan_el, _q("PartId")).text = plan.part_id
    ET.SubElement(plan_el, _q("Revision")).text = str(plan.revision)

    characteristics = ET.SubElement(plan_el, _q("Characteristics"))
    for characteristic in plan.characteristics:
        item = ET.SubElement(characteristics, _q("Characteristic"))
        ET.SubElement(item, _q("Name")).text = characteristic.name
        nominal = ET.SubElement(item, _q("Nominal"))
        ET.SubElement(nominal, _q("Value")).text = repr(characteristic.nominal)
        ET.SubElement(nominal, _q("Unit")).text = characteristic.unit
        limits = ET.SubElement(item, _q("Limits"))
        ET.SubElement(limits, _q("Lower")).text = repr(characteristic.lower_limit)
        ET.SubElement(limits, _q("Upper")).text = repr(characteristic.upper_limit)

    if plan.resources:
        resources = ET.SubElement(plan_el, _q("Resources"))
        for resource in plan.resources:
            ET.SubElement(resources, _q("Resource")).text = resource

    ET.ElementTree(root).write(path, encoding="utf-8", xml_declaration=True)
    return []


def read_plan(path: str) -> InspectionPlan:
    """Read a QIF-subset inspection plan."""
    tree = ET.parse(path)
    root = tree.getroot()
    plan_el = root.find(_q("InspectionPlan"))
    if plan_el is None:
        raise ValueError("document has no InspectionPlan element")

    part_id = (plan_el.findtext(_q("PartId")) or "").strip()
    revision_text = plan_el.findtext(_q("Revision")) or "1"
    try:
        revision = int(revision_text)
    except ValueError as exc:
        raise ValueError(f"invalid revision {revision_text!r}") from exc

    characteristics: list[Characteristic] = []
    characteristics_el = plan_el.find(_q("Characteristics"))
    if characteristics_el is not None:
        for item in characteristics_el.findall(_q("Characteristic")):
            name = (item.findtext(_q("Name")) or "").strip()
            nominal_el = item.find(_q("Nominal"))
            if nominal_el is None:
                raise ValueError(f"characteristic {name!r} has no nominal")
            nominal = float(nominal_el.findtext(_q("Value")) or "nan")
            unit = nominal_el.findtext(_q("Unit")) or "mm"
            limits = item.find(_q("Limits"))
            if limits is None:
                raise ValueError(f"characteristic {name!r} has no limits")
            lower = float(limits.findtext(_q("Lower")) or "nan")
            upper = float(limits.findtext(_q("Upper")) or "nan")
            characteristics.append(
                Characteristic(name, nominal, upper, lower, unit)
            )

    resources: list[str] = []
    resources_el = plan_el.find(_q("Resources"))
    if resources_el is not None:
        resources = [
            (resource.text or "").strip()
            for resource in resources_el.findall(_q("Resource"))
        ]

    return InspectionPlan(
        part_id=part_id,
        revision=revision,
        characteristics=characteristics,
        resources=resources,
        semantic_loss=[
            "measurement results are not part of this subset",
            "statistical evaluation is not implemented",
        ],
    )


__all__ = ["Characteristic", "InspectionPlan", "read_plan", "write_plan"]