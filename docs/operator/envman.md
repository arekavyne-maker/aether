# envman strict mode

The envman `StrictVerifier` is the MVR-11 substrate. In strict
mode, a workflow refuses to start unless every pin in the
Environment Manifest resolves to a content-hash-matching artifact
in the local artifact root.

## What strict mode means

`mode: strict` in the AWL's `reproducibility:` block is the
declaration that the workflow requires strict reproducibility.
The kernel's workflow executor reads this flag and refuses to
start the workflow unless the envman StrictVerifier reports
`all_resolved: true`.

```yaml
reproducibility:
  mode: strict
  env_manifest: env/site.lock
```

If a plugin's hash in the envman doesn't match the hash of the
local bundle, the workflow does not start. The CLI surfaces this
as exit code 30 (contract violation, SDP §15.1).

## Running the verifier

```sh
aether envman verify /opt/aether/site.lock \
    --artifact-root /opt/aether/artifacts
```

The CLI produces a JSON report listing every pin and whether it
resolved. Exits 30 on any unresolved pin.

## Pin types

The envman manifest declares the following pin types:

| Pin | Example | Resolved by |
|---|---|---|
| `plugins` | `org.aether.foo` version + hash | a file in `plugins/<id>-<version>.tar.gz` whose SHA-256 matches `hash` |
| `containers` | `opensfoam.sif` image + image_hash | a file in `containers/<name>.sif` whose SHA-256 matches `image_hash` |
| `policy_bundle` | version + hash | a file in `policy/<version>-<hash>.json` |
| `python` | lock file + lock_hash | a file at `python.lock` whose SHA-256 matches |
| `contracts` | version pin per contract id | a file in `contracts/<id>.json` |

The `StrictVerifier` walks each artifact root and reports per pin
whether the file exists with the matching hash.

## Installing a plugin

```sh
aether envman install <plugin>.tar.gz /opt/aether/site.lock \
    --artifact-root /opt/aether/artifacts
```

The CLI copies the plugin bundle into `plugins/` and runs the
verifier. The output is the per-pin resolution report. In strict
mode, exit 30 on any unresolved pin.

## Why strict mode matters

Without strict mode, a workflow can start even if its declared
plugin versions don't match the local bundles. The workflow may
succeed in production but the run is not bit-reproducible — a
later re-run with a different plugin version may produce different
results. Strict mode is the structural guarantee that the run is
reproducible; a green `aether envman verify` is a precondition
for any production workflow.
