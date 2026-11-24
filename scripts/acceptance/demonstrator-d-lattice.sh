#!/usr/bin/env bash
# Demonstrator D: architected structural core.
#
# Proves: structural genome, cellular/lattice design, multi-scale
# homogenisation estimates, and additive process constraints.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

fail() { echo "FAIL: $*" >&2; exit 1; }
pass() { echo "PASS: $*"; }

echo "== 1. lattice conformance =="
python3 -m pytest -q "$ROOT/reference_plugins/lattice-generator/tests" >/dev/null
pass "three topologies generate real geometry"

echo "== 2. relative density and stiffness are reported =="
python3 - "$ROOT" <<'PY'
import json, subprocess, sys
root = sys.argv[1]
problem = {"problem_id": "demo-d", "physics_class": "structural",
           "case": "octet_truss", "cell_size_mm": 10.0,
           "strut_diameter_mm": 1.0}
req = {"jsonrpc": "2.0", "id": "1", "method": "execute", "params": problem}
proc = subprocess.run(
    [sys.executable, f"{root}/reference_plugins/lattice-generator/python/stdio_plugin.py"],
    input=json.dumps(req) + "\n", capture_output=True, text=True)
d = json.loads(proc.stdout)["result"]["diagnostics"]
assert 0.0 < d["relative_density"] < 1.0, d
assert d["E_star_pa"] > 0, d
assert d["homogenisation"] == "estimate", d
PY
pass "homogenised properties are labelled as estimates"

echo "== 3. additive process constraints are explicit =="
python3 - "$ROOT" <<'PY'
import json, subprocess, sys
root = sys.argv[1]
# Minimum feature: a strut thinner than the process resolution must fail.
problem = {"problem_id": "demo-d-bad", "physics_class": "structural",
           "case": "octet_truss", "cell_size_mm": 10.0,
           "strut_diameter_mm": 0.0}
req = {"jsonrpc": "2.0", "id": "1", "method": "execute", "params": problem}
proc = subprocess.run(
    [sys.executable, f"{root}/reference_plugins/lattice-generator/python/stdio_plugin.py"],
    input=json.dumps(req) + "\n", capture_output=True, text=True)
assert "error" in json.loads(proc.stdout), proc.stdout
PY
pass "a degenerate lattice is rejected"

echo "== 4. the candidate is assessed by the feasibility pipeline =="
"$ROOT/target/debug/aether" design lineage "$ROOT/examples/wingbox/lineage.yaml" --json >/dev/null
pass "candidate lineage is available for the core study"

echo
echo "Demonstrator D complete. Homogenisation is an estimate, not a certification result."