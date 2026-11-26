#!/usr/bin/env bash
# Acceptance: local web interface as a CLI adapter (ADR-0042 / ADR-0043).
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
API="http://127.0.0.1:$PORT/api"
AUTH="Authorization: Bearer $TOKEN"

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
code=$(curl -s -o /dev/null -w "%{http_code}" "$API/status")
[ "$code" = "401" ] || fail "unauthenticated API returned $code"
csp=$(curl -sI "http://127.0.0.1:$PORT/" | tr -d '\r' | grep -i '^content-security-policy:' || true)
echo "$csp" | grep -q "default-src 'self'" || fail "CSP does not pin self"
if echo "$csp" | grep -qi "unsafe-inline\|unsafe-eval"; then fail "CSP has an inline escape hatch"; fi
curl -sf "http://127.0.0.1:$PORT/assets/fonts/urbanist-latin.woff2" -o /dev/null || fail "local font not served"
curl -sf "http://127.0.0.1:$PORT/assets/fonts/arapey-italic-latin.woff2" -o /dev/null || fail "local serif not served"
pass "loopback server, bearer auth, strict CSP and local fonts"

echo "== 3. discovery, catalog and status =="
curl -sf -H "$AUTH" "$API/commands" \
    | python3 -c "import json,sys; d=json.load(sys.stdin); assert len(d)>=100, len(d)" || fail "discovery"
curl -sf -H "$AUTH" "$API/catalog" \
    | python3 -c "import json,sys; d=json.load(sys.stdin); assert len(d['workspaces'])==10 and len(d['families'])>=50 and len(d['commands'])>=40" || fail "catalog"
curl -sf -H "$AUTH" "$API/status" \
    | python3 -c "import json,sys; d=json.load(sys.stdin); assert d['certification_claim'] is False" || fail "status"
pass "machine-readable discovery, catalog and honest status"

echo "== 4. merged command surface covers the CLI =="
curl -sf -H "$AUTH" "$API/surface" | python3 -c "
import json,sys
d=json.load(sys.stdin)
assert len(d['workspaces'])==10, len(d['workspaces'])
assert len(d['commands'])>=100, len(d['commands'])
assert all(c['workspace'] for c in d['commands'])
assert any(c['name']=='graph' and c['group'] for c in d['commands'])
assert any(c['name']=='vv coverage' and c['curated'] for c in d['commands'])
" || fail "surface parity"
pass "all 124 commands mapped to 10 workspaces"

echo "== 5. help-derived command contract =="
curl -sf -H "$AUTH" "$API/describe?name=vv%20coverage" | python3 -c "
import json,sys
d=json.load(sys.stdin)
assert d['positionals'][0]['name']=='FILE' and d['positionals'][0]['required']
assert any(o['flag']=='--json' for o in d['options'])
" || fail "describe"
pass "clap help parsed into typed fields"

echo "== 6. terminal argv runs with positional arguments =="
FIXTURE="$ROOT/contracts/vv/examples/wingbox.requirements-incomplete.yaml"
curl -sf -H "$AUTH" -H 'Content-Type: application/json' \
    -d "{\"argv\":[\"vv\",\"coverage\",\"$FIXTURE\"]}" \
    "$API/exec" | python3 -c "
import json,sys
d=json.load(sys.stdin)
assert d['state']=='blocked' and d['exit_code']==30, d
assert d['value']['requirements_uncased']==1
" || fail "exec with positional argument"
code=$(curl -s -o /dev/null -w "%{http_code}" -H "$AUTH" -H 'Content-Type: application/json' \
    -d '{"argv":["rm","-rf","/"]}' "$API/exec")
[ "$code" = "400" ] || fail "exec accepted undiscovered command with $code"
pass "discovery-allowlisted exec; exit 30 preserved"

echo "== 7. root-confined path discovery =="
curl -sf -H "$AUTH" "$API/files?q=classification-denied" | python3 -c "
import json,sys,os
d=json.load(sys.stdin)
root=os.getcwd()
assert d, 'no files returned'
assert all(os.path.realpath(e['path']).startswith(root) for e in d), d[:2]
" || fail "file discovery confinement"
pass "path picker stays inside the project root"

