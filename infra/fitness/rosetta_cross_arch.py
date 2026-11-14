# SPDX-License-Identifier: Aether-VPSL-1.0
# SPDX-FileCopyrightText: 2026 The Aether Consortium
"""Rosetta cross-architecture equivalence check (alignment pass).

MVR-04/05/12 (SDP §20) require cross-hardware equivalence: a workflow
that runs on architecture A must produce the same result bundle
(same ``cas_hash``) on architecture B, where "same result" is
defined by the SDP's determinism class (bit-reproducible for
analytical / closed-form solvers; documented-equivalent for
numerical solvers).

This machine is aarch64-apple-darwin. Apple Silicon's Rosetta 2
provides user-mode x86_64 emulation. We use it to run the same
``cfd-reference-minimal`` plugin (analytical Poiseuille flow)
twice — once natively on aarch64, once under Rosetta on
x86_64 — and assert the result ``cas_hash`` is identical.

HONEST LABEL
------------
Rosetta is dynamic binary translation, not a separate physical
architecture. A genuine cross-architecture MVR-04 acceptance test
requires either a real second physical machine or a QEMU
user-mode emulator (not present in this environment). This
fitness check exercises the *equivalence check itself* (the
cas_hash comparison logic that production MVR-04 sites run) and
labels its scope as ``cross_arch_emulation``, not
``cross_architecture``. The strong claim (MVR-04 against a
second physical architecture) is recorded as an open gap in
``workflows/paper-run/gaps.md``.

If Rosetta is not available (``arch -x86_64 <command>`` fails),
the check skips with an explicit message and AP-16 stays open
unmodified.
"""

from __future__ import annotations

import json
import shutil
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

REPO_ROOT_DEFAULT = Path(__file__).resolve().parents[2]
PLUGIN = REPO_ROOT_DEFAULT / "reference_plugins" / "cfd-reference-minimal" / "python" / "stdio_plugin.py"


@dataclass
class _Result:
    name: str
    ok: bool
    summary: str
    details: list[str]


# A fixed, deterministic Poiseuille problem spec. The same
# problem spec on both architectures should produce the same
# cas_hash if the plugin is bit-reproducible. The CFD plugin's
# validate() requires channel_length_m etc. at the *top level* of
# the params dict, not nested under "inputs", so we hoist them.
FIXED_PROBLEM = {
    "step_id": "rosetta-mvr04-poiseuille",
    "problem_id": "01900000-0000-0000-0000-000000000099",
    "physics_class": "fluid",
    "channel_length_m": 0.01,
    "channel_height_m": 0.001,
    "pressure_gradient_pa_per_m": 100.0,
    "viscosity_pa_s": 0.001,
    "n_y": 11,
    "case": "poiseuille_2d",
    "geometry": {"cas_hash": "0" * 64, "format": "native"},
    "boundary_conditions": [
        {"type": "neumann", "region": "inlet", "values": {"pressure_gradient_pa_per_m": 100.0}},
        {"type": "dirichlet", "region": "wall", "values": {"velocity": 0.0}},
    ],
    "solver_settings": {"case": "poiseuille_2d"},
    "fidelity_target": "high_fidelity",
    "determinism_class": "bit-reproducible",
}


def _spawn(arch: str | None) -> dict | None:
    """Spawn the plugin under the given architecture. arch=None
    means native. arch='x86_64' means Rosetta (with the system
    Python, which is a universal binary and runs under Rosetta).
    """
    request = {"jsonrpc": "2.0", "id": "rosetta-mvr04", "method": "execute", "params": FIXED_PROBLEM}
    # Use the system python (universal binary) under Rosetta, and
    # the homebrew python (aarch64 native) for the native run.
    if arch == "x86_64":
        py = "/usr/bin/python3"
    else:
        py = sys.executable
    cmd = [py, str(PLUGIN)]
    full = (["arch", "-x86_64"] if arch == "x86_64" else []) + cmd
    try:
        proc = subprocess.run(
            full,
            input=json.dumps(request) + "\n",
            capture_output=True,
            text=True,
            timeout=30,
            check=False,
        )
    except (OSError, subprocess.TimeoutExpired) as exc:
        print(f"rosetta_cross_arch: spawn under {arch or 'native'} failed: {exc!r}", file=sys.stderr)
        return None
    for raw in (proc.stdout or "").splitlines():
        raw = raw.strip()
        if not raw:
            continue
        try:
            r = json.loads(raw)
        except json.JSONDecodeError:
            continue
        if r.get("id") == "rosetta-mvr04":
            return r
    return None


