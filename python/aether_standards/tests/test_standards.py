# SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
# SPDX-FileCopyrightText: 2026 Crocell Company
"""Conformance tests for the interchange-format adapters."""

from __future__ import annotations

import json
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from aether_standards import (  # noqa: E402
    CSV,
    HDF5,
    JSON,
    STEP,
    ConversionResult,
    describe,
    read,
    write,
)


def test_describe_covers_every_format() -> None:
    descriptors = describe()
    ids = {d["format_id"] for d in descriptors}
    assert "aether.io.format.step-ap242" in ids
    assert "aether.io.format.hdf5" in ids
    assert "aether.io.format.csv" in ids
    assert "aether.io.format.json" in ids
    for descriptor in descriptors:
        assert descriptor["capabilities"]["round_trip_fidelity"] in {
            "lossless",
            "lossy_declared",
            "lossy_partial",
        }


def test_json_round_trip_is_lossless(tmp_path: Path) -> None:
    payload = {"a": 1, "b": [1, 2, 3], "c": {"d": "e"}}
    path = tmp_path / "data.json"
    result = write("json", payload, str(path))
    assert result.ok
    assert result.descriptor.round_trip_fidelity == "lossless"
    back = read("json", str(path))
    assert back.data == payload


def test_csv_round_trip_preserves_rows(tmp_path: Path) -> None:
    rows = [{"x": "1", "y": "2"}, {"x": "3", "y": "4"}]
    path = tmp_path / "data.csv"
    result = write("csv", rows, str(path))
    assert result.ok
    assert any(loss.kind == "types" for loss in result.loss)
    back = read("csv", str(path))
    assert back.data == rows


def test_hdf5_round_trip_is_lossless(tmp_path: Path) -> None:
    pytest.importorskip("h5py")
    arrays = {"temperature": [1.0, 2.0, 3.0], "pressure": [4.0, 5.0, 6.0]}
    path = tmp_path / "field.h5"
    result = write("hdf5", arrays, str(path))
    assert result.ok
    back = read("hdf5", str(path))
    assert list(back.data["temperature"]) == [1.0, 2.0, 3.0]


def test_step_round_trip_preserves_volume(tmp_path: Path) -> None:
    pytest.importorskip("OCP")
    from OCP.BRepPrimAPI import BRepPrimAPI_MakeBox

    shape = BRepPrimAPI_MakeBox(10.0, 20.0, 30.0).Shape()
    path = tmp_path / "box.step"
    result = write("step-ap242", {"shape": shape}, str(path))
    assert result.ok
    assert path.is_file()

    back = read("step-ap242", str(path))
    assert back.ok
    assert any(loss.kind in ("pmi", "assembly") for loss in back.loss)
    from OCP.GProp import GProp_GProps
    from OCP.BRepGProp import BRepGProp

    props = GProp_GProps()
    BRepGProp.VolumeProperties_s(back.data["shape"], props)
    assert abs(props.Mass() - 6000.0) < 1e-6


def test_unsupported_format_is_rejected(tmp_path: Path) -> None:
    with pytest.raises(ValueError):
        read("docx", str(tmp_path / "x.docx"))


def test_descriptors_are_json_serialisable() -> None:
    json.dumps(describe())