#!/usr/bin/env bash
# Phase 1 acceptance: typed quantities and frames fail before solver submission.
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

echo "== 1. equivalent units and frames pass =="
"$BIN" units check "$ROOT/examples/wingbox/units-ok.yaml" --json > "$WORK/ok.json"
python3 - "$WORK/ok.json" <<'PY'
import json, sys
v = json.load(open(sys.argv[1]))["report"]
assert v["valid"] is True, v
assert v["quantities_checked"] == 2, v
assert v["points_checked"] == 1, v
PY
pass "kN to N and mm to m convert, and the frame round-trips"

echo "== 2. a dimension mismatch fails closed =="
if "$BIN" units check "$ROOT/examples/wingbox/units-bad.yaml" --json > "$WORK/bad.json" 2>/dev/null; then
    fail "a length supplied as a force was accepted"
fi
python3 - "$WORK/bad.json" <<'PY'
import json, sys
v = json.load(open(sys.argv[1]))["report"]
assert v["valid"] is False, v
assert any("tip_load" in i for i in v["issues"]), v
PY
pass "tip_load supplied in metres is rejected before the solver"

echo
echo "Phase 1 units/frames boundary acceptance complete."