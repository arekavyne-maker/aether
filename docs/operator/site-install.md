# Site install

This guide covers installing Aether v1.0 on a fresh site, in air-gapped
mode (the default and recommended). For online mode, see
`airgap.md`.

## Pre-flight

The site must have:

- A POSIX-compliant shell (`bash`, `sh`)
- A C toolchain (gcc or clang)
- A Rust toolchain (`rustc` 1.75+)
- A Python 3.11+ interpreter
- A target architecture: `x86_64` or `aarch64`

The dark-build CI validates these on `x86_64-linux-gnu` and
`aarch64-linux-gnu` (MVR-04/05/12).

## 1. Vendor the platform

The site must receive the platform as a signed bundle. The bundle is
a directory or tarball containing:

- `aether-cli` and the kernel shared library
- The TUF-metadata directory: `targets/`, `snapshot.json`, `timestamp.json`
- The envman lockfile: `env/site.lock`
- The .aetherpkg manifest: `manifest.json`
- The plugin bundles: `plugins/*.tar.gz` (referenced by the lockfile)
- The container images (SIF): `containers/*.sif`
- The policy bundle: `policy/policy-*.json`

Verify the bundle's TUF signature chain before unpacking:

```sh
aether tuf verify --root root.json targets.json snapshot.json timestamp.json
```

A `Validation` exit code is a hard error; do not proceed.

## 2. Unpack to a target site root

```sh
mkdir -p /opt/aether/artifacts
cp -r plugins containers policy contracts /opt/aether/artifacts/
cp env/site.lock /opt/aether/site.lock
aether envman install <plugin>.tar.gz /opt/aether/site.lock \
    --artifact-root /opt/aether/artifacts
```

Repeat for every plugin. Each `install` runs the `StrictVerifier`
and reports whether every pin resolved. In strict mode, an
unresolved pin is a hard error.

## 3. Initialize the site graph

```sh
cd /opt/aether
./aether init /opt/aether/site
```

This creates a `.aether/project.toml` and the per-project CAS
directories. The site graph is the kernel's default storage
(SQLite + local CAS).

## 4. Verify strict reproducibility

```sh
aether envman verify /opt/aether/site.lock \
    --artifact-root /opt/aether/artifacts
```

`all_resolved: true` is required for production. Any `unresolved`
pin is a contract violation (exit code 30).

## 5. Run a smoke workflow

```sh
aether workflow run workflows/paper-run.awl.yaml \
    --project /opt/aether/site
```

A green smoke run is the structural assertion that the install is
correct. Save the run's `chain_head` and provenance records; they
are the inputs to the daily diff in `operator/airgap.md`.

## Network policy

Production sites run with `AETHER_OFFLINE=1` set in the site
environment. Every network-aware code path fails closed; the
`aether envman verify` is the canonical check that the site is
air-gapped.
