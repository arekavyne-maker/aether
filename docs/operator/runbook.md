# Aether 3.0 — Release and Operations Runbook

**Status:** living document, 2026-11-24.
**Audience:** site operators, release engineers, quality auditors.
**Rule:** every procedure below is executable from this repository. Where a
step depends on infrastructure that is not present, it is marked as a
prerequisite rather than silently skipped.

---

## 1. Prerequisites

| Requirement | Command | Notes |
|---|---|---|
| Rust toolchain | `cargo --version` | 1.75+ |
| Python | `python3 --version` | 3.9+ |
| Python deps | `python3 -m pip install -r requirements-dev.txt` | includes PyNaCl |
| CalculiX (optional) | `ccx_2.23 -v` or `$CCX_BIN` | absent ⇒ BENCH-08 is `SKIP` |

## 2. Build

```bash
cargo build --workspace
```

## 3. Verify the substrate

```bash
cargo test --workspace
python3 -m pytest -q
python3 infra/fitness/run.py
bash infra/ci/dark-build.sh
```

Expected: all tests pass; fitness reports `11/11 checks passed` with any
absent optional tools reported as `SKIP`, never as a silent pass.

## 4. Acceptance gates

```bash
bash scripts/acceptance/phase1-units-frames.sh
bash scripts/acceptance/phase2-wingbox.sh
bash scripts/acceptance/phase2-release-package.sh
bash scripts/acceptance/phase3-manufacturing.sh
bash scripts/acceptance/phase4-control-flow.sh
bash scripts/acceptance/phase4-fidelity.sh
bash scripts/acceptance/phase5-design-exploration.sh
bash scripts/acceptance/phase6-model-governance.sh
bash scripts/acceptance/phase6-agents.sh
bash scripts/acceptance/phase7-hardening.sh
bash scripts/acceptance/phase7-sandbox.sh
```

Each script exits non-zero on a failed invariant. Do not waive a failing
acceptance script; fix the defect or record a waiver through the release gate.

## 5. Configuration and release

```bash
# 1. Snapshot the released object set.
aether config snapshot examples/wingbox/configuration.asm.yaml \
    --output /tmp/wingbox.snapshot.json --json

# 2. Verify the snapshot against the object set.
aether config verify /tmp/wingbox.snapshot.json \
    examples/wingbox/configuration.asm.yaml --json

# 3. Prepare and approve the release package.
aether release prepare examples/wingbox/release-prepare.yaml \
    --approver quality-auditor --output /tmp/release.json --json
```

A release package is approved only when every gate passes and an authorized
approver is recorded. There is no execution-only release path.

## 6. Signing a plugin bundle

```bash
python3 scripts/plugin-sign.py sign \
    --key keys/plugin.pem --bundle dist/plugin.tar.gz --out dist/plugin.sig

python3 scripts/plugin-sign.py verify \
    --pubkey keys/plugin.pub --bundle dist/plugin.tar.gz --sig dist/plugin.sig
```

A bundle that fails verification is not installable. Keep the private key
outside the repository and rotate it on a documented schedule.

## 7. Offline install

1. Build the workspace on a connected machine.
2. Export the project bundle: `aether export --project <project> --output <bundle>`.
3. Transfer the bundle and the environment lock file to the air-gapped site.
4. Verify the bundle: `aether replay <bundle> --json`.
5. Confirm `verified: true` and `manifest_sha256_ok: true`.

## 8. Restore drill

```bash
python3 -m pytest -q python/aether_testing/tests/test_dr_drill.py
```

This verifies that a site can be reconstructed from signed bundles only.
A production drill must additionally restore the graph database from backup
and replay the event chain to its head.

## 9. Degraded modes

| Condition | Behavior | Operator action |
|---|---|---|
| CalculiX absent | BENCH-08 `SKIP`; fitness `ccx_binary` `SKIP` | Install CalculiX or record the skip |
| Rosetta absent | `rosetta_cross_arch` `SKIP` | Run on a real second architecture |
| Graph DB locked | Commands fail with an I/O error | Stop competing writers; restore from backup |
| Plugin binary missing | Step fails; workflow records the failure | Reinstall the plugin; re-run |

## 10. Incident response

1. Stop the affected workflow or release.
2. Verify the graph chain: `aether graph chain --project <project> --json`.
3. If the chain is invalid, restore from the last known-good backup and
   re-run the acceptance scripts.
4. Record the incident, the root cause, and the corrective action in the
   project's evidence ledger.
5. Re-run the release gate before resuming controlled work.

## 11. What this runbook does not cover

- Certification or airworthiness approval.
- Real machine control or MES integration.
- Federation key exchange between organizations.
- Kernel-level sandbox hardening (seccomp, namespaces, cgroups).
- Security penetration testing.

Those require separate, qualified programs with their own evidence.
## Local web interface

Start with `aether-web` (built via `cargo build -p aether_web`). The server
binds to `127.0.0.1` only, writes a 0600 token to `~/.aether/web.token`, and
prints the URL with the token in the fragment. Open that URL in a browser.

The interface is a CLI adapter: every action spawns the `aether` binary with
`--format json`. It cannot bypass policy; exit 30 is shown as `blocked`.
Override the bind address with `AETHER_WEB_BIND` and the token with
`AETHER_WEB_TOKEN` for testing. See `docs/operator/local-web-ui.md`.
