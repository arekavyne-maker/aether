#!/usr/bin/env bash
# Phase 3 acceptance: manufacturing genealogy and as-built traceability.
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

echo "== 1. complete genealogy verifies =="
"$BIN" manufacturing verify "$ROOT/examples/wingbox/genealogy.yaml" --json > "$WORK/verify.json"
python3 - "$WORK/verify.json" <<'PY'
import json, sys
v = json.load(open(sys.argv[1]))
assert v["valid"] is True, v
assert v["builds_checked"] == 3, v
PY
pass "three builds trace to material lots, inspections, and dispositions"

echo "== 2. incomplete genealogy fails closed =="
if "$BIN" manufacturing verify "$ROOT/examples/wingbox/genealogy-incomplete.yaml" --json > "$WORK/bad.json" 2>/dev/null; then
    fail "incomplete genealogy was accepted"
fi
python3 - "$WORK/bad.json" <<'PY'
import json, sys
v = json.load(open(sys.argv[1]))
assert v["valid"] is False, v
codes = {i["code"] for i in v["issues"]}
assert "missing_material_lot" in codes, codes
assert "failed_inspection_unresolved" in codes, codes
PY
pass "missing material and unresolved failure are reported"

echo "== 3. as-built snapshot is reproducible =="
"$BIN" manufacturing as-built "$ROOT/examples/wingbox/genealogy.yaml" \
    --part rib-07 --json > "$WORK/asbuilt-a.json"
"$BIN" manufacturing as-built "$ROOT/examples/wingbox/genealogy.yaml" \
    --part rib-07 --json > "$WORK/asbuilt-b.json"
python3 - "$WORK/asbuilt-a.json" "$WORK/asbuilt-b.json" <<'PY'
import json, sys
a = json.load(open(sys.argv[1]))
b = json.load(open(sys.argv[2]))
assert a["manifest_hash"] == b["manifest_hash"], (a, b)
assert len(a["entries"]) == 3, a
PY
pass "as-built snapshot hashes the same input identically"

echo "== 4. revision impact is explained =="
"$BIN" manufacturing impact "$ROOT/examples/wingbox/genealogy.yaml" \
    --part rib-07 --to-revision 3 --json > "$WORK/impact.json"
python3 - "$WORK/impact.json" <<'PY'
import json, sys
v = json.load(open(sys.argv[1]))
assert v["affected_builds"] == ["build-rib07-001", "build-rib07-002", "build-rib07-003"], v
assert len(v["revalidation_required"]) == 3, v
assert "predate revision 3" in v["reason"], v
PY
pass "impact names every affected build and its revalidation need"

echo
echo "Phase 3 acceptance complete. No machine control is claimed."
