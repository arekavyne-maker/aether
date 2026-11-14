# SPDX-License-Identifier: Aether-VPSL-1.0
# SPDX-FileCopyrightText: 2026 The Aether Consortium
"""Fixture-label sentinel (alignment pass, I-7 / honesty discipline).

The Phase 0–2 reference plugin set contains several deliberately
fake components (bounding-box geometry, hash-stub solvers). The
codebase's culture of honest labeling (every fake component says so
in its docstring) is itself a load-bearing invariant. This check
enforces the *machine-readable* part of that discipline: it
spawns the geometry reference plugin, calls ``discover()``, and
asserts the response carries ``is_real_brep: False`` with a
``note`` field that explicitly states the fixture nature.

If a future edit drops the honest label, this check fails the
build — preventing the codebase from regressing into a state
where a bounding-box fixture is silently presented as a real
B-rep kernel.

A regression here is treated as a Phase 0 acceptance violation:
the project cannot claim Phase 0–2 substrate-completeness while
deliberately mislabeling a reference implementation.
"""

from __future__ import annotations

import json
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path


# A path-relative spawn of the geometry reference plugin. The
# plugin implements JSON-RPC 2.0 over stdio (ADR-0015); we send
# one ``discover`` request and parse the response.
GEOM_PLUGIN = "reference_plugins/occ-geometry/python/stdio_plugin.py"
# The literal text that MUST appear in the discover() response's
# ``note`` field. We assert the substring (not equality) so the
# honest label can be reworded slightly without breaking the
# check, but the substantive claim cannot be silently dropped.
REQUIRED_NOTE_FRAGMENT = "not real B-rep"
REQUIRED_FLAG = "is_real_brep"
REQUIRED_FLAG_VALUE = False


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
            timeout=15,
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


def check(repo_root: Path) -> _Result:  # type: ignore[name-defined]
    from run import CheckResult  # local import; avoid circular at module import

    plugin_path = repo_root / GEOM_PLUGIN
    if not plugin_path.is_file():
        return CheckResult(
            name="geom_fixture_label",
            ok=False,
            summary=f"geometry fixture not found at {plugin_path}",
            details=[],
        )

    response = _spawn_discover(plugin_path)
    if response is None:
        return CheckResult(
            name="geom_fixture_label",
            ok=False,
            summary="geometry fixture did not return a discover() response",
            details=[f"plugin path: {plugin_path}"],
        )
    if "error" in response:
        return CheckResult(
            name="geom_fixture_label",
            ok=False,
            summary=f"geometry fixture discover() returned an error: {response['error']!r}",
            details=[],
        )
    result = response.get("result", {})

    issues: list[str] = []
    if REQUIRED_FLAG not in result:
        issues.append(
            f"missing required field {REQUIRED_FLAG!r} in discover() result"
        )
    elif result[REQUIRED_FLAG] is not REQUIRED_FLAG_VALUE:
        issues.append(
            f"{REQUIRED_FLAG} must be {REQUIRED_FLAG_VALUE!r}; got {result[REQUIRED_FLAG]!r}"
        )
    note = result.get("note", "")
    if not isinstance(note, str) or REQUIRED_NOTE_FRAGMENT not in note:
        issues.append(
            f"note must contain {REQUIRED_NOTE_FRAGMENT!r}; got {note!r}"
        )

    if issues:
        return CheckResult(
            name="geom_fixture_label",
            ok=False,
            summary="geometry fixture is mislabeled (must declare is_real_brep=False and a 'not real B-rep' note)",
            details=issues + [
                "fix: update reference_plugins/occ-geometry/python/stdio_plugin.py:method_discover()"
            ],
        )

    return CheckResult(
        name="geom_fixture_label",
        ok=True,
        summary=(
            f"geometry fixture honest: is_real_brep={result[REQUIRED_FLAG]}, "
            f"note fragment '{REQUIRED_NOTE_FRAGMENT}' present"
        ),
        details=[],
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
    print(f"[{'OK ' if r.ok else 'FAIL'}] {r.name}: {r.summary}")
    for d in r.details:
        print(f"       {d}")
    sys.exit(0 if r.ok else 1)
