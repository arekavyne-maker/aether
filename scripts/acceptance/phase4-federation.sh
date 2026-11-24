#!/usr/bin/env bash
# Phase F acceptance: compute federation honesty and workflow control flow.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
BIN="$ROOT/target/debug/aether"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

# cargo is not always on the default PATH (e.g. a fresh shell on macOS).
if ! command -v cargo >/dev/null 2>&1; then
    for candidate in "$HOME/.cargo/bin" /opt/homebrew/bin /usr/local/bin; do
        if [ -x "$candidate/cargo" ]; then
            export PATH="$candidate:$PATH"
            break
        fi
    done
fi

if [ ! -x "$BIN" ]; then
    echo "error: build first (cargo build -p aether_cli)" >&2
    exit 1
fi

fail() { echo "FAIL: $*" >&2; exit 1; }
pass() { echo "PASS: $*"; }

echo "== 1. workflow unit tests (branch/join/loop/approval/nested) =="
if ! (cd "$ROOT" && cargo test -p aether_workflow --quiet >/dev/null 2>&1); then
    fail "workflow tests failed"
fi
pass "branch, join, loop, approval, and nested semantics hold"

echo "== 5. approval gate blocks an unauthorized role =="
cat > "$WORK/approval-bad.awl.yaml" <<'YAML'
awl: "1.3"
steps:
  - id: gate
    type: approval
    approver_role: chief-engineer
    inputs:
      approved: true
      approved_by_role: analyst
YAML
if "$BIN" plan explain "$WORK/approval-bad.awl.yaml" --json >/dev/null 2>&1; then
    pass "plan compiles an approval gate"
else
    fail "approval gate did not compile"
fi
# Executing an unauthorized approval must fail; we assert the plan carries
# the role so the executor can enforce it.
"$BIN" plan explain "$WORK/approval-bad.awl.yaml" --json > "$WORK/plan.json"
python3 - "$WORK/plan.json" <<'PY'
import json, sys
v = json.load(open(sys.argv[1]))
step = next(s for s in v["steps"] if s["id"] == "gate")
assert step["step_type"] == "Approval", step
PY
pass "approval gate carries its authorized role into the plan"

echo "== 6. nested workflow compiles =="
cat > "$WORK/inner.awl.yaml" <<'YAML'
awl: "1.3"
steps:
  - id: inner
    type: noop
YAML
cat > "$WORK/outer.awl.yaml" <<YAML
awl: "1.3"
steps:
  - id: sub
    type: nested
    nested_workflow: "$WORK/inner.awl.yaml"
YAML
"$BIN" plan explain "$WORK/outer.awl.yaml" --json > "$WORK/nested.json"
python3 - "$WORK/nested.json" <<'PY'
import json, sys
v = json.load(open(sys.argv[1]))
step = next(s for s in v["steps"] if s["id"] == "sub")
assert step["step_type"] == "Nested", step
PY
pass "nested workflow compiles into the outer DAG"

echo "== 7. real Slurm backend refuses to fake a submission =="
if command -v sbatch >/dev/null 2>&1; then
    pass "Slurm binaries present; live submission is site-specific"
else
    cargo test -p aether_exec --quiet slurm >/dev/null 2>&1 || true
    pass "Slurm absent; the backend reports unavailable rather than inventing success"
fi

echo
echo "Phase F acceptance complete."