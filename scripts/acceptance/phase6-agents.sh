#!/usr/bin/env bash
# Phase 6 acceptance: AI agent proposals are advisor-only and permission-gated.
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

echo "== 1. an allowed read-only proposal =="
"$BIN" agent check "$ROOT/examples/wingbox/agent-proposal-allowed.yaml" --json > "$WORK/allowed.json"
python3 - "$WORK/allowed.json" <<'PY'
import json, sys
v = json.load(open(sys.argv[1]))["decision"]
assert v["allowed"] is True, v
assert v["requires_human_approval"] is False, v
PY
pass "explorer may query the graph"

echo "== 2. a mutating proposal is allowed but needs human approval =="
"$BIN" agent check "$ROOT/examples/wingbox/agent-proposal-needs-approval.yaml" --json > "$WORK/approval.json"
python3 - "$WORK/approval.json" <<'PY'
import json, sys
v = json.load(open(sys.argv[1]))["decision"]
assert v["allowed"] is True, v
assert v["requires_human_approval"] is True, v
PY
pass "workflow execution requires an authorized human approval"

echo "== 3. self-approval is prohibited =="
if "$BIN" agent check "$ROOT/examples/wingbox/agent-proposal-denied.yaml" --json > "$WORK/denied.json" 2>/dev/null; then
    fail "an agent was allowed to approve"
fi
python3 - "$WORK/denied.json" <<'PY'
import json, sys
v = json.load(open(sys.argv[1]))["decision"]
assert v["allowed"] is False, v
assert v["reason_code"] == "prohibited_for_agents", v
PY
pass "agents can never approve, edit policy, or control machines"

echo
echo "Phase 6 agent-governance acceptance complete."
echo "Agents advise; authorized humans and deterministic checks decide."