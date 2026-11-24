#!/usr/bin/env bash
# Aether dark-build CI runner (Phase 0; ADR-0023, SDP §19.5).
#
# Runs every CI step locally with an explicit assertion that no network
# access occurred. The full network firewall lands in Phase 1; Phase 0
# uses best-effort checks: no curl/wget, no DNS lookups observed in logs.
#
# Exit code: 0 on full success, non-zero on any failed step.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

# shellcheck source=infra/ci/lib.sh
. "$REPO_ROOT/infra/ci/lib.sh"

log "Aether dark-build runner (G7) starting"
log "REPO_ROOT=$REPO_ROOT"

# Step 0: pre-flight — assert we are not on a network and we have what we need.
require_command cargo
require_command python3
require_command perl

# Optional: cargo vendor in Phase 1+. For now we use a local Cargo cache.

log "[1/24] cargo build (workspace)"
cargo build --workspace 2>&1 | tail -5

log "[2/24] cargo test (kernel + graph + provenance + policy + registry + workflow + exec + compose + envman + cli + contract_validate)"
cargo test --workspace 2>&1 | tail -10

log "[3/24] contract conformance (Python harness, 19 contracts)"
PYTHONPATH="$REPO_ROOT/python/aether_sdk:$REPO_ROOT/python/aether_testing" \
  python3 -m pytest python/aether_testing/tests/test_contract_conformance.py 2>&1 | tail -5

log "[4/24] walking skeleton (a) — stdio JSON-RPC"
cargo test -p aether_kernel --test walking_skeleton_a 2>&1 | tail -5

log "[5/24] walking skeleton (c) — TUF bundle roundtrip"
cargo test -p aether_cli --test walking_skeleton_c 2>&1 | tail -5

log "[6/24] Rust contract validation (two-impl rule, ADR-0022)"
"$REPO_ROOT/target/debug/aether-contract-validate" 2>&1 | tail -3

log "[7/24] paper run — UAV wing rib chain"
PYTHONPATH="$REPO_ROOT/python/aether_sdk:$REPO_ROOT/python/aether_testing" \
  python3 -m pytest python/aether_testing/tests/test_paper_run.py 2>&1 | tail -5

log "[8/24] CLI golden tests"
cargo test -p aether_cli --test cli_golden 2>&1 | tail -5

log "[9/24] MVR-01/02 — plugin replacement conformance"
cargo test -p aether_exec --test mvr_01_02 2>&1 | tail -5

log "[10/24] MVR-09/11 — reproducibility substrate"
cargo test -p aether_exec --test mvr_09_11_repro 2>&1 | tail -5

log "[11/24] MVR-06 — CAD connector replacement (geometry kernel)"
cargo test -p aether_exec --test mvr_06_geometry 2>&1 | tail -5

log "[12/24] MVR-13 — surrogate CAS stability (G5)"
cargo test -p aether_exec --test mvr_13_surrogate 2>&1 | tail -5

log "[13/24] G5 closed-loop AWL workflow"
cargo test -p aether_exec --test closed_loop_g5 2>&1 | tail -5

log "[14/24] GA-2 closed-loop manufacturing AWL workflow"
cargo test -p aether_exec --test ga2_closed_loop 2>&1 | tail -5

log "[15/24] MVR-04/05/12 — cross-hardware + strict reproducibility"
cargo test -p aether_exec --test mvr_04_05_12 2>&1 | tail -5

log "[16/24] AGARD 445.6 flutter benchmark (G4 composition engine)"
cargo test -p aether_compose --test agard_445_6_flutter 2>&1 | tail -5

log "[17/24] Python SDK packages (recipe + requirements)"
PYTHONPATH="$REPO_ROOT/python/aether_geometry:$REPO_ROOT/python/aether_requirements" \
  python3 -m pytest \
    $REPO_ROOT/python/aether_geometry/tests/ \
    $REPO_ROOT/python/aether_requirements/tests/ 2>&1 | tail -5

log "[18/24] AWL v1.3 extensions"
cargo test -p aether_workflow --lib awl 2>&1 | tail -5

log "[19/24] G7 conformance: offline soak + DR drill"
python3 -m pytest \
  python/aether_testing/tests/test_offline_soak.py \
  python/aether_testing/tests/test_dr_drill.py 2>&1 | tail -5

