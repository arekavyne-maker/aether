# Aether Desktop (macOS, Windows, Linux)

**Status:** deployable native shell (ADR-0044, ADR-0047).
**Scope:** the embedded console in a system WebView, backed by the same CLI.

## Platforms

| Platform | Build command | Artifact | Signing |
|---|---|---|---|
| macOS | `make app` | `dist/Aether.app` | ad-hoc, or Developer ID + notarisation with `AETHER_SIGNING_IDENTITY` / `AETHER_NOTARY_PROFILE` |
| Windows | `pwsh experimental/windows/windows-app.ps1` | `dist/windows/Aether-windows.zip`, `Aether.msi` (WiX) | `signtool` with `AETHER_SIGNING_CERT` |
| Linux | `make app-linux` | `dist/linux/Aether-linux.tar.gz`, `.deb`, optional AppImage | GPG detached signature with `AETHER_GPG_KEY` |

Set `AETHER_REQUIRE_SIGNED=1` for release builds to fail when credentials are
missing. SHA-256 sums accompany every artifact. Windows needs the WebView2
runtime; Linux needs `libwebkit2gtk-4.0-dev`, `libgtk-3-dev` and
`libayatana-appindicator3-dev` at build time.

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
cargo build --manifest-path experimental/desktop/Cargo.toml
./experimental/desktop/target/debug/aether-desktop --project /path/to/project
```

## Runtime model

- The Axum server runs **in-process** on `127.0.0.1:0`.
- A 64-hex token is generated per launch and kept only in memory.
- WebView navigation is restricted to `http://127.0.0.1:<port>`; external
  links and `window.open` are blocked.
- Every action still spawns the canonical `aether` binary with discrete argv,
  `--offline` and by default `--json`.
- There is no token file, no service, and no network egress.

## Application icon

The product mark lives at `experimental/desktop/AppIcon.png`. Packaging converts it to
`AppIcon.icns` for macOS and `AppIcon.ico` for the Windows installer; the same
PNG is embedded in the console and shown as the brand mark in the top bar.

## Window

The window is undecorated and carries a custom dark titlebar with macOS
traffic-light buttons, the project name and a drag region. Buttons and dragging
are forwarded to native actions over wry IPC; the native menu and its
accelerators remain available. Window geometry and recent projects are stored
under `~/Library/Application Support/Aether/`. The content uses the same
locally bundled AIXOR-inspired typography and palette as the browser console.

## Native workflow

The menu bar uses stable ids and real accelerators:

| Menu | Actions |
|---|---|
| Aether | About, Diagnostics, Reload `⌘R`, Toggle theme `⌘T`, Quit `⌘Q` |
| File | Open Project… `⌘O`, Current Project, Recent Projects |
| Run | Run Command `⌘↵`, Cancel Job `⌘.`, CLI Terminal `⌘J`, Session Ledger |
| View | Home, Run, Atlas, Verify, Command Index |
| Help | Keyboard Shortcuts `?` |

Opening or choosing a recent project calls the in-process `/api/project`
endpoint; the server does not restart and no token changes. Recent paths and
window geometry are stored in `~/Library/Application Support/Aether/` as JSON.
Those files contain no engineering data or secrets.

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
