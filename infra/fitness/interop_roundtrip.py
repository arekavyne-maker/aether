# SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
# SPDX-FileCopyrightText: 2026 Crocell Company
"""Interoperability round-trip check (A3-I08, A3-I19).

A format adapter is not conformant merely because it can read a file. This
check exercises each adapter the platform claims to support and asserts:

- the descriptor is present and declares its fidelity honestly;
- a lossless format round-trips exactly;
- a lossy format declares what it lost.

Absent optional dependencies (OpenCASCADE, h5py) are reported as a skip,
never as a silent pass, and never as a failure of the formats that are
available.
"""

from __future__ import annotations

import json
import sys
import tempfile
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
PYTHON_ROOT = REPO_ROOT / "python" / "aether_standards"

LOSSLESS = {"aether.io.format.json", "aether.io.format.hdf5"}


def check(repo_root: Path = REPO_ROOT):
    from run import CheckResult

    sys.path.insert(0, str(PYTHON_ROOT))
    try:
        from aether_standards import describe, read, write
    except Exception as exc:  # pragma: no cover - environment dependent
        return CheckResult(
            name="interop_roundtrip",
            ok=False,
            summary=f"aether_standards is not importable: {exc}",
            details=[],
        )

    descriptors = {d["format_id"]: d for d in describe()}
    problems: list[str] = []
    checked: list[str] = []
    skipped: list[str] = []

    with tempfile.TemporaryDirectory() as tmp:
        tmp_path = Path(tmp)

        # JSON: always available, must be lossless.
        json_path = tmp_path / "x.json"
        payload = {"a": 1, "b": [1, 2, 3]}
        write("json", payload, str(json_path))
        back = read("json", str(json_path))
        if back.data != payload:
            problems.append("json round trip changed the payload")
        else:
            checked.append("json:lossless")

        # CSV: always available, must declare type loss.
        csv_path = tmp_path / "x.csv"
        result = write("csv", [{"a": "1"}], str(csv_path))
        if not any(loss.kind == "types" for loss in result.loss):
            problems.append("csv did not declare its type loss")
        else:
            checked.append("csv:lossy_declared")

        # HDF5: optional.
        try:
            import h5py  # noqa: F401

            h5_path = tmp_path / "x.h5"
            write("hdf5", {"t": [1.0, 2.0]}, str(h5_path))
            back = read("hdf5", str(h5_path))
            if list(back.data["t"]) != [1.0, 2.0]:
                problems.append("hdf5 round trip changed the data")
            else:
                checked.append("hdf5:lossless")
        except ImportError:
            skipped.append("hdf5 (h5py not installed)")

        # STEP: optional, must preserve volume and declare loss.
        try:
            import OCP  # noqa: F401

            from OCP.BRepPrimAPI import BRepPrimAPI_MakeBox
            from OCP.GProp import GProp_GProps
            from OCP.BRepGProp import BRepGProp

            step_path = tmp_path / "x.step"
            shape = BRepPrimAPI_MakeBox(1.0, 2.0, 3.0).Shape()
            # OCP writes a banner to fd 1; silence it for the check output.
            import contextlib
            import os

            with open(os.devnull, "w") as devnull, contextlib.redirect_stdout(devnull):
                sys.stdout.flush()
                saved = os.dup(1)
                os.dup2(devnull.fileno(), 1)
                try:
                    write("step-ap242", {"shape": shape}, str(step_path))
                finally:
                    os.dup2(saved, 1)
                    os.close(saved)
            back = read("step-ap242", str(step_path))
            props = GProp_GProps()
            BRepGProp.VolumeProperties_s(back.data["shape"], props)
            if abs(props.Mass() - 6.0) > 1e-6:
                problems.append(f"step volume changed: {props.Mass()}")
            elif not back.loss:
                problems.append("step did not declare semantic loss")
            else:
                checked.append("step-ap242:lossy_declared")
        except ImportError:
            skipped.append("step-ap242 (OCP not installed)")

    # Every descriptor must declare a fidelity the checker understands.
    for format_id, descriptor in descriptors.items():
        fidelity = descriptor["capabilities"]["round_trip_fidelity"]
        if fidelity not in ("lossless", "lossy_declared", "lossy_partial"):
            problems.append(f"{format_id}: unknown fidelity {fidelity!r}")

    if problems:
        return CheckResult(
            name="interop_roundtrip",
            ok=False,
            summary=f"{len(problems)} interoperability problem(s)",
            details=problems,
        )

    summary = f"{len(checked)} format(s) round-tripped"
    if skipped:
        summary += f"; {len(skipped)} skipped"
    return CheckResult(
        name="interop_roundtrip",
        ok=True,
        skipped=bool(skipped) and not checked,
        summary=summary,
        details=checked + [f"skip: {s}" for s in skipped],
    )


if __name__ == "__main__":
    result = check()
    print(result.summary)
    for detail in result.details:
        print(" ", detail)
    raise SystemExit(0 if result.ok or result.skipped else 1)