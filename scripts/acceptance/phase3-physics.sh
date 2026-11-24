#!/usr/bin/env bash
# Phase C acceptance: real structural, thermal, and buckling solvers.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

fail() { echo "FAIL: $*" >&2; exit 1; }
pass() { echo "PASS: $*"; }

echo "== 1. sparse beam FEM matches four closed forms =="
python3 -m pytest -q "$ROOT/reference_plugins/fea-reference-spar/tests" >/dev/null
pass "cantilever, SS point, SS uniform, and axial bar match analytics"

echo "== 2. thermal conduction matches analytics =="
python3 -m pytest -q "$ROOT/reference_plugins/fea-reference-thermal/tests" >/dev/null
pass "no-source and uniform-source conduction match analytics"

echo "== 3. buckling matches the Euler column =="
python3 -m pytest -q "$ROOT/reference_plugins/fea-reference-buckling/tests" >/dev/null
pass "pinned-pinned and cantilever Euler loads match analytics"

echo "== 4. CalculiX still agrees with the beam FEM =="
"$ROOT/target/debug/aether" benchmark AETHER-BENCH-08 --json > "$WORK/b8.json"
python3 - "$WORK/b8.json" <<'PY'
import json, sys
v = json.load(open(sys.argv[1]))
b = v["benchmarks"][0]
assert b["skipped"] is False, b
assert b["passed"] is True, b
assert b["metrics"]["relative_difference"] < 0.05, b
PY
pass "beam FEM and CalculiX agree within 5%"

echo
echo "Phase C acceptance complete. Every solver is verified against analytics."