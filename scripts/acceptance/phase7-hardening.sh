#!/usr/bin/env bash
# Phase 7 acceptance (partial): offline doctor, graph chain, export/replay,
# tamper detection, and the documented restore drill.
#
# This does not claim plugin sandboxing, federation, or production
# hardening. Those remain open Phase 7 items.
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

PROJECT="$WORK/site"
BUNDLE="$WORK/bundle"

echo "== 1. offline doctor =="
"$BIN" doctor --json > "$WORK/doctor.json"
python3 - "$WORK/doctor.json" <<'PY'
import json, sys
v = json.load(open(sys.argv[1]))
assert v["core_operational"] is True, v
PY
pass "core runtime and graph are operational"

echo "== 2. project chain and export =="
"$BIN" init "$PROJECT" --profile aerospace --json > /dev/null
"$BIN" run --project "$PROJECT" --json > "$WORK/run.json"
python3 - "$WORK/run.json" <<'PY'
import json, sys
v = json.load(open(sys.argv[1]))
assert v["status"] == "succeeded", v
assert v["recorded"], v
PY
"$BIN" graph chain --project "$PROJECT" --json > "$WORK/chain.json"
python3 - "$WORK/chain.json" <<'PY'
import json, sys
v = json.load(open(sys.argv[1]))
assert v["valid"] is True, v
PY
"$BIN" export --project "$PROJECT" --output "$BUNDLE" --json > "$WORK/export.json"
python3 - "$WORK/export.json" <<'PY'
import json, sys
v = json.load(open(sys.argv[1]))
assert v["workflows"] >= 1, v
assert v["event_count"] >= 1, v
PY
pass "chain verifies and the export bundle is written"

echo "== 3. replay reproduces the run =="
"$BIN" replay "$BUNDLE" --json > "$WORK/replay.json"
python3 - "$WORK/replay.json" <<'PY'
import json, sys
v = json.load(open(sys.argv[1]))
assert v["verified"] is True, v
assert v["manifest_sha256_ok"] is True, v
assert all(e["deterministic_repeat_match"] for e in v["executions"]), v
PY
pass "bundle replay is deterministic and hash-checked"

echo "== 4. tampered bundle is refused =="
WF="$BUNDLE/workflows/wing-baseline.awl.yaml"
cp "$WF" "$WORK/original.awl.yaml"
printf '\n# tampered after export\n' >> "$WF"
if "$BIN" replay "$BUNDLE" --json > "$WORK/tamper.json" 2>/dev/null; then
    fail "tampered bundle was accepted"
fi
cp "$WORK/original.awl.yaml" "$WF"
"$BIN" replay "$BUNDLE" --json > "$WORK/replay-restored.json"
python3 - "$WORK/replay-restored.json" <<'PY'
import json, sys
v = json.load(open(sys.argv[1]))
assert v["verified"] is True, v
PY
pass "tampering is detected and the restored bundle replays"

echo "== 5. documented restore drill =="
python3 -m pytest -q "$ROOT/python/aether_testing/tests/test_dr_drill.py"
pass "DR drill fixture passes"

echo
echo "Phase 7 partial acceptance complete."
echo "Open: plugin sandbox, federation, signed production release, SLOs."
