# SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
# SPDX-FileCopyrightText: 2026 Crocell Company
"""Geometry honesty check (I-7 / A3-I22).

The geometry plugin must report whether it is backed by a real B-rep
kernel, and the label must match what is actually importable. Two
outcomes are acceptable:

- real OpenCASCADE bindings present -> ``is_real_brep: true`` and the
  discover() response must name the kernel and its version;
- bindings absent -> ``is_real_brep: false`` with a note explaining the
  fixture nature, and the build must not claim real geometry.

A mismatch between the label and the environment is a build failure: the
platform must never present a fixture as a real kernel, and it must also
never deny real capability it actually has.
"""

from __future__ import annotations

import json
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

GEOM_PLUGIN = "reference_plugins/occ-geometry/python/stdio_plugin.py"
REQUIRED_NOTE_FRAGMENT = "not real B-rep"


@dataclass
class _Result:  # mirrors run.CheckResult shape; declared locally to avoid an import cycle
    name: str
    ok: bool
    summary: str
    details: list[str]


def _spawn_discover(plugin_path: Path) -> dict | None:
    request = {"jsonrpc": "2.0", "id": "fitness-geom-discover", "method": "discover"}
    try:
        proc = subprocess.run(
            ["python3", str(plugin_path)],
            input=json.dumps(request) + "\n",
            capture_output=True,
            text=True,
            timeout=30,
            check=False,
        )
    except (OSError, subprocess.TimeoutExpired) as exc:
        print(f"geom_fixture_label: spawn failed: {exc!r}", file=sys.stderr)
        return None
    for raw in (proc.stdout or "").splitlines():
        raw = raw.strip()
        if not raw:
            continue
        try:
            response = json.loads(raw)
        except json.JSONDecodeError:
            continue
        if response.get("id") == "fitness-geom-discover":
            return response
    return None


def _bindings_available() -> bool:
    try:
        import OCP  # noqa: F401

        return True
    except Exception:
        return False


def check(repo_root: Path) -> _Result:  # type: ignore[name-defined]
    from run import CheckResult  # local import; avoid circular at module import

    plugin_path = repo_root / GEOM_PLUGIN
    if not plugin_path.is_file():
        return CheckResult(
            name="geom_fixture_label",
            ok=False,
            summary=f"geometry plugin not found at {plugin_path}",
            details=[],
        )

    response = _spawn_discover(plugin_path)
    if response is None:
        return CheckResult(
            name="geom_fixture_label",
            ok=False,
            summary="geometry plugin did not return a discover() response",
            details=[f"plugin path: {plugin_path}"],
        )
    if "error" in response:
        return CheckResult(
            name="geom_fixture_label",
            ok=False,
            summary=f"geometry discover() returned an error: {response['error']!r}",
            details=[],
        )

    result = response.get("result", {})
    available = _bindings_available()
    declared = result.get("is_real_brep")
    kernel = result.get("geometry_kernel", "")
    kernel_version = result.get("geometry_kernel_version", "")
    note = result.get("note", "")

    issues: list[str] = []

    if "is_real_brep" not in result:
        issues.append("discover() result is missing the is_real_brep field")

    if available:
        # Real bindings: the label must say so and name the kernel.
        if declared is not True:
            issues.append(
                f"OCP bindings are importable but is_real_brep={declared!r}; "
                "the label must be true when the real kernel is present"
            )
        if not kernel:
            issues.append("real kernel present but geometry_kernel is empty")
        if kernel_version in ("", "unavailable"):
            issues.append(
                "real kernel present but geometry_kernel_version does not resolve"
            )
    else:
        # No bindings: the plugin must be honest about being a fixture.
        if declared is not False:
            issues.append(
                f"OCP bindings are absent but is_real_brep={declared!r}; "
                "a fixture must declare itself as not real"
            )
        if REQUIRED_NOTE_FRAGMENT not in note:
            issues.append(
                f"note must contain {REQUIRED_NOTE_FRAGMENT!r} when the fixture "
                f"is active; got {note!r}"
            )

    if issues:
        return CheckResult(
            name="geom_fixture_label",
            ok=False,
            summary="geometry label does not match the installed capability",
            details=issues
            + [
                "fix: install cadquery-ocp==7.7.2 for real B-rep, or keep the "
                "fixture honestly labelled"
            ],
        )

    if available:
        return CheckResult(
            name="geom_fixture_label",
            ok=True,
            summary=f"real B-rep active: {kernel} {kernel_version}",
            details=["OpenCASCADE bindings importable and reported truthfully"],
        )

    return CheckResult(
        name="geom_fixture_label",
        ok=False,
        skipped=True,
        summary="OpenCASCADE bindings absent; geometry is a labelled fixture",
        details=["install cadquery-ocp==7.7.2 for real B-rep geometry"],
    )


if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument(
        "--repo-root",
        type=Path,
        default=Path(__file__).resolve().parents[2],
    )
    args = parser.parse_args()
    r = check(args.repo_root)
    print(f"[{'SKIP' if getattr(r, 'skipped', False) else ('OK ' if r.ok else 'FAIL')}] {r.name}: {r.summary}")
    for d in r.details:
        print(f"       {d}")
    sys.exit(0 if r.ok or getattr(r, "skipped", False) else 1)