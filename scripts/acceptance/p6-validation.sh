#!/usr/bin/env bash
# P6 acceptance: benchmark registry, fault injection and security tests.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
export PATH="$HOME/.cargo/bin:$PATH"

fail() { echo "FAIL: $*" >&2; exit 1; }
pass() { echo "PASS: $*"; }

echo "== 1. benchmark registry covers §17.2 and §21 =="
python3 "$ROOT/infra/benchmarks_registry.py" --list | python3 -c "
import json,sys
d=json.load(sys.stdin)
assert len(d['scientific'])==11 and len(d['performance'])==12, d
print('11 scientific, 12 performance')
" || fail "benchmark registry incomplete"
pass "all 23 benchmark categories declared"

echo "== 2. benchmark coverage fitness =="
python3 "$ROOT/infra/fitness/run.py" benchmark_coverage >/dev/null || fail "benchmark coverage"
pass "benchmark coverage check passes"

echo "== 3. fault injection and security tests =="
(cd "$ROOT" && cargo test -p aether_cli --test security_faults --quiet >/dev/null 2>&1) || fail "Rust security tests"
(cd "$ROOT" && python3 -m pytest -q python/aether_testing/tests/test_security.py >/dev/null 2>&1) || fail "Python security tests"
pass "prompt injection, malformed parsers and oversized requests fail closed"

echo "== 4. full suite =="
(cd "$ROOT" && cargo test --workspace --quiet >/dev/null 2>&1) || fail "cargo workspace"
(cd "$ROOT" && python3 -m pytest -q >/dev/null 2>&1) || fail "pytest"
python3 "$ROOT/infra/fitness/run.py" >/dev/null || fail "fitness"
pass "cargo, pytest and fitness green"

echo
echo "P6 acceptance complete."