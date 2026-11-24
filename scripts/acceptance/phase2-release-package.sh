#!/usr/bin/env bash
# Phase 1/2 acceptance: evidence-native release package.
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

echo "== 1. capture the wing-box configuration snapshot =="
"$BIN" config snapshot "$ROOT/examples/wingbox/configuration.asm.yaml" \
    --output "$WORK/snapshot.json" --json > /dev/null
python3 - "$ROOT/examples/wingbox/release-prepare.yaml" "$WORK/snapshot.json" "$WORK/release.yaml" <<'PY'
import sys, yaml
doc = yaml.safe_load(open(sys.argv[1]))
doc["snapshot"] = sys.argv[2]
yaml.safe_dump(doc, open(sys.argv[3], "w"), sort_keys=False)
PY
pass "snapshot captured"

echo "== 2. a complete package is approved =="
"$BIN" release prepare "$WORK/release.yaml" --approver quality-auditor \
    --output "$WORK/release.json" --json > "$WORK/prepare.json"
python3 - "$WORK/prepare.json" "$WORK/release.json" <<'PY'
import json, sys
v = json.load(open(sys.argv[1]))
assert v["gates_pass"] is True, v
assert v["approval"]["error"] is None, v
assert v["approval"]["state"] == "approved", v
rel = json.load(open(sys.argv[2]))
assert rel["package"]["approved_by"] == "quality-auditor", rel
assert rel["package"]["state"] == "approved", rel
PY
pass "release package binds the snapshot, evidence, and approver"

echo "== 3. missing evidence blocks approval =="
python3 - "$WORK/release.yaml" "$WORK/incomplete.yaml" <<'PY'
import sys, yaml
doc = yaml.safe_load(open(sys.argv[1]))
doc["evidence"] = [e for e in doc["evidence"] if e["class"] != "validation"]
yaml.safe_dump(doc, open(sys.argv[2], "w"), sort_keys=False)
PY
if "$BIN" release prepare "$WORK/incomplete.yaml" --approver quality-auditor --json > "$WORK/bad.json" 2>/dev/null; then
    fail "an incomplete release was approved"
fi
python3 - "$WORK/bad.json" <<'PY'
import json, sys
v = json.load(open(sys.argv[1]))
assert v["gates_pass"] is False, v
assert v["approval"]["error"], v
PY
pass "missing validation evidence blocks the release"

echo
echo "Release-package acceptance complete. No certification claim is made."