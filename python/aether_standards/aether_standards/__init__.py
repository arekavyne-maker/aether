# SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
# SPDX-FileCopyrightText: 2026 Crocell Company
"""aether-standards — format adapters with declared semantic loss.

Each adapter reads and writes one interchange format and reports, in a
machine-readable way, what it preserved and what it lost. A format
adapter is an interoperability boundary, not the internal schema: the
platform's canonical model is the Engineering Graph, and adapters
translate to and from it.

Supported adapters:
- STEP AP242 (via OpenCASCADE, when OCP is installed)
- IGES (via OpenCASCADE)
- STL (via OpenCASCADE / meshio)
- HDF5 (via h5py)
- CSV (stdlib)
- JSON (stdlib)

Every adapter declares its `FormatDescriptor`: whether it can read and
write, whether the round trip is lossless, and which standards edition it
targets. `describe()` is the machine-readable contract; `read()` and
`write()` are the operations.
"""

from __future__ import annotations

import csv
import io
import json
import os
from dataclasses import dataclass, field
from typing import Any

try:
    from OCP.STEPControl import STEPControl_AsIs, STEPControl_Reader, STEPControl_Writer
    from OCP.IFSelect import IFSelect_RetDone
    from OCP.IGESControl import IGESControl_Reader, IGESControl_Writer
    from OCP.StlAPI import StlAPI_Writer

    HAVE_OCC = True
except Exception:  # pragma: no cover - environment dependent
    HAVE_OCC = False

try:
    import h5py

    HAVE_H5PY = True
except Exception:  # pragma: no cover
    HAVE_H5PY = False


@dataclass(frozen=True)
class FormatDescriptor:
    """Machine-readable description of an adapter's capabilities."""

    format_id: str
    format_name: str
    version: str
    read: bool
    write: bool
    round_trip_fidelity: str
    standards: tuple[dict[str, str], ...] = ()
    notes: str = ""

    def to_dict(self) -> dict[str, Any]:
        return {
            "format_id": self.format_id,
            "format_name": self.format_name,
            "version": self.version,
            "capabilities": {
                "read": self.read,
                "write": self.write,
                "round_trip_fidelity": self.round_trip_fidelity,
            },
            "standards": [dict(s) for s in self.standards],
            "notes": self.notes,
        }


@dataclass
class SemanticLoss:
    """What a conversion did not preserve."""

    kind: str
    detail: str

    def to_dict(self) -> dict[str, str]:
        return {"kind": self.kind, "detail": self.detail}


@dataclass
class ConversionResult:
    """Result of a read or write, including declared semantic loss."""

    ok: bool
    descriptor: FormatDescriptor
    loss: list[SemanticLoss] = field(default_factory=list)
    data: Any = None
    error: str = ""

    def to_dict(self) -> dict[str, Any]:
        return {
            "ok": self.ok,
            "format": self.descriptor.to_dict(),
            "semantic_loss": [item.to_dict() for item in self.loss],
            "error": self.error or None,
        }


# ---- descriptors ----

STEP = FormatDescriptor(
    format_id="aether.io.format.step-ap242",
    format_name="STEP AP242",
    version="1.0",
    read=HAVE_OCC,
    write=HAVE_OCC,
    round_trip_fidelity="lossy_declared" if HAVE_OCC else "lossy_partial",
    standards=({"name": "ISO 10303-242", "version": "AP242", "edition": "2"},),
    notes=(
        "Exact B-rep and PMI where OpenCASCADE is installed. Tessellated "
        "geometry and some PMI semantics may not survive a round trip."
    ),
)

IGES = FormatDescriptor(
    format_id="aether.io.format.iges",
    format_name="IGES",
    version="1.0",
    read=HAVE_OCC,
    write=HAVE_OCC,
    round_trip_fidelity="lossy_declared",
    standards=({"name": "ANSI Y14.26M", "version": "5.3", "edition": "1"},),
    notes="Legacy exchange; no PMI, no assembly structure.",
)

STL = FormatDescriptor(
    format_id="aether.io.format.stl",
    format_name="STL",
    version="1.0",
    read=True,
    write=HAVE_OCC,
    round_trip_fidelity="lossy_declared",
    standards=({"name": "STL", "version": "1.0", "edition": "1"},),
    notes="Triangulated surface only; no units, no exact geometry.",
)

HDF5 = FormatDescriptor(
    format_id="aether.io.format.hdf5",
    format_name="HDF5",
    version="1.0",
    read=HAVE_H5PY,
    write=HAVE_H5PY,
    round_trip_fidelity="lossless",
    standards=({"name": "HDF5", "version": "1.14", "edition": "1"},),
    notes="Lossless for the numeric arrays it stores; no engineering semantics.",
)

CSV = FormatDescriptor(
    format_id="aether.io.format.csv",
    format_name="CSV",
    version="1.0",
    read=True,
    write=True,
    round_trip_fidelity="lossy_declared",
    standards=({"name": "RFC 4180", "version": "1.0", "edition": "1"},),
    notes="Tabular only; column types are inferred, not declared.",
)

JSON = FormatDescriptor(
    format_id="aether.io.format.json",
    format_name="JSON",
    version="1.0",
    read=True,
    write=True,
    round_trip_fidelity="lossless",
    standards=({"name": "RFC 8259", "version": "1.0", "edition": "1"},),
    notes="Lossless for JSON-representable data.",
)

ALL_FORMATS: dict[str, FormatDescriptor] = {
    "step-ap242": STEP,
    "iges": IGES,
    "stl": STL,
    "hdf5": HDF5,
    "csv": CSV,
    "json": JSON,
}


