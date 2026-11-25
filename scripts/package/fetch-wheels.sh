#!/usr/bin/env bash
# Download pinned wheels for the bundled Python runtime into vendor/wheels.
# Run once on a connected build machine; packaging then works offline.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DEFAULT_PYTHON="$ROOT/vendor/python/bin/python3"
[ -x "$DEFAULT_PYTHON" ] || DEFAULT_PYTHON="$ROOT/vendor/python/python.exe"
PYTHON="${AETHER_BUNDLE_PYTHON:-$DEFAULT_PYTHON}"
WHEELS="${AETHER_WHEELHOUSE:-$ROOT/vendor/wheels}"
REQUIREMENTS="$ROOT/apps/desktop/python-runtime.txt"

[ -x "$PYTHON" ] || { echo "run scripts/package/fetch-python.sh first" >&2; exit 1; }
mkdir -p "$WHEELS"
echo "==> downloading runtime wheels into $WHEELS"
"$PYTHON" -m pip download --only-binary=:all: --requirement "$REQUIREMENTS" \
  --dest "$WHEELS"
echo "==> recording wheel hashes"
(cd "$WHEELS" && shasum -a 256 *.whl > SHA256SUMS)
