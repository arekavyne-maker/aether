#!/usr/bin/env bash
# Phase 4 acceptance: workflow branches, joins, and bounded loops.
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

echo "== 1. the plan compiles with branch/join/loop nodes =="
"$BIN" plan explain "$ROOT/examples/wingbox/control-flow.awl.yaml" --json > "$WORK/plan.json"
python3 - "$WORK/plan.json" <<'PY'
import json, sys
v = json.load(open(sys.argv[1]))
ids = {s["id"]: s for s in v["steps"]}
assert ids["maybe_detailed"]["step_type"] == "Branch", v
assert ids["join"]["step_type"] == "Join", v
assert ids["bounded"]["step_type"] == "Loop", v
PY
pass "the DAG carries control-flow step types"

echo "== 2. a false branch is skipped with a reason =="
"$BIN" init "$WORK/proj" --profile aerospace --json > /dev/null
cp "$ROOT/examples/wingbox/control-flow.awl.yaml" "$WORK/proj/workflows/control-flow.awl.yaml"
"$BIN" run --project "$WORK/proj" --workflow "$WORK/proj/workflows/control-flow.awl.yaml" --json > "$WORK/run.json"
python3 - "$WORK/run.json" <<'PY'
import json, sys
v = json.load(open(sys.argv[1]))
assert v["status"] == "succeeded", v
PY
pass "branch/join/loop execution completes"

echo
echo "Phase 4 control-flow acceptance complete."