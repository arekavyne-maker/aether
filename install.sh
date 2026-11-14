#!/usr/bin/env bash
# Aether installer (Phase 2 — Installation & UX).
#
# Installs Aether from this source tree onto the local machine:
#   1. builds the `aether` CLI (release profile) with cargo;
#   2. installs an `aether` launcher into $PREFIX/bin that pins the
#      platform home (AETHER_MONOREPO) so projects anywhere on disk can
#      resolve the contract catalog and reference plugins;
#   3. runs `aether doctor` to report, honestly, what works here.
#
# The installer is intentionally boring: no network beyond the Rust
# crates already vendored/cached, no Python installation steps, no
# services. Optional solvers (CalculiX, OpenFOAM) are probed by
# `aether doctor` and reported as optional-absent when missing.

set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PREFIX="${AETHER_PREFIX:-$HOME/.local}"
BIN_DIR="$PREFIX/bin"

if ! command -v cargo >/dev/null 2>&1; then
    echo "error: cargo not found — install Rust (https://rustup.rs) and re-run." >&2
    exit 1
fi

echo "==> Building aether (release) — this takes a few minutes on first run"
(cd "$HERE" && cargo build --release -p aether_cli)

echo "==> Installing launcher into $BIN_DIR"
mkdir -p "$BIN_DIR"
cat > "$BIN_DIR/aether" <<LAUNCHER
#!/usr/bin/env bash
# Aether launcher — pins the platform home so standalone projects can
# resolve the contract catalog and reference plugins. Overridable via
# AETHER_MONOREPO.
export AETHER_MONOREPO="\${AETHER_MONOREPO:-$HERE}"
exec "$HERE/target/release/aether" "\$@"
LAUNCHER
chmod +x "$BIN_DIR/aether"

case ":$PATH:" in
    *":$BIN_DIR:"*) ;;
    *)
        echo ""
        echo "NOTE: $BIN_DIR is not on your PATH. Add this to your shell profile:"
        echo "      export PATH=\"$BIN_DIR:\$PATH\""
        echo ""
        ;;
esac

echo "==> Running aether doctor"
"$BIN_DIR/aether" doctor || true

echo ""
echo "Aether installed. Next steps:"
echo "  aether demo aircraft"
echo "  aether init my-wing --profile aerospace"
echo "  cd my-wing && aether run && aether explain"
