#!/usr/bin/env bash
# Phase 7 acceptance: plugin sandbox policy enforcement.
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

echo "== 1. a conforming request passes =="
"$BIN" sandbox check "$ROOT/examples/wingbox/sandbox-ok.yaml" --json > "$WORK/ok.json"
python3 - "$WORK/ok.json" <<'PY'
import json, sys
v = json.load(open(sys.argv[1]))
assert v["valid"] is True, v
PY
pass "reference plugin request fits the policy"

echo "== 2. escape attempts are refused =="
if "$BIN" sandbox check "$ROOT/examples/wingbox/sandbox-bad.yaml" --json > "$WORK/bad.json" 2>/dev/null; then
    fail "an escaping request was accepted"
fi
python3 - "$WORK/bad.json" <<'PY'
import json, sys
v = json.load(open(sys.argv[1]))
assert v["valid"] is False, v
assert v["error"], v
PY
pass "write escape, secret env, and network escalation are refused"

echo
echo "Phase 7 sandbox-policy acceptance complete."
echo "Kernel-level isolation (seccomp/namespaces/containers) remains an adapter."