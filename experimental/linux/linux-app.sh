#!/usr/bin/env bash
# Build a portable Linux tree, tar.gz, optional .deb and AppImage.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
export PATH="$HOME/.cargo/bin:$PATH"
OUT="$ROOT/dist/linux"
BUNDLE="$OUT/Aether"
SKIP_PYTHON="${AETHER_SKIP_PYTHON:-0}"

echo "==> building canonical CLI (release)"
(cd "$ROOT" && cargo build --release -p aether_cli >/dev/null)
echo "==> building desktop shell (release)"
(cd "$ROOT" && cargo build --release --manifest-path experimental/desktop/Cargo.toml >/dev/null)

rm -rf "$OUT"
mkdir -p "$BUNDLE/bin" "$OUT/deb/DEBIAN" "$OUT/deb/opt/aether/bin"
bash "$ROOT/scripts/package/stage-resources.sh" "$BUNDLE/share" "$ROOT/target/release/aether"
cp "$ROOT/experimental/desktop/target/release/aether-desktop" "$BUNDLE/bin/Aether"
chmod +x "$BUNDLE/bin/Aether"

if [ "$SKIP_PYTHON" != "1" ]; then
  echo "==> installing bundled Python runtime"
  PYSTAGE="$OUT/python-stage"
  bash "$ROOT/scripts/package/fetch-python.sh" "$PYSTAGE"
  cp -R "$PYSTAGE" "$BUNDLE/share/python"
  if [ -d "$ROOT/vendor/wheels" ]; then
    SITE="$BUNDLE/share/python/lib/python3.12/site-packages"
    mkdir -p "$SITE"
    "$BUNDLE/share/python/bin/python3" -m pip install --no-index \
      --find-links "$ROOT/vendor/wheels" --target "$SITE" \
      --requirement "$ROOT/experimental/desktop/python-runtime.txt" || true
  fi
else
  echo "NOTE: Python bundle skipped"
fi

ICON="$ROOT/experimental/desktop/AppIcon.png"
[ -f "$ICON" ] && cp "$ICON" "$BUNDLE/share/aether.png"
cat > "$BUNDLE/share/Aether.desktop" <<DESKTOP
[Desktop Entry]
Type=Application
Name=Aether
Comment=Local engineering console
Exec=Aether
Icon=aether
Terminal=false
Categories=Development;Science;
DESKTOP

echo "==> smoke test"
"$BUNDLE/bin/Aether" --smoke-test --project "$ROOT" | python3 -c \
  "import json,sys; d=json.load(sys.stdin); assert d['schema']['node_types']; print('bundle smoke:', len(d['schema']['node_types']), 'node types')"

echo "==> archiving tar.gz"
TAR="$OUT/Aether-linux.tar.gz"
tar -czf "$TAR" -C "$OUT" Aether

echo "==> building .deb"
cat > "$OUT/deb/DEBIAN/control" <<CONTROL
Package: aether
Version: 3.0.0-rc1
Section: devel
Priority: optional
Architecture: amd64
Maintainer: Crocell Company
Description: Local engineering console and canonical Aether CLI
Offline, evidence-native engineering workspace with an embedded local UI.
CONTROL
mkdir -p "$OUT/deb/opt/aether"
cp -R "$BUNDLE/." "$OUT/deb/opt/aether/"
ln -sf /opt/aether/bin/Aether "$OUT/deb/usr/bin/aether-desktop" 2>/dev/null || {
  mkdir -p "$OUT/deb/usr/bin"; ln -sf /opt/aether/bin/Aether "$OUT/deb/usr/bin/aether-desktop";
}
DEB="$OUT/aether_3.0.0-rc1_amd64.deb"
if command -v dpkg-deb >/dev/null 2>&1; then
  dpkg-deb --build "$OUT/deb" "$DEB" >/dev/null
else
  echo "NOTE: dpkg-deb absent; .deb skipped"
fi

echo "==> building AppImage"
if command -v appimagetool >/dev/null 2>&1; then
  cp "$BUNDLE/share/Aether.desktop" "$BUNDLE/Aether.desktop"
  (cd "$OUT" && appimagetool Aether Aether-x86_64.AppImage >/dev/null) || echo "NOTE: AppImage build skipped"
else
  echo "NOTE: appimagetool absent; AppImage skipped"
fi

for artifact in "$TAR" "$DEB"; do
  [ -f "$artifact" ] && bash "$ROOT/scripts/package/sign-artifacts.sh" "$artifact"
done
for artifact in "$OUT"/*.AppImage; do
  [ -f "$artifact" ] && bash "$ROOT/scripts/package/sign-artifacts.sh" "$artifact"
done
echo "Built $BUNDLE"