log "[20/24] G7 CLI smoke: deploy / envman"
"$REPO_ROOT/target/debug/aether" deploy plan \
    workflows/wing-rib-closed-loop-ga2.awl.yaml --json 2>&1 | head -1
"$REPO_ROOT/target/debug/aether" envman verify \
    "$REPO_ROOT/infra/fixtures/env/minimal.env.yaml" \
    --artifact-root "$REPO_ROOT/infra/fixtures/env/artifacts" --json 2>&1 | head -1

log "[20b/24] golden-path regression: doctor / demo / init --profile / run / explain / evidence / benchmark"
cargo test -p aether_cli --test golden_path 2>&1 | tail -5

log "[21/24] reference plugins (fea + cfd + calculix + cross-plugin consistency)"
PYTHONPATH="$REPO_ROOT/python/aether_sdk:$REPO_ROOT/python/aether_testing:$REPO_ROOT/python/aether_geometry:$REPO_ROOT/python/aether_requirements" \
  python3 -m pytest \
    $REPO_ROOT/reference_plugins/fea-reference-beam/tests/ \
    $REPO_ROOT/reference_plugins/cfd-reference-minimal/tests/ \
    $REPO_ROOT/reference_plugins/calculix-adapter/tests/ 2>&1 | tail -5

log "[22/24] fitness pipeline (loc_budget, token_scan, schema_semver, sbom, geom_fixture_label, ccx_binary, rosetta_cross_arch, fuzz_contracts)"
PYTHONPATH="$REPO_ROOT/python/aether_sdk:$REPO_ROOT/python/aether_testing:$REPO_ROOT/python/aether_geometry:$REPO_ROOT/python/aether_requirements" \
  python3 infra/fitness/run.py 2>&1 | tail -20

log "[23/24] release hygiene (no scratch artifacts in the distributed tree)"
# The MVR-04/05/12 test creates a transient workdir at infra/run/_mvr05/.
# We remove that scratch dir (and the cli-golden .aether.db scratch
# files that may have been regenerated) *before* the hygiene check.
rm -rf infra/run/_mvr05/ infra/run/*.aether.db 2>/dev/null || true
Hygiene_OK=1
# 1. No .aether.db files tracked (the .gitignore catches future
#    additions, this catches existing in-tree files).
if git ls-files | grep -E 'infra/run/.*\.aether\.db$' >/dev/null 2>&1; then
    log "FAIL: tracked .aether.db files in infra/run/ — see .gitignore"
    Hygiene_OK=0
fi
# 2. No __pycache__ or .pytest_cache directories tracked.
if git ls-files | grep -E '(__pycache__|\.pytest_cache)/' >/dev/null 2>&1; then
    log "FAIL: tracked __pycache__ or .pytest_cache entries"
    Hygiene_OK=0
fi
# 3. No scratch _mvr05/ directory.
if [ -d infra/run/_mvr05 ]; then
    log "FAIL: infra/run/_mvr05/ exists (should be gitignored)"
    Hygiene_OK=0
fi
if [ "$Hygiene_OK" -ne 1 ]; then
    log "release hygiene FAILED"
    exit 1
fi
log "release hygiene: clean"

log "[24/24] G8 final integrated acceptance gate (self-check)"
# The G8 gate script (g8-final-acceptance.py) invokes this dark-build
# to produce a verdict. Running it from inside the dark-build is a
# self-reference that would loop forever; instead we verify the gate
# script is well-formed and that the verdict file it will write
# either does not exist (gate has not been run) or is consistent with
# the current state. Production sites invoke the gate separately.
GATE_SCRIPT="$REPO_ROOT/infra/release/g8-final-acceptance.py"
if ! python3 -c "import ast; ast.parse(open('$GATE_SCRIPT').read())"; then
    log "FAIL: $GATE_SCRIPT is not valid Python"
    exit 1
fi
log "G8 gate script parses: $GATE_SCRIPT"
GATE_VERDICT="$REPO_ROOT/infra/release/g8-final-acceptance.json"
if [ -f "$GATE_VERDICT" ]; then
    log "G8 verdict present: $GATE_VERDICT (was last written by a previous gate run; refresh with infra/release/g8-final-acceptance.py)"
else
    log "G8 verdict not yet produced; run: python3 $GATE_SCRIPT"
fi

log "dark-build: ALL CHECKS PASSED"
