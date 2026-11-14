# Long-term support (LTS) policy

Every major release of Aether is supported for **36 months** as
an LTS branch. The latest LTS receives:

- **Patch releases** (bug fixes, non-breaking) for the full 36
  months.
- **Security CVE backports** for the full 36 months.
- **Conformance test backports** for the full 36 months.

A previous LTS branch receives:

- **Patch releases** for 18 months after the next major version
  ships.
- **Security CVE backports** for the full 36 months.
- **Conformance test backports** for the full 36 months.

After 36 months, the LTS branch enters **end-of-life** mode:
security CVEs are no longer backported; sites must upgrade to the
current LTS.

## LTS branches

| Version | Release | EOL | Status |
|---|---|---|---|
| v1.0 | 2026-Q3 | 2029-Q3 | current LTS |
| v0.7 | 2026-Q1 | 2027-Q3 | previous LTS (security + conformance) |
| v0.5 | 2025-Q2 | 2026-Q4 | previous LTS (security only) |
| v0.3 | 2024-Q2 | 2025-Q4 | EOL |

(These are illustrative; actual dates track the release cadence.)

## What an LTS gets

An LTS branch is a git branch (e.g. `lts/v1.0`) that:

- Receives patch releases cherry-picked from `main`.
- Receives security CVE backports from the upstream dep audit.
- Has its conformance fixtures re-run on every patch.
- Is supported by the documentation site (this site is built from
  the latest LTS).

A patch release is tagged `v1.0.N` (e.g. `v1.0.1`).

## What an LTS does NOT get

- New features. A new feature is added to `main`; cherry-picking
  to an LTS is reserved for the next major release.
- New contracts. A new contract is added to `main`; LTS
  conformance tests don't include the new contract.
- Documentation updates. The docs site reflects the current LTS;
  older LTS sites use the versioned docs (git tag).

## How to upgrade an LTS site

A site on v0.7 LTS upgrading to v1.0 LTS:

1. Read the v1.0 migration guide (in `certification/v1.0.md`).
2. Run `aether workflow validate <site.workflow.yaml>` against
   v1.0; fix any deprecated method calls.
3. Run `aether envman verify <new_manifest>` against v1.0; resolve
   any new pins.
4. Deploy v1.0 alongside v0.7 (parallel run); route new workflows to
   v1.0.
5. After a 30-day soak, decommission v0.7.

The dark-build CI is the structural guarantee: every commit to
`main` is tested against the current LTS and the previous LTS in
parallel. A green dark-build means the upgrade path is real.
