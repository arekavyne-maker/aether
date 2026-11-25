#!/usr/bin/env bash
# Assemble dist/Aether.app: native shell + embedded console + canonical CLI +
# contracts + a relocatable Python runtime. All runtime access stays local.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
export PATH="$HOME/.cargo/bin:$PATH"
APP="$ROOT/dist/Aether.app"
CONTENTS="$APP/Contents"
RES="$CONTENTS/Resources"
SKIP_PYTHON="${AETHER_SKIP_PYTHON:-0}"

echo "==> building canonical CLI (release)"
(cd "$ROOT" && cargo build --release -p aether_cli >/dev/null)

echo "==> building desktop shell (release)"
(cd "$ROOT" && cargo build --release --manifest-path apps/desktop/Cargo.toml >/dev/null)

echo "==> assembling $APP"
rm -rf "$APP"
mkdir -p "$CONTENTS/MacOS" "$RES/aether" "$RES/bin"
cp "$ROOT/apps/desktop/target/release/aether-desktop" "$CONTENTS/MacOS/Aether"
cp "$ROOT/target/release/aether" "$RES/bin/aether"
chmod +x "$CONTENTS/MacOS/Aether" "$RES/bin/aether"
for item in contracts reference_plugins python examples; do
  [ -e "$ROOT/$item" ] && cp -R "$ROOT/$item" "$RES/aether/$item"
done

if [ "$SKIP_PYTHON" != "1" ]; then
  echo "==> installing bundled Python runtime"
  "$ROOT/scripts/package/fetch-python.sh" "$RES/python"
  if [ -d "$ROOT/vendor/wheels" ]; then
    SITE="$RES/python/lib/python3.12/site-packages"
    mkdir -p "$SITE"
    "$RES/python/bin/python3" -m pip install --no-index --find-links "$ROOT/vendor/wheels" \
      --target "$SITE" --requirement "$ROOT/apps/desktop/python-runtime.txt"
  else
    echo "NOTE: vendor/wheels absent; run scripts/package/fetch-wheels.sh for full offline support"
  fi
else
  echo "NOTE: Python bundle skipped (AETHER_SKIP_PYTHON=1)"
fi

mkdir -p "$RES/licenses"
for license in LICENSE NOTICE; do
  [ -f "$ROOT/$license" ] && cp "$ROOT/$license" "$RES/licenses/$license"
done

ICON_LINE=""
if [ -f "$ROOT/apps/desktop/AppIcon.png" ]; then
  sips -s format icns "$ROOT/apps/desktop/AppIcon.png" --out "$RES/AppIcon.icns" >/dev/null
  ICON_LINE="    <key>CFBundleIconFile</key><string>AppIcon.icns</string>"
fi
cat > "$CONTENTS/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>CFBundleName</key><string>Aether</string>
  <key>CFBundleDisplayName</key><string>Aether</string>
  <key>CFBundleIdentifier</key><string>local.aether.desktop</string>
  <key>CFBundleExecutable</key><string>Aether</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>3.0.0-rc.1</string>
  <key>CFBundleVersion</key><string>1</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>NSHighResolutionCapable</key><true/>
  $ICON_LINE
</dict></plist>
PLIST

echo "==> signing (ad-hoc)"
codesign --force --deep --sign - "$APP" >/dev/null 2>&1 || echo "NOTE: ad-hoc signing unavailable"

echo "==> verifying bundle"
"$CONTENTS/MacOS/Aether" --smoke-test --project "$ROOT" | python3 -c \
  "import json,sys; d=json.load(sys.stdin); assert d['schema']['node_types']; print('bundle smoke:', len(d['schema']['node_types']), 'node types')"
echo "Built $APP"
