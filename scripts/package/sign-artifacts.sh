#!/usr/bin/env bash
# Sign and checksum a distributable artifact.
#
# Configure one of:
#   AETHER_SIGNING_CERT   Windows Authenticode certificate subject (signtool)
#   AETHER_GPG_KEY        Linux GPG key id (detach-sign)
# Set AETHER_REQUIRE_SIGNED=1 to fail when signing is not configured.
set -euo pipefail

ARTIFACT="${1:?artifact path required}"
[ -f "$ARTIFACT" ] || [ -d "$ARTIFACT" ] || { echo "no such artifact: $ARTIFACT" >&2; exit 1; }

OUT="$ARTIFACT.sha256"
if [ -d "$ARTIFACT" ]; then
  tar -cf - "$ARTIFACT" | shasum -a 256 > "$OUT"
else
  (cd "$(dirname "$ARTIFACT")" && shasum -a 256 "$(basename "$ARTIFACT")" > "$(basename "$OUT")")
fi

case "$(uname -s)" in
  MINGW*|MSYS*|CYGWIN*)
    if [ -n "${AETHER_SIGNING_CERT:-}" ]; then
      "${AETHER_SIGNTOOL:-signtool}" sign /fd SHA256 /n "$AETHER_SIGNING_CERT" "$ARTIFACT"
      echo "signed $ARTIFACT with Authenticode certificate $AETHER_SIGNING_CERT"
    elif [ "${AETHER_REQUIRE_SIGNED:-0}" = "1" ]; then
      echo "FAIL: AETHER_SIGNING_CERT is required" >&2; exit 1
    else
      echo "NOTE: Windows artifact is unsigned (dev build)"
    fi
    ;;
  Linux)
    if [ -n "${AETHER_GPG_KEY:-}" ]; then
      gpg --batch --yes --armor --detach-sign --local-user "$AETHER_GPG_KEY" --output "$ARTIFACT.asc" "$ARTIFACT"
      echo "signed $ARTIFACT with GPG key $AETHER_GPG_KEY"
    elif [ "${AETHER_REQUIRE_SIGNED:-0}" = "1" ]; then
      echo "FAIL: AETHER_GPG_KEY is required" >&2; exit 1
    else
      echo "NOTE: Linux artifact is unsigned (dev build)"
    fi
    ;;
  Darwin)
    echo "NOTE: macOS .app is ad-hoc signed by macos-app.sh; notarise with xcrun when distributing"
    ;;
esac

# A detached signature is also checksummed when present.
if [ -f "$ARTIFACT.asc" ]; then
  (cd "$(dirname "$ARTIFACT")" && shasum -a 256 "$(basename "$ARTIFACT").asc" >> "$(basename "$OUT")")
fi
echo "checksums: $OUT"
