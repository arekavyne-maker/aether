#!/usr/bin/env bash
# Shared resource staging for every desktop bundle.
# Usage: stage-resources.sh <resources-dir> <cli-binary> [python-runtime-dir]
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
RES="${1:?resources directory required}"
CLI="${2:?CLI binary required}"
PYTHON="${3:-}"

mkdir -p "$RES/aether" "$RES/bin" "$RES/licenses"
cp "$CLI" "$RES/bin/$(basename "$CLI")"
chmod +x "$RES/bin/$(basename "$CLI")" || true
for item in contracts reference_plugins python examples; do
  [ -e "$ROOT/$item" ] && cp -R "$ROOT/$item" "$RES/aether/$item"
done
for license in LICENSE NOTICE; do
  [ -f "$ROOT/$license" ] && cp "$ROOT/$license" "$RES/licenses/$license"
done
if [ -n "$PYTHON" ] && [ -d "$PYTHON" ]; then
  rm -rf "$RES/python"
  cp -R "$PYTHON" "$RES/python"
fi
echo "staged resources in $RES"
