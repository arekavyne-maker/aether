#!/usr/bin/env bash
# Phase B acceptance: verification and validation core.
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

echo "== 1. an incomplete package reports honest coverage =="
if "$BIN" vv coverage "$ROOT/contracts/vv/examples/wingbox.requirements-incomplete.yaml" --json > "$WORK/cov.json" 2>&1; then
    fail "an unverified requirement was reported as covered"
fi
python3 - "$WORK/cov.json" <<'PY'
import json, sys
v = json.load(open(sys.argv[1]))
assert v["requirements_total"] == 2, v
assert v["requirements_verified"] == 1, v
assert v["requirements_uncased"] == 1, v
assert v["gaps"][0]["reason_code"] == "NO_VERIFICATION_CASE", v
assert "not certification" in v["claim"], v
PY
pass "unverified requirement is a gap, and the claim is disclaimed"

echo "== 2. a full package verifies =="
"$BIN" vv coverage "$ROOT/contracts/vv/examples/wingbox.requirements.yaml" --json > "$WORK/full.json"
python3 - "$WORK/full.json" <<'PY'
import json, sys
v = json.load(open(sys.argv[1]))
assert v["requirements_verified"] == 2, v
assert v["requirements_total"] == 2, v
assert v["gaps"] == [], v
PY
pass "current evidence satisfies both requirements"

echo "== 3. an unauthorized approver is rejected =="
if "$BIN" vv decisions "$ROOT/contracts/vv/examples/wingbox.decisions.yaml" --json > "$WORK/dec.json" 2>&1; then
    fail "an unauthorized approval was accepted"
fi
python3 - "$WORK/dec.json" <<'PY'
import json, sys
v = json.load(open(sys.argv[1]))
assert v["decisions_accepted"] == 1, v
assert v["decisions_rejected"], v
assert "unauthorized" in v["decisions_rejected"][0]["error"], v
PY
pass "an analyst cannot approve a safety-significant requirement"

echo
echo "Phase B acceptance complete. Coverage is not certification."