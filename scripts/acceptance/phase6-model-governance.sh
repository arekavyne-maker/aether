#!/usr/bin/env bash
# Phase 6 acceptance: model-card promotion governance.
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

CARD="$ROOT/examples/wingbox/model-card.yaml"
BAD="$ROOT/examples/wingbox/model-card-incomplete.yaml"

echo "== 1. complete evidence promotes =="
"$BIN" model verify "$CARD" --approver-role chief-engineer --json > "$WORK/verify.json"
python3 - "$WORK/verify.json" <<'PY'
import json, sys
v = json.load(open(sys.argv[1]))
assert v["passed"] is True, v
assert v["independent_validation"] is True, v
assert v["missing"] == [] and v["stale"] == [], v
PY
pass "current verification, validation, and benchmark evidence pass"

echo "== 2. unauthorized role fails closed =="
if "$BIN" model verify "$CARD" --approver-role analyst --json > "$WORK/role.json" 2>/dev/null; then
    fail "analyst was allowed to promote a model"
fi
python3 - "$WORK/role.json" <<'PY'
import json, sys
v = json.load(open(sys.argv[1]))
assert v["passed"] is False, v
assert any("not authorized" in r for r in v["reasons"]), v
PY
pass "promotion is role-gated"

echo "== 3. missing and stale evidence are named =="
if "$BIN" model verify "$BAD" --approver-role chief-engineer --json > "$WORK/bad.json" 2>/dev/null; then
    fail "incomplete evidence passed"
fi
python3 - "$WORK/bad.json" <<'PY'
import json, sys
v = json.load(open(sys.argv[1]))
assert v["passed"] is False, v
assert any("Validation" in m for m in v["missing"]), v
assert any("Benchmark" in s for s in v["stale"]), v
PY
pass "missing and stale evidence are distinguished"

echo "== 4. promotion emits an approved card =="
"$BIN" model promote "$CARD" --approver-role chief-engineer \
    --output "$WORK/promoted.json" --json > "$WORK/promote.json"
python3 - "$WORK/promoted.json" <<'PY'
import json, sys
v = json.load(open(sys.argv[1]))
assert v["state"] == "promoted", v
assert v["approved_by"] == "chief-engineer", v
PY
pass "promoted card records state and approver"

echo
echo "Phase 6 acceptance complete. No model is trained or certified here."
