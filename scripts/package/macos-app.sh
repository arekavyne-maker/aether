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
mkdir -p "$CONTENTS/MacOS"
cp "$ROOT/apps/desktop/target/release/aether-desktop" "$CONTENTS/MacOS/Aether"
chmod +x "$CONTENTS/MacOS/Aether"
bash "$ROOT/scripts/package/stage-resources.sh" "$RES" "$ROOT/target/release/aether"

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

echo "==> signing"
if [ -n "${AETHER_SIGNING_IDENTITY:-}" ]; then
  codesign --force --deep --options runtime --sign "$AETHER_SIGNING_IDENTITY" "$APP" \
    || { echo "FAIL: Developer ID signing failed" >&2; exit 1; }
  if [ -n "${AETHER_NOTARY_PROFILE:-}" ]; then
    xcrun notarytool submit "$APP" --keychain-profile "$AETHER_NOTARY_PROFILE" --wait
    xcrun stapler staple "$APP"
  fi
elif [ "${AETHER_REQUIRE_SIGNED:-0}" = "1" ]; then
  echo "FAIL: AETHER_SIGNING_IDENTITY is required" >&2; exit 1
else
  codesign --force --deep --sign - "$APP" >/dev/null 2>&1 || echo "NOTE: ad-hoc signing unavailable"
fi

echo "==> verifying bundle"
"$CONTENTS/MacOS/Aether" --smoke-test --project "$ROOT" | python3 -c \
  "import json,sys; d=json.load(sys.stdin); assert d['schema']['node_types']; print('bundle smoke:', len(d['schema']['node_types']), 'node types')"
bash "$ROOT/scripts/package/sign-artifacts.sh" "$APP"
echo "Built $APP"
