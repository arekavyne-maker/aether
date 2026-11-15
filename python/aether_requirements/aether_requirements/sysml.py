# SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
# SPDX-FileCopyrightText: 2026 Crocell Company
"""Aether requirements package — SysML v2 textual subset + verification coverage.

A `SysMLFragment` is a JSON/YAML representation of a subset of the
SysML v2 textual notation sufficient to round-trip Aether
requirements. The subset covers:

- `requirement` — text, criticality, verification_method, optional
  subject_ref (the requirement this one specializes or depends on).
- `verification` — a verification case; references the requirement(s)
  it verifies via `verify_ref`.
- `actor`, `use_case`, `concern`, `stakeholder` — minimal context
  (the full SysML v2 surface is much larger; G3 ships a faithful
  subset and G3.5 expands to the full KerML/SysML model).

The `verify_report()` method walks the fragment and produces a
`VerificationReport`: the total requirement count, the count of
requirements that have at least one `verifies` path, and the gap list
(requirements that no verification case covers). This is the
executable form of SDP §3.6 invariant 7.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Iterable

import yaml

_KIND_VALUES = {
    "requirement", "verification", "actor", "use_case", "concern", "stakeholder",
}
_VERIFICATION_METHODS = {"analysis", "test", "inspection", "demonstration", "similarity"}
_CRITICALITY = {"catastrophic", "hazardous", "major", "minor", "no_effect"}


@dataclass
class FragmentElement:
    kind: str
    id: str
    name: str | None = None
    text: str | None = None
    verification_method: str | None = None
    criticality: str | None = None
    subject_ref: str | None = None
    verify_ref: str | None = None

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> "FragmentElement":
        if data.get("kind") not in _KIND_VALUES:
            raise ValueError(f"unknown element kind: {data.get('kind')!r}")
        if not data.get("id"):
            raise ValueError("element missing id")
        vm = data.get("verification_method")
        if vm is not None and vm not in _VERIFICATION_METHODS:
            raise ValueError(f"unknown verification_method: {vm!r}")
        crit = data.get("criticality")
        if crit is not None and crit not in _CRITICALITY:
            raise ValueError(f"unknown criticality: {crit!r}")
        return cls(
            kind=data["kind"],
            id=data["id"],
            name=data.get("name"),
            text=data.get("text"),
            verification_method=vm,
            criticality=crit,
            subject_ref=data.get("subject_ref"),
            verify_ref=data.get("verify_ref"),
        )


@dataclass
class SysMLFragment:
    package: str
    elements: list[FragmentElement]

    @classmethod
    def from_yaml(cls, path: str | Path) -> "SysMLFragment":
        with open(path, "r", encoding="utf-8") as f:
            raw = yaml.safe_load(f)
        return cls.from_dict(raw)

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> "SysMLFragment":
        # Strip the harness-only _schema_id field.
        if isinstance(data, dict) and "_schema_id" in data:
            data = {k: v for k, v in data.items() if k != "_schema_id"}
        if "package" not in data or "elements" not in data:
            raise ValueError("fragment missing required fields 'package' or 'elements'")
        elements = [FragmentElement.from_dict(e) for e in data["elements"]]
        return cls(package=data["package"], elements=elements)

    def requirements(self) -> list[FragmentElement]:
        return [e for e in self.elements if e.kind == "requirement"]

    def verifications(self) -> list[FragmentElement]:
        return [e for e in self.elements if e.kind == "verification"]

    def verify_report(self) -> "VerificationReport":
        requirements = self.requirements()
        verifications = self.verifications()
        verified_ids = {v.verify_ref for v in verifications if v.verify_ref}
        gaps: list[VerificationGap] = []
        for req in requirements:
            if req.id not in verified_ids:
                gaps.append(
                    VerificationGap(
                        requirement_id=req.id,
                        reason="no verification case references this requirement",
                        verification_method=req.verification_method,
                    )
                )
        return VerificationReport(
            requirements_total=len(requirements),
            requirements_verified=sum(1 for r in requirements if r.id in verified_ids),
            gaps=gaps,
        )

    def to_yaml(self) -> str:
        data = {
            "package": self.package,
            "elements": [
                {
                    "kind": e.kind,
                    "id": e.id,
                    "name": e.name,
                    "text": e.text,
                    "verification_method": e.verification_method,
                    "criticality": e.criticality,
                    "subject_ref": e.subject_ref,
                    "verify_ref": e.verify_ref,
                }
                for e in self.elements
            ],
        }
        # Strip None values for cleaner output.
        for e_dict in data["elements"]:
            for k in list(e_dict.keys()):
                if e_dict[k] is None:
                    del e_dict[k]
        return yaml.safe_dump(data, sort_keys=True)


@dataclass
class VerificationGap:
    requirement_id: str
    reason: str
    verification_method: str | None = None


@dataclass
class VerificationReport:
    requirements_total: int
    requirements_verified: int
    gaps: list[VerificationGap] = field(default_factory=list)


__all__ = [
    "FragmentElement",
    "SysMLFragment",
    "VerificationGap",
    "VerificationReport",
]
