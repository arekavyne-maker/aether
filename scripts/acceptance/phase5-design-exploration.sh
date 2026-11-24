#!/usr/bin/env bash
# Phase 5 acceptance: design-space feasibility and candidate lineage.
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

echo "== 1. a valid candidate passes the full pipeline =="
"$BIN" design evaluate "$ROOT/examples/wingbox/candidate-pass.yaml" --json > "$WORK/pass.json"
python3 - "$WORK/pass.json" <<'PY'
import json, sys
v = json.load(open(sys.argv[1]))["report"]
assert v["accepted"] is True, v
assert len(v["gates"]) == 10, v
assert v["rejection"] is None, v
PY
pass "feasibility pipeline accepts the reviewed candidate"

echo "== 2. a bad candidate is rejected at the first failing gate =="
if "$BIN" design evaluate "$ROOT/examples/wingbox/candidate-reject.yaml" --json > "$WORK/reject.json" 2>/dev/null; then
    fail "an infeasible candidate was accepted"
fi
python3 - "$WORK/reject.json" <<'PY'
import json, sys
v = json.load(open(sys.argv[1]))["report"]
assert v["accepted"] is False, v
assert v["rejection"]["gate"] == "feature_tolerance", v
assert v["rejection"]["reason_code"] == "MIN_FEATURE_TOO_SMALL", v
assert len(v["gates"]) < 10, v
PY
pass "rejection stops at the first gate and keeps a reason code"

echo "== 3. candidate lineage is acyclic =="
"$BIN" design lineage "$ROOT/examples/wingbox/lineage.yaml" --json > "$WORK/lineage.json"
python3 - "$WORK/lineage.json" <<'PY'
import json, sys
v = json.load(open(sys.argv[1]))
assert v["valid"] is True, v
assert len(v["candidates"]) == 2, v
assert all(len(c["content_hash"]) == 64 for c in v["candidates"]), v
PY
pass "parentage and content hashes are reported"

echo "== 4. a lineage cycle is refused =="
if "$BIN" design lineage "$ROOT/examples/wingbox/lineage-cycle.yaml" --json > "$WORK/cycle.json" 2>/dev/null; then
    fail "a cyclic lineage was accepted"
fi
pass "cyclic lineage is rejected"

echo "== 5. uncertainty classes stay separate =="
python3 - "$ROOT/examples/wingbox/candidate-pass.yaml" <<'PY'
import sys, yaml
doc = yaml.safe_load(open(sys.argv[1]))
classes = [u["class"] for u in doc["candidate"]["uncertainty"]]
assert classes == ["material", "manufacturing"], classes
PY
pass "material and manufacturing uncertainty are distinct classes"

echo
echo "Phase 5 design-exploration acceptance complete."
echo "Search methods are pluggable; no candidate is a certification claim."