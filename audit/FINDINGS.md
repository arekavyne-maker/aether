# Aether audit — findings register

**Date:** 2026-11-25
**Scope:** Rust workspace, Python peers, local web console, macOS desktop shell,
packaging, acceptance, fitness, docs and cross-platform portability.
**Rule:** every finding has a source or command behind it; nothing is guessed.

## Method

1. Source review for platform assumptions, process boundaries, secret handling
   and engineering-claim honesty.
2. Gate replay: `cargo test --workspace`, `pytest`, Node tests, fitness,
   acceptance scripts, dark-build, docs links.
3. Functional probes against the live CLI and console: 124 discovered commands,
   82 generated leaf forms, 42 group nodes, 4 approval and 4 long commands,
   Atlas routes, live job lifecycle and project switching.
4. Browser pass over every destination, both themes and a narrow viewport.
5. Cross-platform analysis of binaries, paths, WebView dependencies and
   packaging artifacts.

## Confirmed strengths

- CLI source, command tree and golden tests remain unchanged.
- Every displayed engineering value is traced to a canonical CLI call; the
  console never invents geometry, progress or metrics.
- Strict CSP, loopback-only bind, bearer auth, discrete argv, no shell.
- 124 commands and 82 generated leaf forms are reachable from the UI.
- Live jobs stream real stdout/stderr, expose elapsed time and cancel.
- Air-gapped runtime with locally bundled fonts and no external assets.

## Bug register

| ID | Sev | Finding | Evidence | Status |
|---|---|---|---|---|
| B-01 | P0 | Token generation read `/dev/urandom`, failing on Windows. | `auth.rs:72` | **Fixed** — `getrandom` for all platforms. |
| B-02 | P0 | Token path required `$HOME`, absent on Windows. | `auth.rs:67` | **Fixed** — `dirs::home_dir()` plus best-effort `icacls`. |
| B-03 | P0 | CLI discovery ignored `.exe`, so Windows could not find the binary. | `cli.rs:112-124` | **Fixed** — `EXE_SUFFIX` candidates and explicit PATH search. |
| B-04 | P0 | Desktop state dir hardcoded to macOS Application Support. | `desktop/src/main.rs:57` | **Fixed** — `dirs::data_dir()/Aether`. |
| B-05 | P0 | Packaging produced only a macOS `.app`; no Windows/Linux artifact. | `scripts/package/macos-app.sh` | **Fixed** — shared staging, Windows ZIP + WiX MSI, Linux tar.gz + .deb + AppImage. |
| B-06 | P0 | Python runtime fetch supported only Apple targets. | `fetch-python.sh:12-13` | **Fixed** — Apple, Windows and Linux targets with platform wheelhouses. |
| B-07 | P1 | Desktop acceptance skipped non-Darwin hosts. | `acceptance/desktop.sh:11` | **Fixed** — `desktop-linux.sh` and `desktop-windows.ps1`. |
| B-08 | P1 | Fitness only checked the macOS bundle layout. | `fitness/desktop_shell.py` | **Fixed** — platform-aware bundle checks. |
| B-09 | P2 | Job cancel used Unix process-group kill only. | `jobs.rs:42` | **Fixed** — `taskkill /PID /T /F` on Windows, no shell. |
| B-10 | P2 | `sh -c command -v` probes always report tools unavailable on Windows. | `spine.rs:295`, `sandbox.rs:338` | **Accepted** — CLI stays untouched; reports honestly. |
| B-11 | P2 | Programmatic drag may fail on Wayland/WebKitGTK. | desktop `drag_window()` | **Mitigated** — Linux keeps native decorations and hides the custom titlebar. |
| B-12 | P2 | No CI matrix verifies Windows or Linux. | no `.github/workflows` | **Fixed** — `.github/workflows/portability.yml` runs all three platforms. |

## Audit tracks still to close

| Track | Status |
|---|---|
| Functional command parity (124/82/42/4/4) | Verified on macOS; to repeat on Windows/Linux |
| Rust/Python/Node gates | Green on macOS; to repeat in CI matrix |
| Security: token, CSP, argv, path confinement, job cancel | Green on macOS; Windows token ACL added, needs runner verification |
| Reliability: stale jobs, restart, concurrency, process cleanup | Job registry in memory by design; restart discards jobs — documented |
| Performance: 5k/10k caps, SVG DOM, cold start | Server caps verified; client benchmark still unrecorded |
| UX: both themes, narrow viewport, keyboard, reduced motion | Browser pass green, no console errors |
| Release integrity: SBOM, font licences, docs links, claims | Links and licences resolve; signing hooks to be added |

## Release blockers

Final `3.0.0` remains held pending real second-site federation evidence or a
signed-off harness plus independent security review. Cross-platform support
does not change that gate. Aether remains `3.0.0-rc.1`.
