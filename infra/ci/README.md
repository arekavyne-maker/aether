# Aether Dark-Build CI Runner (Phase 0; ADR-0023, SDP §19.5)

The Aether CI runs on **egress-free** runners. The runner is the substrate
that honors I-5 (air-gap by design): no network access from any CI step,
with all dependencies vendored.

## Phase 0 runner

`dark-build.sh` is the Phase 0 entry point. It runs every CI check the
local environment can run, and asserts the network is unreachable.

```bash
./infra/ci/dark-build.sh
```

The script:

1. Records the network state before the run.
2. Runs:
   - `cargo build -p aether_kernel -p aether_cli -p aether_graph`
   - `cargo test` for each crate
   - `python/aether_testing/` contract conformance
   - `python/aether_sdk/` validation
   - `infra/fitness/run.py` (LOC budget, token scan, schema semver, SBOM)
3. Asserts no DNS lookups or TCP connections were made (best-effort: in
   Phase 0, by checking that no curl/wget/etc. was invoked; the network
   firewall in production runners is at the OS level).

## Phase 1+ runner

Phase 1 adds:

- Apptainer-based runner images built from the vendored cache.
- Network firewall config in `infra/ci/dark-build-firewall.sh` (Linux
  nftables; macOS `pf`).
- Per-arch matrix (x86_64 + aarch64).
- MVR-suite runner (modularity validation).
- Conformance suites for touched contracts.

The runner is **the same script** for both local and CI use; the only
difference is where the output goes.
