# Repository scripts

| Path | Purpose |
|---|---|
| `acceptance/` | End-to-end acceptance scripts for every phase, the web console and the three desktop bundles |
| `author_tuf_bundle.py` | Build a signed offline TUF bundle |
| `bootstrap-dev.sh` | Local developer bootstrap |
| `fetch_calculix.sh` | Fetch and verify the pinned CalculiX runtime |
| `plugin-sign.py` | Sign reference plugins |
| `package/fetch-python.sh` | Fetch a relocatable CPython for any supported platform (build-time network only) |
| `package/fetch-wheels.sh` | Download pinned runtime wheels |
| `package/stage-resources.sh` | Shared resource staging for every desktop bundle |
| `package/sign-artifacts.sh` | Checksum and sign distributables |

Platform-specific packagers live beside the apps they build:

- `experimental/macos/macos-app.sh`
- `experimental/windows/windows-app.ps1`
- `experimental/linux/linux-app.sh`

Keep acceptance scripts here so gate commands do not move when application
sources are reorganised.