def describe() -> list[dict[str, Any]]:
    """Machine-readable descriptors for every adapter."""
    return [descriptor.to_dict() for descriptor in ALL_FORMATS.values()]


# ---- STEP / IGES ----

def _read_step(path: str) -> Any:
    if not HAVE_OCC:
        raise RuntimeError("OpenCASCADE bindings unavailable")
    reader = STEPControl_Reader()
    if reader.ReadFile(path) != IFSelect_RetDone:
        raise RuntimeError(f"STEP read failed for {path}")
    reader.TransferRoots()
    shape = reader.OneShape()
    loss = [
        SemanticLoss(
            "pmi",
            "PMI is read where present but not all semantic PMI is preserved",
        ),
        SemanticLoss(
            "assembly",
            "Assembly structure is flattened to a single shape",
        ),
    ]
    return ConversionResult(True, STEP, loss, {"shape": shape})


def _write_step(shape: Any, path: str) -> ConversionResult:
    if not HAVE_OCC:
        raise RuntimeError("OpenCASCADE bindings unavailable")
    writer = STEPControl_Writer()
    writer.Transfer(shape, STEPControl_AsIs)
    writer.Write(path)
    return ConversionResult(True, STEP, [], {"path": path})


def _read_iges(path: str) -> ConversionResult:
    if not HAVE_OCC:
        raise RuntimeError("OpenCASCADE bindings unavailable")
    reader = IGESControl_Reader()
    if reader.ReadFile(path) != IFSelect_RetDone:
        raise RuntimeError(f"IGES read failed for {path}")
    reader.TransferRoots()
    shape = reader.OneShape()
    loss = [
        SemanticLoss("pmi", "IGES carries no PMI"),
        SemanticLoss("units", "IGES unit declarations are not always honoured"),
    ]
    return ConversionResult(True, IGES, loss, {"shape": shape})


def _write_iges(shape: Any, path: str) -> ConversionResult:
    if not HAVE_OCC:
        raise RuntimeError("OpenCASCADE bindings unavailable")
    writer = IGESControl_Writer()
    writer.AddShape(shape)
    writer.Write(path)
    return ConversionResult(True, IGES, [], {"path": path})


# ---- STL ----

def _write_stl(shape: Any, path: str) -> ConversionResult:
    if not HAVE_OCC:
        raise RuntimeError("OpenCASCADE bindings unavailable")
    writer = StlAPI_Writer()
    writer.ASCIIMode = True
    writer.Write(shape, path)
    loss = [
        SemanticLoss("exact_geometry", "triangulated; exact surfaces are lost"),
        SemanticLoss("units", "STL carries no units"),
    ]
    return ConversionResult(True, STL, loss, {"path": path})


# ---- HDF5 ----

def _write_hdf5(arrays: dict[str, Any], path: str) -> ConversionResult:
    if not HAVE_H5PY:
        raise RuntimeError("h5py unavailable")
    with h5py.File(path, "w") as handle:
        for name, value in arrays.items():
            handle.create_dataset(name, data=value)
    return ConversionResult(True, HDF5, [], {"path": path})


def _read_hdf5(path: str) -> ConversionResult:
    if not HAVE_H5PY:
        raise RuntimeError("h5py unavailable")
    out: dict[str, Any] = {}
    with h5py.File(path, "r") as handle:
        handle.visititems(lambda name, obj: out.__setitem__(name, obj[()]))
    return ConversionResult(True, HDF5, [], out)


# ---- CSV ----

def _write_csv(rows: list[dict[str, Any]], path: str) -> ConversionResult:
    if not rows:
        return ConversionResult(True, CSV, [], {"path": path, "rows": 0})
    fieldnames = list(rows[0].keys())
    with open(path, "w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=fieldnames)
        writer.writeheader()
        writer.writerows(rows)
    loss = [SemanticLoss("types", "column types are inferred on read")]
    return ConversionResult(True, CSV, loss, {"path": path, "rows": len(rows)})


def _read_csv(path: str) -> ConversionResult:
    with open(path, newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle))
    return ConversionResult(True, CSV, [], rows)


# ---- JSON ----

def _write_json(data: Any, path: str) -> ConversionResult:
    with open(path, "w", encoding="utf-8") as handle:
        json.dump(data, handle, sort_keys=True)
    return ConversionResult(True, JSON, [], {"path": path})


def _read_json(path: str) -> ConversionResult:
    with open(path, encoding="utf-8") as handle:
        return ConversionResult(True, JSON, [], json.load(handle))


# ---- dispatch ----

def read(fmt: str, path: str) -> ConversionResult:
    if fmt in ("step-ap242", "step"):
        return _read_step(path)
    if fmt == "iges":
        return _read_iges(path)
    if fmt == "hdf5":
        return _read_hdf5(path)
    if fmt == "csv":
        return _read_csv(path)
    if fmt == "json":
        return _read_json(path)
    raise ValueError(f"unsupported format {fmt!r}")


def write(fmt: str, data: Any, path: str) -> ConversionResult:
    if fmt in ("step-ap242", "step"):
        return _write_step(data["shape"], path)
    if fmt == "iges":
        return _write_iges(data["shape"], path)
    if fmt == "stl":
        return _write_stl(data["shape"], path)
    if fmt == "hdf5":
        return _write_hdf5(data, path)
    if fmt == "csv":
        return _write_csv(data, path)
    if fmt == "json":
        return _write_json(data, path)
    raise ValueError(f"unsupported format {fmt!r}")


__all__ = [
    "ALL_FORMATS",
    "CSV",
    "ConversionResult",
    "FormatDescriptor",
    "HDF5",
    "IGES",
    "JSON",
    "STL",
    "STEP",
    "SemanticLoss",
    "describe",
    "read",
    "write",
]