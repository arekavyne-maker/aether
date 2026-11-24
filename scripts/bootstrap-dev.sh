#!/usr/bin/env bash
# Bootstrap a reproducible local development environment.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

echo "==> Rust toolchain"
if ! command -v cargo >/dev/null 2>&1; then
    if [ -x "$HOME/.cargo/bin/cargo" ]; then
        export PATH="$HOME/.cargo/bin:$PATH"
    else
        echo "error: cargo not found; install Rust from https://rustup.rs" >&2
        exit 1
    fi
fi
rustc --version
cargo --version

echo "==> Python"
python3 --version
python3 -m pip install --upgrade pip >/dev/null 2>&1 || true
python3 -m pip install -r requirements-dev.txt

echo "==> Editable Python packages"
python3 -m pip install -e python/aether_sdk -e python/aether_geometry \
    -e python/aether_requirements -e python/aether_testing

echo "==> Build"
cargo build --workspace

echo "==> Smoke"
python3 -m pytest -q python/aether_geometry/tests python/aether_requirements/tests
cargo test -p aether_kernel --test walking_skeleton_a

echo "Bootstrap complete. Try: make test && make fitness"
