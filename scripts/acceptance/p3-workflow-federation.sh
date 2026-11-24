#!/usr/bin/env bash
# P3 acceptance: workflow features, job envelope, federation protocol.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
export PATH="$HOME/.cargo/bin:$PATH"

fail() { echo "FAIL: $*" >&2; exit 1; }
pass() { echo "PASS: $*"; }

echo "== 1. workflow control flow and new features =="
(cd "$ROOT" && cargo test -p aether_workflow --quiet >/dev/null 2>&1) || fail "workflow tests"
pass "branch, join, loop, approval, nested, cache, trigger, idempotency"

echo "== 2. portable job envelope =="
(cd "$ROOT" && cargo test -p aether_job --quiet >/dev/null 2>&1) || fail "job envelope tests"
pass "job envelope validates hashes and idempotency keys"

echo "== 3. federation protocol fails closed =="
(cd "$ROOT" && cargo test -p aether_federation --quiet >/dev/null 2>&1) || fail "federation tests"
pass "untrusted sites, bad checksums and over-classified transfers rejected"

echo "== 4. §20 canonical contracts present =="
for c in analysis.run design.candidate environment.profile job.envelope credibility.record release.gate; do
    [ -f "$ROOT/contracts/$c/manifest.json" ] || fail "missing contract $c"
done
pass "six canonical contracts registered"

echo
echo "P3 acceptance complete. Federation is a harness until a real second site exists."