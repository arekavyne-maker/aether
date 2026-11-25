#!/usr/bin/env bash
# W4 acceptance: local web interface as a CLI adapter.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
export PATH="$HOME/.cargo/bin:$PATH"

fail() { echo "FAIL: $*" >&2; exit 1; }
pass() { echo "PASS: $*"; }

BIN="$ROOT/target/debug/aether-web"
[ -x "$BIN" ] || fail "aether-web not built"

TOKEN="aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
WORK="$(mktemp -d)"
PORT="${AETHER_WEB_TEST_PORT:-8789}"

echo "== 1. CLI is untouched =="
(cd "$ROOT" && git diff --quiet -- crates/aether_cli) || fail "CLI source changed"
pass "no CLI source modifications"

echo "== 2. server starts on loopback and requires a token =="
AETHER_WEB_TOKEN="$TOKEN" AETHER_WEB_BIND="127.0.0.1:$PORT" "$BIN" > "$WORK/server.log" 2>&1 &
SERVER_PID=$!
trap 'kill "$SERVER_PID" 2>/dev/null || true; rm -rf "$WORK"' EXIT
for _ in $(seq 1 30); do
    if curl -sf "http://127.0.0.1:$PORT/" >/dev/null 2>&1; then break; fi
    sleep 0.2
done
curl -sf "http://127.0.0.1:$PORT/" >/dev/null || fail "static shell not served"
code=$(curl -s -o /dev/null -w "%{http_code}" "http://127.0.0.1:$PORT/api/status")
[ "$code" = "401" ] || fail "unauthenticated API returned $code"
pass "loopback server and bearer auth"

echo "== 3. discovery, catalog and status =="
curl -sf -H "Authorization: Bearer $TOKEN" "http://127.0.0.1:$PORT/api/commands" \
    | python3 -c "import json,sys; d=json.load(sys.stdin); assert len(d)>=100, len(d)" || fail "discovery"
curl -sf -H "Authorization: Bearer $TOKEN" "http://127.0.0.1:$PORT/api/catalog" \
    | python3 -c "import json,sys; d=json.load(sys.stdin); assert len(d['workspaces'])==9 and len(d['commands'])>=40" || fail "catalog"
curl -sf -H "Authorization: Bearer $TOKEN" "http://127.0.0.1:$PORT/api/status" \
    | python3 -c "import json,sys; d=json.load(sys.stdin); assert d['certification_claim'] is False" || fail "status"
pass "machine-readable discovery, catalog and honest status"

echo "== 4. a curated command runs through the CLI =="
curl -sf -H "Authorization: Bearer $TOKEN" -H 'Content-Type: application/json' \
    -d '{"name":"status","values":{}}' \
    "http://127.0.0.1:$PORT/api/invoke" \
    | python3 -c "import json,sys; d=json.load(sys.stdin); assert d['state']=='ok' and d['exit_code']==0, d" || fail "invoke status"
pass "curated command spawns real CLI"

echo "== 5. fail-closed state is preserved =="
curl -sf -H "Authorization: Bearer $TOKEN" -H 'Content-Type: application/json' \
    -d "{\"name\":\"classify check\",\"values\":{\"file\":\"$ROOT/examples/wingbox/classification-denied.yaml\"}}" \
    "http://127.0.0.1:$PORT/api/invoke" \
    | python3 -c "import json,sys; d=json.load(sys.stdin); assert d['state']=='blocked' and d['exit_code']==30, d" || fail "blocked state"
pass "exit 30 rendered as Blocked, not success"

echo "== 6. raw console rejects undiscovered commands =="
code=$(curl -s -o /dev/null -w "%{http_code}" -H "Authorization: Bearer $TOKEN" \
    -H 'Content-Type: application/json' -d '{"command":["rm","-rf","/"],"flags":[]}' \
    "http://127.0.0.1:$PORT/api/raw")
[ "$code" = "400" ] || fail "undiscovered command accepted with $code"
pass "raw console is discovery-validated and shell-free"

echo
echo "W4 acceptance complete. CLI remains untouched."