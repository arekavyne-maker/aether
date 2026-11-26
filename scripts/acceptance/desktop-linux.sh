#!/usr/bin/env bash
# Acceptance: Linux desktop bundle boots the embedded console.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
export PATH="$HOME/.cargo/bin:$PATH"
fail() { echo "FAIL: $*" >&2; exit 1; }
pass() { echo "PASS: $*"; }

if [ "$(uname -s)" != "Linux" ]; then
  echo "SKIP: Linux bundle is only tested on Linux"
  exit 0
fi

BUNDLE="$ROOT/dist/linux/Aether"
echo "== 1. Linux bundle exists =="
if [ ! -x "$BUNDLE/bin/Aether" ]; then
  AETHER_SKIP_PYTHON=1 bash "$ROOT/experimental/linux/linux-app.sh" >/dev/null 2>&1 \
    || fail "cannot build Linux bundle"
fi
[ -x "$BUNDLE/bin/Aether" ] || fail "desktop shell missing"
[ -x "$BUNDLE/share/bin/aether" ] || fail "bundled CLI missing"
[ -d "$BUNDLE/share/aether/contracts" ] || fail "bundled contracts missing"
pass "bundle contains shell, CLI and contracts"

echo "== 2. smoke test =="
OUT=$("$BUNDLE/bin/Aether" --smoke-test --project "$ROOT")
echo "$OUT" | python3 -c "
import json,sys
d=json.load(sys.stdin)
assert len(d['schema']['node_types'])==78
assert d['status']['certification_claim'] is False
" || fail "desktop smoke"
pass "in-process server serves the canonical atlas"

echo "== 3. no token is persisted =="
if find "$BUNDLE" -name '*token*' | grep -q .; then fail "token material found in bundle"; fi
pass "per-launch token stays in memory"

echo
echo "Linux desktop acceptance complete. CLI remains untouched."
