#!/usr/bin/env bash
# Acceptance: native desktop shell bundles and boots the same CLI boundary.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
export PATH="$HOME/.cargo/bin:$PATH"

fail() { echo "FAIL: $*" >&2; exit 1; }
pass() { echo "PASS: $*"; }

if [ "$(uname -s)" != "Darwin" ]; then
  echo "SKIP: desktop bundle is macOS-only"
  exit 0
fi

echo "== 1. bundle exists =="
if [ ! -x "$ROOT/dist/Aether.app/Contents/MacOS/Aether" ]; then
  AETHER_SKIP_PYTHON=1 bash "$ROOT/experimental/macos/macos-app.sh" >/dev/null 2>&1 \
    || fail "cannot build desktop bundle"
fi
[ -f "$ROOT/dist/Aether.app/Contents/Resources/bin/aether" ] || fail "bundled CLI missing"
[ -d "$ROOT/dist/Aether.app/Contents/Resources/aether/contracts" ] || fail "bundled contracts missing"
pass "bundle contains shell, CLI and contracts"

echo "== 2. smoke test fetches the atlas =="
OUT=$("$ROOT/dist/Aether.app/Contents/MacOS/Aether" --smoke-test --project "$ROOT")
echo "$OUT" | python3 -c "
import json,sys
d=json.load(sys.stdin)
assert len(d['schema']['node_types'])==78, len(d['schema']['node_types'])
assert d['status']['certification_claim'] is False
" || fail "desktop smoke atlas"
pass "in-process server serves the canonical atlas"

echo "== 3. no token is persisted =="
if find "$ROOT/dist/Aether.app" -name '*token*' | grep -q .; then
  fail "token material found in bundle"
fi
pass "per-launch token stays in memory"

echo
echo "Desktop acceptance complete. CLI remains untouched."
