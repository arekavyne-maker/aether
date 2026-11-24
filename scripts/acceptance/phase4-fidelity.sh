#!/usr/bin/env bash
# Phase 4 acceptance: deterministic multi-fidelity selection.
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

echo "== 1. cheapest level meeting the target is selected =="
"$BIN" fidelity plan "$ROOT/examples/wingbox/fidelity.yaml" --json > "$WORK/plan.json"
python3 - "$WORK/plan.json" <<'PY'
import json, sys
v = json.load(open(sys.argv[1]))["plan"]
assert v["selected"] == "beam_fem", v
assert v["meets_target"] is True, v
assert v["budget_exceeded"] is False, v
assert v["validation_required"] is True, v
assert v["escalation_path"] == ["closed_form", "beam_fem", "shell_fem"], v
assert any("fidelity changed" in r for r in v["reasons"]), v
PY
pass "selection and fidelity change are explained"

echo "== 2. unreachable target escalates explicitly =="
"$BIN" fidelity plan "$ROOT/examples/wingbox/fidelity-tight.yaml" --json > "$WORK/tight.json"
python3 - "$WORK/tight.json" <<'PY'
import json, sys
v = json.load(open(sys.argv[1]))["plan"]
assert v["selected"] == "shell_fem", v
assert v["meets_target"] is False, v
assert v["budget_exceeded"] is True, v
assert any("no available level" in r for r in v["reasons"]), v
assert any("above the" in r for r in v["reasons"]), v
PY
pass "escalation names the binding constraint"

echo "== 3. invalid requests fail closed =="
python3 - "$ROOT/examples/wingbox/fidelity.yaml" "$WORK/bad.yaml" <<'PY'
import sys, yaml
doc = yaml.safe_load(open(sys.argv[1]))
doc["target_error"] = 0.0
yaml.safe_dump(doc, open(sys.argv[2], "w"), sort_keys=False)
PY
if "$BIN" fidelity plan "$WORK/bad.yaml" --json > /dev/null 2>&1; then
    fail "invalid target_error was accepted"
fi
pass "invalid target_error is rejected"

echo
echo "Phase 4 acceptance complete. Declared errors remain claims, not measurements."
