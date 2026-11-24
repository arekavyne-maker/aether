#!/usr/bin/env bash
# Phase C acceptance: material allowables and flight-regime envelopes.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
BIN="$ROOT/target/debug/aether"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

if [ ! -x "$BIN" ]; then
    echo "error: build first (cargo build -p aether_cli)" >&2
    exit 1
fi

fail() { echo "FAIL: $*" >&2; exit 1; }
pass() { echo "PASS: $*"; }

echo "== 1. an in-envelope allowable interpolates =="
"$BIN" material check "$ROOT/examples/wingbox/material-al2024.yaml" --json > "$WORK/mat.json"
python3 - "$WORK/mat.json" <<'PY'
import json, sys
v = json.load(open(sys.argv[1]))
assert v["valid"] is True, v
assert 290e6 < v["allowable"]["value"] < 345e6, v
assert v["allowable"]["basis"] == "abasis", v
PY
pass "interpolation stays inside the declared temperature range"

echo "== 2. an out-of-envelope request fails closed =="
python3 - "$ROOT/examples/wingbox/material-al2024.yaml" "$WORK/hot.yaml" <<'PY'
import sys, yaml
doc = yaml.safe_load(open(sys.argv[1]))
doc["temperature_c"] = 500.0
yaml.safe_dump(doc, open(sys.argv[2], "w"), sort_keys=False)
PY
if "$BIN" material check "$WORK/hot.yaml" --json > "$WORK/hot.json" 2>&1; then
    fail "an out-of-envelope temperature was accepted"
fi
python3 - "$WORK/hot.json" <<'PY'
import json, sys
v = json.load(open(sys.argv[1]))
assert v["valid"] is False, v
assert "outside" in v["error"].lower() or "envelope" in v["error"].lower(), v
PY
pass "extrapolation is rejected"

echo "== 3. a valid flight envelope passes =="
"$BIN" loads validate "$ROOT/examples/wingbox/loads-subsonic.yaml" --json > "$WORK/loads.json"
python3 - "$WORK/loads.json" <<'PY'
import json, sys
v = json.load(open(sys.argv[1]))
assert v["valid"] is True, v
assert v["limit_load_factor"] == 2.5, v
assert abs(v["ultimate_load_factor"] - 3.75) < 1e-9, v
assert len(v["environments"]) == 4, v
PY
pass "envelope validates and derives limit/ultimate factors"

echo "== 4. a Mach number outside the envelope is rejected =="
python3 - "$ROOT/examples/wingbox/loads-subsonic.yaml" "$WORK/bad-mach.yaml" <<'PY'
import sys, yaml
doc = yaml.safe_load(open(sys.argv[1]))
doc["envelope"]["operating_points"][0]["mach"] = 2.0
yaml.safe_dump(doc, open(sys.argv[2], "w"), sort_keys=False)
PY
if "$BIN" loads validate "$WORK/bad-mach.yaml" --json > "$WORK/bad.json" 2>&1; then
    fail "a Mach number outside the envelope was accepted"
fi
pass "operating points are checked against the declared range"

echo
echo "Phase C materials/loads acceptance complete."