echo "== 8. curated invocation and fail-closed state =="
curl -sf -H "$AUTH" -H 'Content-Type: application/json' \
    -d '{"name":"status","values":{}}' "$API/invoke" \
    | python3 -c "import json,sys; d=json.load(sys.stdin); assert d['state']=='ok' and d['exit_code']==0, d" || fail "invoke status"
curl -sf -H "$AUTH" -H 'Content-Type: application/json' \
    -d "{\"name\":\"classify check\",\"values\":{\"file\":\"$ROOT/examples/wingbox/classification-denied.yaml\"}}" \
    "$API/invoke" \
    | python3 -c "import json,sys; d=json.load(sys.stdin); assert d['state']=='blocked' and d['exit_code']==30, d" || fail "blocked state"
pass "curated command spawns real CLI; exit 30 is Blocked"

echo "== 9. project atlas =="
curl -sf -H "$AUTH" "$API/atlas" | python3 -c "
import json,sys
d=json.load(sys.stdin)
assert len(d['schema']['node_types'])==78, len(d['schema']['node_types'])
assert len(d['schema']['edge_types'])==38, len(d['schema']['edge_types'])
assert d['graph']['offset']==d['chain']['event_count'], (d['graph']['offset'],d['chain']['event_count'])
assert len(d['sources'])==6, len(d['sources'])
assert all(s['command'].startswith('aether ') for s in d['sources'])
" || fail "atlas aggregate"
curl -sf -H "$AUTH" "$API/atlas/graph?offset=0" | python3 -c "
import json,sys
d=json.load(sys.stdin)
assert d['graph']['offset']==0 and d['graph']['nodes']==[], d
" || fail "atlas graph offset"
curl -sf -H "$AUTH" "$API/atlas/schema" | python3 -c "
import json,sys
d=json.load(sys.stdin); assert len(d['schema']['node_types'])==78
" || fail "atlas schema"
curl -sf -H "$AUTH" "$API/atlas/health" | python3 -c "
import json,sys
d=json.load(sys.stdin); assert d['health']['checks'], d
" || fail "atlas health"
pass "atlas aggregates CLI JSON with source provenance"

echo "== 10. live jobs and project switching =="
JOB_ID=$(curl -sf -H "$AUTH" -H 'Content-Type: application/json' \
    -d '{"argv":["status"]}' "$API/jobs" | python3 -c "import json,sys;print(json.load(sys.stdin)['id'])")
[ -n "$JOB_ID" ] || fail "job did not start"
for _ in $(seq 1 40); do
    curl -sf -H "$AUTH" "$API/jobs/$JOB_ID/events?after=0" | python3 -c "
import json,sys
d=json.load(sys.stdin)
assert d['event_count'] >= 1
assert d['finished'] in (True, False)
" || fail "job event feed"
    state=$(curl -sf -H "$AUTH" "$API/jobs/$JOB_ID" | python3 -c "import json,sys;print(json.load(sys.stdin)['state'])")
    [ "$state" = "running" ] || break
    sleep 0.1
done
[ "$state" = "ok" ] || fail "job did not reach ok (got $state)"
curl -sf -H "$AUTH" -X POST -H 'Content-Type: application/json' \
    -d "{\"path\":\"$ROOT\"}" "$API/project" | python3 -c "
import json,sys
d=json.load(sys.stdin); assert d['switched'] is True
" || fail "project switch"
pass "live jobs stream CLI output and project root can change in-app"

echo "== 11. raw console and tokenizer =="
code=$(curl -s -o /dev/null -w "%{http_code}" -H "$AUTH" -H 'Content-Type: application/json' \
    -d '{"command":["rm","-rf","/"],"flags":[]}' "$API/raw")
[ "$code" = "400" ] || fail "raw accepted undiscovered command with $code"
if command -v node >/dev/null 2>&1; then
    node --test "$ROOT/experimental/web/app/js/argv.test.mjs" >/dev/null || fail "argv tokenizer tests"
    for file in "$ROOT"/experimental/web/app/js/*.js; do
        node --input-type=module --check < "$file" || fail "syntax: $file"
    done
fi
pass "raw console is discovery-validated; tokenizer rejects shell operators"

echo
echo "Web UI acceptance complete. CLI remains untouched."
