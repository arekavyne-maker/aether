# Experimental application surfaces

This folder contains the user-facing applications Aether ships. The name is
intentional: these surfaces are versioned independently of the canonical CLI
and are expected to change faster than the engineering contracts. They are not
a research sandbox — the same code is embedded in the released web server and
desktop bundles.

| Path | Contents |
|---|---|
| `web/` | Marketing one-page preview (`index.html`, `COPY.md`) and the zero-build console under `app/` |
| `web/app/` | Embedded console: HTML, CSS, ES modules and locally bundled fonts |
| `desktop/` | Shared `wry` + `tao` + `muda` native shell, app icon and Python runtime requirements |
| `macos/` | macOS `.app` packaging, icon conversion and signing |
| `windows/` | Windows portable ZIP, WiX MSI and ICO conversion |
| `linux/` | Linux portable tree, tar.gz, `.deb` and optional AppImage |

The native shell source is shared. Platform folders contain only packaging and
icon work, so macOS and Windows never diverge in behaviour. All three apps call
the same canonical CLI through `crates/aether_web`; no command logic lives here.

## Build

```bash
cargo build --manifest-path experimental/desktop/Cargo.toml
bash experimental/macos/macos-app.sh        # macOS
pwsh experimental/windows/windows-app.ps1   # Windows
bash experimental/linux/linux-app.sh        # Linux
```

Use `AETHER_SKIP_PYTHON=1` for a slim bundle. Signing credentials come from the
environment; see `docs/operator/desktop-app.md`.

## Tests

```bash
node --test experimental/web/app/js/*.test.mjs \
  experimental/web/app/js/viz/*.test.mjs \
  experimental/web/app/js/data/*.test.mjs
python3 infra/fitness/run.py
bash scripts/acceptance/web-ui.sh
```
