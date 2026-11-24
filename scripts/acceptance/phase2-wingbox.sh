#!/usr/bin/env bash
# Phase 2 acceptance: metallic wing-box vertical slice.
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

echo "== 1. structural baseline =="
"$BIN" demo structure --json > "$WORK/structure.json"
python3 - "$WORK/structure.json" <<'PY'
import json, sys
v = json.load(open(sys.argv[1]))
assert v["success"] is True, v
PY
pass "beam FEM baseline matched its closed-form check"

echo "== 2. configuration snapshot =="
"$BIN" config snapshot "$ROOT/examples/wingbox/configuration.asm.yaml" \
    --output "$WORK/snapshot.json" --json > "$WORK/snapshot-run.json"
"$BIN" config verify "$WORK/snapshot.json" \
    "$ROOT/examples/wingbox/configuration.asm.yaml" --json > "$WORK/verify.json"
python3 - "$WORK/verify.json" <<'PY'
import json, sys
v = json.load(open(sys.argv[1]))
assert v["valid"] is True, v
PY
pass "snapshot reconstructs the exact released revisions"

echo "== 3. changed object set is rejected =="
sed 's/thickness_mm: 2.4/thickness_mm: 2.9/' \
    "$ROOT/examples/wingbox/configuration.asm.yaml" > "$WORK/changed.asm.yaml"
if "$BIN" config verify "$WORK/snapshot.json" "$WORK/changed.asm.yaml" --json > "$WORK/changed.json" 2>/dev/null; then
    fail "changed object set was accepted"
fi
pass "changed input invalidates the snapshot"

echo "== 4. release gate =="
"$BIN" evidence gate "$ROOT/examples/wingbox/release-gate.yaml" --json > "$WORK/gate.json"
python3 - "$WORK/gate.json" <<'PY'
import json, sys
v = json.load(open(sys.argv[1]))
assert v["passed"] is True, v
assert v["waived_by"] is None, v
PY
pass "complete, credible evidence passes the gate"

echo "== 5. missing evidence fails closed =="
python3 - "$ROOT/examples/wingbox/release-gate.yaml" "$WORK/incomplete.yaml" <<'PY'
import sys, yaml
doc = yaml.safe_load(open(sys.argv[1]))
doc["evidence"] = [e for e in doc["evidence"] if e["class"] != "validation"]
yaml.safe_dump(doc, open(sys.argv[2], "w"), sort_keys=False)
PY
if "$BIN" evidence gate "$WORK/incomplete.yaml" --json > "$WORK/incomplete.json" 2>/dev/null; then
    fail "incomplete evidence passed the gate"
fi
python3 - "$WORK/incomplete.json" <<'PY'
import json, sys
v = json.load(open(sys.argv[1]))
assert v["passed"] is False, v
assert v["missing"], v
PY
pass "missing validation evidence blocks release"

echo "== 6. validation report separates coverage from gaps =="
"$BIN" init "$WORK/project" --profile aerospace >/dev/null
"$BIN" requirement add --text "Wing-box tip deflection shall not exceed 0.5 m" \
    --verification-method analysis --criticality major \
    --project "$WORK/project" --json >/dev/null
"$BIN" validation report --project "$WORK/project" --json > "$WORK/validation.json"
python3 - "$WORK/validation.json" <<'PY'
import json, sys
v = json.load(open(sys.argv[1]))
assert v["requirements_total"] == 1, v
assert v["requirements_verified"] == 0, v
assert v["gaps"], v
assert v["claim"].startswith("coverage only"), v
PY
pass "unverified requirement is reported as a gap, not as success"

echo
echo "Phase 2 acceptance complete. No certification claim is made."
