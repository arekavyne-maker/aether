# Aether Desktop (macOS)

**Status:** deployable native shell (ADR-0044).
**Scope:** the embedded console in a system WebView, backed by the same CLI.

## Build

```bash
make python-fetch   # relocatable CPython, build-time network only
make wheels-fetch   # pinned runtime wheels
make app            # dist/Aether.app
make app-smoke
```

For a smaller bundle without Python, run `AETHER_SKIP_PYTHON=1 make app`.
Rust-backed commands still work; `aether doctor` reports Python-backed
commands honestly.

Run from source without packaging:

```bash
cargo build --manifest-path apps/desktop/Cargo.toml
./apps/desktop/target/debug/aether-desktop --project /path/to/project
```

## Runtime model

- The Axum server runs **in-process** on `127.0.0.1:0`.
- A 64-hex token is generated per launch and kept only in memory.
- WebView navigation is restricted to `http://127.0.0.1:<port>`; external
  links and `window.open` are blocked.
- Every action still spawns the canonical `aether` binary with discrete argv,
  `--offline` and by default `--json`.
- There is no token file, no service, and no network egress.

## Bundled resources

The shell locates resources relative to `Contents/MacOS/Aether`:

| Variable | Path |
|---|---|
| `AETHER_BIN` | `Contents/Resources/bin/aether` |
| `AETHER_MONOREPO` | `Contents/Resources/aether` |
| `AETHER_PYTHON` | `Contents/Resources/python/bin/python3` |
| `PYTHONPATH` | bundled packages plus `python/` sources |

Recent projects, window state and preferences live in
`~/Library/Application Support/Aether/`. No engineering data leaves the
project directory.

## Distribution

The local build is ad-hoc signed. Distribution requires Developer ID signing
and notarization. The bundle is self-contained at runtime; Python and wheel
fetching happens only on the build machine, with hashes recorded in
`vendor/python/SHA256SUMS` and `vendor/wheels/SHA256SUMS`.

Windows and Linux packaging are not shipped yet; the `wry` shell itself is
portable.
