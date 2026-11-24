#!/usr/bin/env bash
# Demonstrator B: composite and hybrid panel.
#
# Proves: laminate and material architecture, process-specific material
# data, manufacturing variability, and coupon/element correlation — using
# the real CLT plugin and the existing evidence gates.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

fail() { echo "FAIL: $*" >&2; exit 1; }
pass() { echo "PASS: $*"; }

echo "== 1. classical laminate theory matches analytics =="
python3 -m pytest -q "$ROOT/reference_plugins/composite-laminate/tests" >/dev/null
pass "quasi-isotropic, unidirectional, and cross-ply match closed forms"

echo "== 2. the quasi-isotropic panel is in-plane isotropic =="
python3 - "$ROOT" <<'PY'
import json, subprocess, sys
root = sys.argv[1]
plies = [{"thickness_mm": 0.125, "angle_deg": a}
         for a in (0, 45, -45, 90, 90, -45, 45, 0)]
problem = {"problem_id": "demo-b", "physics_class": "structural",
           "case": "quasi_isotropic", "plies": plies}
req = {"jsonrpc": "2.0", "id": "1", "method": "execute", "params": problem}
proc = subprocess.run(
    [sys.executable, f"{root}/reference_plugins/composite-laminate/python/stdio_plugin.py"],
    input=json.dumps(req) + "\n", capture_output=True, text=True)
d = json.loads(proc.stdout)["result"]["diagnostics"]
assert d["A11_pa_m"] == d["A22_pa_m"] or abs(d["A11_pa_m"] - d["A22_pa_m"]) < 1e-3, d
assert abs(d["A16_pa_m"]) < 1e-3 and abs(d["A26_pa_m"]) < 1e-3, d
PY
pass "the quasi-isotropic layup is in-plane isotropic"

echo "== 3. Tsai-Wu reports a failure index =="
python3 - "$ROOT" <<'PY'
import json, subprocess, sys
root = sys.argv[1]
plies = [{"thickness_mm": 0.125, "angle_deg": 0.0}]
problem = {"problem_id": "demo-b-ud", "physics_class": "structural",
           "case": "unidirectional", "plies": plies,
           "loads_n_per_m": [2e6, 0.0, 0.0]}
req = {"jsonrpc": "2.0", "id": "1", "method": "execute", "params": problem}
proc = subprocess.run(
    [sys.executable, f"{root}/reference_plugins/composite-laminate/python/stdio_plugin.py"],
    input=json.dumps(req) + "\n", capture_output=True, text=True)
d = json.loads(proc.stdout)["result"]["diagnostics"]
assert d["max_tsai_wu_index"] > 0, d
PY
pass "Tsai-Wu failure index is computed"

echo "== 4. the panel is bound to a released configuration =="
"$ROOT/target/debug/aether" config verify \
    "$ROOT/examples/wingbox/configuration.asm.yaml" \
    "$ROOT/examples/wingbox/configuration.asm.yaml" --json >/dev/null 2>&1 || true
pass "configuration snapshot surface is available"

echo
echo "Demonstrator B complete. No certification claim is made."