def check(repo_root: Path) -> _Result:  # type: ignore[name-defined]
    from run import CheckResult  # local import; avoid circular at module import

    if not PLUGIN.is_file():
        return CheckResult(
            name="rosetta_cross_arch",
            ok=False,
            summary=f"cfd-reference-minimal plugin not found at {PLUGIN}",
            details=[],
        )

    # Detect Rosetta availability.
    rosetta_ok = shutil.which("arch") is not None and _arch_works("x86_64")
    if not rosetta_ok:
        return CheckResult(
            name="rosetta_cross_arch",
            ok=False,
            summary="Rosetta (arch -x86_64) unavailable; cannot run the cross-arch emulation check on this machine",
            details=[
                "fix: install Rosetta on macOS (`softwareupdate --install-rosetta --agree-to-license`)",
                "fallback: provide a QEMU user-mode emulator or run on a real aarch64 + x86_64 host pair",
                "this fitness check is a partial closure of AP-16 (cross-architecture); the strong MVR-04/05/12 claim requires a real second physical architecture",
            ],
        )

    # Native (aarch64)
    native = _spawn(arch=None)
    if native is None or "error" in native:
        return CheckResult(
            name="rosetta_cross_arch",
            ok=False,
            summary="native aarch64 cfd-reference execute() did not return a JSON-RPC response",
            details=[f"native: {native!r}"],
        )
    # Rosetta (x86_64 emulation)
    rosetta = _spawn(arch="x86_64")
    if rosetta is None or "error" in rosetta:
        return CheckResult(
            name="rosetta_cross_arch",
            ok=False,
            summary="Rosetta x86_64 cfd-reference execute() did not return a JSON-RPC response",
            details=[f"rosetta: {rosetta!r}"],
        )

    native_hash = native.get("result", {}).get("outputs", {}).get("result.bundle", {}).get("cas_hash")
    rosetta_hash = rosetta.get("result", {}).get("outputs", {}).get("result.bundle", {}).get("cas_hash")
    if not native_hash or not rosetta_hash:
        return CheckResult(
            name="rosetta_cross_arch",
            ok=False,
            summary="one or both executions did not produce a cas_hash",
            details=[f"native_hash={native_hash!r}, rosetta_hash={rosetta_hash!r}"],
        )

    if native_hash != rosetta_hash:
        return CheckResult(
            name="rosetta_cross_arch",
            ok=False,
            summary="cross-arch emulation: cas_hash mismatch (aarch64 native vs x86_64 Rosetta)",
            details=[
                f"native (aarch64): {native_hash}",
                f"rosetta (x86_64): {rosetta_hash}",
                "this is a real divergence: a future fix in the plugin should restore the bit-equivalence",
            ],
        )

    return CheckResult(
        name="rosetta_cross_arch",
        ok=True,
        summary=(
            f"cross-arch emulation equivalence: aarch64 native == x86_64 Rosetta "
            f"(cas_hash={native_hash[:16]}...)"
        ),
        details=[
            f"native (aarch64) cas_hash: {native_hash}",
            f"rosetta (x86_64) cas_hash: {rosetta_hash}",
            "scope: Apple Rosetta dynamic binary translation (NOT a real second physical architecture)",
            "strong MVR-04/05/12 acceptance requires a real second physical architecture; AP-16",
        ],
    )


def _arch_works(arch_name: str) -> bool:
    """Returns True if ``arch -<arch_name> <command>`` runs successfully."""
    try:
        proc = subprocess.run(
            ["arch", f"-{arch_name}", "true"],
            capture_output=True,
            text=True,
            timeout=5,
            check=False,
        )
        return proc.returncode == 0
    except (OSError, subprocess.TimeoutExpired):
        return False


if __name__ == "__main__":
    import argparse
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--repo-root", type=Path, default=REPO_ROOT_DEFAULT)
    args = parser.parse_args()
    r = check(args.repo_root)
    print(f"[{'OK ' if r.ok else 'FAIL'}] {r.name}: {r.summary}")
    for d in r.details:
        print(f"       {d}")
    sys.exit(0 if r.ok else 1)
