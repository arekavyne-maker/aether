#!/usr/bin/env bash
# Fetch a pinned, relocatable CPython for the desktop bundle.
# Runtime stays air-gapped; this script is a build-time step only.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUT="${1:-$ROOT/vendor/python}"
VERSION="${AETHER_PYTHON_VERSION:-3.12.9}"
TAG="${AETHER_PYTHON_BUILD_TAG:-20250214}"
OS="$(uname -s)"
ARCH="$(uname -m)"
case "$OS-$ARCH" in
  Darwin-arm64|Darwin-aarch64) TARGET="aarch64-apple-darwin" ;;
  Darwin-x86_64) TARGET="x86_64-apple-darwin" ;;
  Linux-x86_64|Linux-amd64) TARGET="x86_64-unknown-linux-gnu" ;;
  Linux-arm64|Linux-aarch64) TARGET="aarch64-unknown-linux-gnu" ;;
  MINGW*-x86_64|MSYS*-x86_64|CYGWIN*-x86_64) TARGET="x86_64-pc-windows-msvc" ;;
  *) echo "unsupported platform: $OS-$ARCH" >&2; exit 1 ;;
esac
NAME="cpython-${VERSION}+${TAG}-${TARGET}-install_only.tar.gz"
URL="${AETHER_PYTHON_URL:-https://github.com/astral-sh/python-build-standalone/releases/download/${TAG}/${NAME}}"

mkdir -p "$OUT"
ARCHIVE="$OUT/$NAME"
if [ ! -f "$ARCHIVE" ]; then
  echo "==> fetching $URL"
  curl -fL --retry 3 -o "$ARCHIVE" "$URL"
fi

# First fetch records the hash; later builds verify the same artefact.
SUMS="$OUT/SHA256SUMS"
if [ -f "$SUMS" ]; then
  (cd "$OUT" && shasum -a 256 -c "$SUMS" --ignore-missing) || {
    echo "FAIL: bundled Python hash does not match $SUMS" >&2; exit 1;
  }
else
  (cd "$OUT" && shasum -a 256 "$NAME" > "$SUMS")
fi

if [ ! -x "$OUT/bin/python3" ] && [ ! -x "$OUT/python.exe" ]; then
  echo "==> extracting relocatable Python"
  tar -xzf "$ARCHIVE" -C "$OUT" --strip-components=1
fi
if [ -x "$OUT/python.exe" ]; then
  "$OUT/python.exe" --version
else
  "$OUT/bin/python3" --version
fi
