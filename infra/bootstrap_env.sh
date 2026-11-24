#!/usr/bin/env bash
# Aether scientific toolchain bootstrap.
#
# Installs the pinned Python stack the reference plugins and fitness checks
# need, verifies that the optional native tools resolve, and writes a lock
# file recording what is actually present.
#
# Usage:
#   ./infra/bootstrap_env.sh            # verify + install
#   ./infra/bootstrap_env.sh --check    # verify only, no network
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PY="${PYTHON:-python3}"
CHECK_ONLY=0
[[ "${1:-}" == "--check" ]] && CHECK_ONLY=1

echo "== Aether toolchain bootstrap =="
echo "   repo:    $ROOT"
echo "   python:  $($PY --version 2>&1)"

if [[ $CHECK_ONLY -eq 0 ]]; then
    echo "== installing pinned requirements =="
    "$PY" -m pip install --quiet -r "$ROOT/requirements-dev.txt"
fi

echo "== verifying imports =="
fail=0
for mod in numpy scipy gmsh meshio trimesh OCP; do
    if "$PY" -c "import $mod" 2>/dev/null; then
        echo "   OK   $mod"
    else
        echo "   MISS $mod"
        fail=1
    fi
done

echo "== verifying native tools =="
CCX="${CCX_BIN:-$ROOT/vendor/bin/ccx_2.23}"
if [[ -x "$CCX" ]]; then
    # CalculiX returns non-zero for -v (201 = version banner), so probe it
    # with a real single-element deck and look for the "Job finished" line.
    CCXPROBE="$(mktemp -d)"
    cat > "$CCXPROBE/probe.inp" <<'INP'
*NODE, NSET=NALL
1, 0.0, 0.0, 0.0
2, 1.0, 0.0, 0.0
3, 1.0, 0.1, 0.0
4, 0.0, 0.1, 0.0
*ELEMENT, TYPE=S4, ELSET=EALL
1, 1, 2, 3, 4
*MATERIAL, NAME=STEEL
*ELASTIC
210000.0, 0.3
*SHELL SECTION, ELSET=EALL, MATERIAL=STEEL
0.1
*BOUNDARY
1,1,3,0.0
*STEP
*STATIC
*CLOAD
2,2,1000.0
*NODE FILE
U
*END STEP
INP
    export DYLD_LIBRARY_PATH="$ROOT/vendor/lib${DYLD_LIBRARY_PATH:+:$DYLD_LIBRARY_PATH}"
    if "$CCX" -i "$CCXPROBE/probe" 2>&1 | grep -q "Job finished"; then
        echo "   OK   CalculiX 2.23 (real solve)"
    else
        echo "   MISS CalculiX at $CCX (did not finish a probe solve)"
        fail=1
    fi
    rm -rf "$CCXPROBE"
else
    echo "   SKIP CalculiX (not present; BENCH-08 will report SKIP)"
fi

echo "== writing toolchain lock =="
LOCK="$ROOT/infra/fixtures/env/toolchain.lock"
mkdir -p "$(dirname "$LOCK")"
{
    echo "aether_toolchain: \"1.0\""
    echo "python: \"$($PY --version 2>&1 | awk '{print $2}')\""
    echo "platform: \"$(uname -s | tr '[:upper:]' '[:lower:]')-$(uname -m)\""
    echo "packages:"
    "$PY" - "$CHECK_ONLY" <<'PYEOF'
import importlib, sys
mods = ["numpy", "scipy", "gmsh", "meshio", "trimesh", "OCP"]
for m in mods:
    try:
        mod = importlib.import_module(m)
        v = getattr(mod, "__version__", "unknown")
        print(f'  {m}: "{v}"')
    except Exception as exc:
        print(f'  {m}: null  # {type(exc).__name__}')
PYEOF
    if [[ -x "$CCX" ]]; then
        echo "calculix:"
        echo "  binary: \"vendor/bin/ccx_2.23\""
        echo "  version: \"2.23\""
        echo "  libs: \"vendor/lib\""
    else
        echo "calculix: null"
    fi
} > "$LOCK"

echo "   wrote $LOCK"

if [[ $fail -ne 0 ]]; then
    echo
    echo "toolchain incomplete; see the MISS lines above" >&2
    exit 1
fi

echo
echo "toolchain OK"