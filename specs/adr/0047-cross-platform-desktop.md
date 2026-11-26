# ADR-0047: Cross-platform desktop and signed distributables

**Status:** Accepted
**Date:** 2026-11-25
**References:** [ADR-0044](0044-atlas-and-desktop-shell.md), [ADR-0046](0046-aixor-inspired-interface.md), [audit/FINDINGS.md](../../audit/FINDINGS.md)

## Context

The audit found platform assumptions that made the native shell macOS-only in
practice: `/dev/urandom` and `$HOME` in the token layer, CLI discovery without
`.exe`, a hardcoded Application Support path, Apple-only Python runtime fetch,
and a single `.app` packager. The CLI must remain untouched, and the web UI
must remain identical on every platform.

## Decision

### Portability fixes

- `aether_web` uses `getrandom` and `dirs` for tokens and paths; Windows token
  files get a best-effort `icacls` restriction.
- CLI discovery uses `std::env::consts::EXE_SUFFIX`, so `aether.exe` is found on
  Windows without invoking a shell.
- Job cancellation uses `taskkill /PID /T /F` on Windows and process-group
  `SIGTERM` on Unix, both as direct processes.
- Desktop state lives under `dirs::data_dir()/Aether`. Accelerators use `Cmd`
  on macOS and `Ctrl` elsewhere. Linux keeps native window decorations and
  hides the custom titlebar because Wayland drag is unreliable.

### Distributables

- **macOS:** `.app`, Developer ID signing and optional notarisation when
  `AETHER_SIGNING_IDENTITY` / `AETHER_NOTARY_PROFILE` are set; ad-hoc by default
  for local development.
- **Windows:** portable directory + ZIP + WiX MSI, built by
  `experimental/windows/windows-app.ps1`; Authenticode signing via `signtool` when
  `AETHER_SIGNING_CERT` is set.
- **Linux:** portable tree + tar.gz + `.deb`, with AppImage when `appimagetool`
  exists; GPG detached signature when `AETHER_GPG_KEY` is set.
- `AETHER_REQUIRE_SIGNED=1` makes any missing signing credential a hard
  failure. SHA-256 sums are emitted for every artifact.
- The Python runtime fetch supports Apple, Windows and Linux targets; wheels
  are pinned per platform. `AETHER_SKIP_PYTHON=1` remains the slim path.

### Verification

Platform-aware acceptance scripts and fitness checks, plus a three-platform CI
matrix under `.github/workflows/portability.yml`. The CLI source, its command
tree and golden tests are untouched; optional tools that rely on `sh` probes
report honestly unavailable on Windows.

## Consequences

- One web UI and one desktop codebase serve all three platforms; no forked
  frontend or separate command surface.
- Windows requires WebView2 runtime; Linux requires WebKitGTK/GTK development
  libraries at build time and runtime.
- Signed releases require secrets supplied by the build environment; the
  repository never contains certificates or private keys.
- Final `3.0.0` is still held pending the federation/security gate.

## Verification

`cargo test --workspace`, `pytest`, Node tests, fitness, acceptance scripts,
dark-build, the portability CI matrix, and `--smoke-test` on each platform.
