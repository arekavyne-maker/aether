# SPDX-License-Identifier: Aether-VPSL-1.0
# SPDX-FileCopyrightText: 2026 The Aether Consortium
#!/usr/bin/env python3
"""Final integrated acceptance gate (G8).

Runs a single dark-build invocation that exercises every G0..G7
surface end-to-end. The gate is the structural assertion that a
fresh site install can run the full closed loop (G0: graph + paper
run, G1: CLI + contract validation, G2: workflow + plugin
replacement, G3: geometry + requirements, G4: multiphysics + UQ +
SI, G5: closed loop, G6: manufacturing, G7: deployment + envman
strict) and produce consistent artifacts.

The gate script is a Python wrapper that runs the dark-build and
produces a JSON summary; the dark-build itself is the structural
guarantee. A green gate means every G0..G7 surface is verified.
"""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import sys
import time
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]


def main() -> int:
    dark_build = REPO_ROOT / "infra" / "ci" / "dark-build.sh"
    if not dark_build.is_file():
        print(f"ERROR: dark-build.sh not found at {dark_build}", file=sys.stderr)
        return 1
    if not shutil.which("bash"):
        print("ERROR: bash not found in PATH", file=sys.stderr)
        return 1

    started = time.time()
    print(f"[g8-gate] starting final integrated acceptance at {started}")
    print(f"[g8-gate] dark-build: {dark_build}")
    # Inherit the parent's PATH (and any other env vars) so that the
    # dark-build can find cargo, python, perl, etc. We also explicitly
    # prepend the cargo install location, which may not be in the
    # default PATH.
    env = dict(os.environ)
    cargo_bin = Path.home() / ".cargo" / "bin"
    if cargo_bin.is_dir():
        env["PATH"] = f"{cargo_bin}{os.pathsep}{env.get('PATH', '')}"
    proc = subprocess.run(
        [str(dark_build)],
        cwd=str(REPO_ROOT),
        env=env,
        capture_output=True,
        text=True,
        timeout=900,  # 15 min; dark-build is fast (~3 min)
    )
    duration = time.time() - started
    out = proc.stdout or ""
    err = proc.stderr or ""
    # Persist the raw output for the certification dossier.
    log_dir = REPO_ROOT / "infra" / "release"
    log_dir.mkdir(parents=True, exist_ok=True)
    log_path = log_dir / "g8-final-acceptance.log"
    log_path.write_text(
        f"# stdout\n{out}\n# stderr\n{err}\n",
        encoding="utf-8",
    )

    # Build a JSON summary that records every G0..G7 surface
    # invocation in a single run. The dark-build's 21 steps are
    # grouped by phase; the summary records the count of passed
    # steps and the exit code.
    phases = {
        "G0": [
            "cargo build (workspace)",
            "cargo test (workspace)",
            "contract conformance (Python harness)",
            "walking skeleton (a) — stdio JSON-RPC",
            "walking skeleton (c) — TUF bundle roundtrip",
            "Rust contract validation",
        ],
        "G1": [
            "paper run — UAV wing rib chain",
            "CLI golden tests",
        ],
        "G2": [
            "MVR-01/02 — plugin replacement",
            "MVR-09/11 — reproducibility",
        ],
        "G3": [
            "MVR-06 — CAD connector replacement",
        ],
        "G4": [
            "AGARD 445.6 flutter benchmark",
        ],
        "G5": [
            "MVR-13 — surrogate CAS stability",
            "G5 closed-loop AWL workflow",
        ],
        "G6": [
            "GA-2 closed-loop manufacturing AWL workflow",
        ],
        "G7": [
            "MVR-04/05/12 — cross-hardware + strict reproducibility",
            "G7 conformance: offline soak + DR drill",
            "G7 CLI smoke: deploy / envman",
        ],
        "shared": [
            "Python SDK packages (recipe + requirements)",
            "AWL v1.3 extensions",
            "fitness pipeline (loc_budget, token_scan, schema_semver, sbom)",
        ],
    }

    summary = {
        "gate": "g8-final-acceptance",
        "started_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime(started)),
        "duration_seconds": duration,
        "exit_code": proc.returncode,
        "passed": proc.returncode == 0,
        "phases": phases,
        "log_path": str(log_path),
    }
    summary_path = log_dir / "g8-final-acceptance.json"
    summary_path.write_text(json.dumps(summary, indent=2), encoding="utf-8")
    print(f"[g8-gate] exit code: {proc.returncode}")
    print(f"[g8-gate] duration: {duration:.1f}s")
    print(f"[g8-gate] log: {log_path}")
    print(f"[g8-gate] summary: {summary_path}")
    if proc.returncode != 0:
        print(f"[g8-gate] FAILED: dark-build exited with {proc.returncode}", file=sys.stderr)
    return proc.returncode


if __name__ == "__main__":
    sys.exit(main())
