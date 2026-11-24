#!/usr/bin/env bash
# Phase G acceptance: governed model inference and bounded agent execution.
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

echo "== 1. an in-envelope prediction is produced =="
"$BIN" infer "$ROOT/examples/wingbox/model-infer.yaml" --json > "$WORK/infer.json"
python3 - "$WORK/infer.json" <<'PY'
import json, sys
v = json.load(open(sys.argv[1]))["prediction"]
assert v["in_domain"] is True, v
assert v["fallback"] is None, v
assert abs(v["value"] - 0.38) < 1e-9, v
PY
pass "approved model predicts inside its envelope"

echo "== 2. an out-of-envelope request is flagged, not used =="
python3 - "$ROOT/examples/wingbox/model-infer.yaml" "$WORK/ood.yaml" <<'PY'
import sys, yaml
doc = yaml.safe_load(open(sys.argv[1]))
doc["features"]["tip_load_kN"] = 500.0
yaml.safe_dump(doc, open(sys.argv[2], "w"), sort_keys=False)
PY
"$BIN" infer "$WORK/ood.yaml" --json > "$WORK/ood.json"
python3 - "$WORK/ood.json" <<'PY'
import json, sys
v = json.load(open(sys.argv[1]))["prediction"]
assert v["in_domain"] is False, v
assert "tip_load_kN" in v["out_of_domain"], v
assert v["fallback"], v
PY
pass "out-of-domain predictions carry a fallback"

echo "== 3. an unapproved model is refused =="
python3 - "$ROOT/examples/wingbox/model-infer.yaml" "$WORK/unapproved.yaml" <<'PY'
import sys, yaml
doc = yaml.safe_load(open(sys.argv[1]))
doc["model"]["state"] = "benchmarked"
yaml.safe_dump(doc, open(sys.argv[2], "w"), sort_keys=False)
PY
if "$BIN" infer "$WORK/unapproved.yaml" --json > /dev/null 2>&1; then
    fail "an unapproved model was used"
fi
pass "only approved models may predict"

echo "== 4. a read-only agent proposal runs =="
"$BIN" agent-run "$ROOT/examples/wingbox/agent-run-allowed.yaml" --json > "$WORK/agent.json"
python3 - "$WORK/agent.json" <<'PY'
import json, sys
v = json.load(open(sys.argv[1]))
assert v["decision"]["allowed"] is True, v
assert v["executed"] is True, v
PY
pass "explorer may query"

echo "== 5. a mutating proposal needs a human =="
if "$BIN" agent-run "$ROOT/examples/wingbox/agent-run-needs-approval.yaml" --json > "$WORK/no-approval.json" 2>&1; then
    fail "a mutating proposal ran without approval"
fi
python3 - "$WORK/no-approval.json" <<'PY'
import json, sys
v = json.load(open(sys.argv[1]))
assert v["decision"]["allowed"] is True, v
assert v["decision"]["requires_human_approval"] is True, v
assert v["executed"] is False, v
assert v["audit"][0]["reason_code"] == "approval_required", v
PY
pass "mutating proposals require an authorized human"

echo "== 6. an unauthorized role is denied =="
if "$BIN" agent-run "$ROOT/examples/wingbox/agent-run-denied.yaml" --json > "$WORK/denied.json" 2>&1; then
    fail "a designer was allowed to run an analysis"
fi
python3 - "$WORK/denied.json" <<'PY'
import json, sys
v = json.load(open(sys.argv[1]))
assert v["decision"]["allowed"] is False, v
assert v["decision"]["reason_code"] == "tool_not_in_role", v
PY
pass "the permission matrix is enforced"

echo
echo "Phase G acceptance complete. Agents advise; the runtime decides."