# Aether Desktop

Native shell for the embedded console. It uses `wry` (system WebView) and
`tao` for the window, with `muda` menus. There is no second UI: the window
loads the same assets served by `aether_web`.

## Run from source

```bash
cargo build --manifest-path experimental/desktop/Cargo.toml
./experimental/desktop/target/debug/aether-desktop --project /path/to/project
./experimental/desktop/target/debug/aether-desktop --smoke-test --project /path/to/project
```

The server runs **in-process** on `127.0.0.1:0`. A fresh 64-hex token is
generated per launch and kept in memory. WebView navigation is restricted to
that exact loopback origin. No token is written to disk or printed to a log.

## MSRV

`wry`, `tao`, and `muda` require Rust 1.92. This is a separate Cargo workspace
so the main workspace keeps its 1.75 minimum. `cargo build --workspace` at the
repository root does not build the desktop shell.

## Bundle

```bash
make python-fetch   # relocatable CPython (build-time network)
make wheels-fetch   # pinned wheels (build-time network)
make app            # dist/Aether.app
make app-smoke
```

`AETHER_SKIP_PYTHON=1 make app` produces a bundle without the Python runtime;
Rust-backed commands still work and `aether doctor` reports Python-backed
commands honestly. The distribution bundle includes Python by default.

The shell reads bundled resources relative to `Contents/MacOS/Aether`:

| Variable | Bundled path |
|---|---|
| `AETHER_BIN` | `Contents/Resources/bin/aether` |
| `AETHER_MONOREPO` | `Contents/Resources/aether` |
| `AETHER_PYTHON` | `Contents/Resources/python/bin/python3` |
| `PYTHONPATH` | bundled packages + `python/` sources |

Ad-hoc signing is applied for local use. Distribution needs Developer ID
signing and notarization; no network access is required at runtime.
