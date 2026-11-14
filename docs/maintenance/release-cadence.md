# Release cadence

Aether v1.0 follows a **predictable release cadence** designed for
long-term maintenance:

- **Major** releases (v1, v2, ...): yearly. Breaking changes
  only; S-1 through S-8 evidence rollup; LTS designation.
- **Minor** releases (v1.1, v1.2, ...): quarterly. New contracts
  (added to the catalog), new features, non-breaking changes.
- **Patch** releases (v1.0.1, v1.0.2, ...): as needed. Bug fixes,
  security patches, conformance-fixture updates.

A new major version is announced in `certification/s-1.md` and
ships with a fresh LTS branch. The previous LTS branch receives
patch releases for 18 months after the new major version.

## Release tags

Every release is a git tag in the platform monorepo. The release
notes are in `certification/<version>.md`. The conformance
fixtures are pinned to the released tag; the dev branch is
unstable and may break contracts (with an ADR).

## LTS policy

The latest LTS branch receives:

- **Patch releases** for 18 months after the next major version.
- **Security CVE backports** for the full lifetime of the LTS
  branch (typically 36 months from initial release).
- **Conformance test backports** for the full LTS lifetime.

The non-LTS branches (older major versions) receive only security
CVE backports for 6 months after the next major version.

## Migration windows

- **Patch → Patch**: drop-in. The interface is stable; the only
  change is a bug fix.
- **Minor → Minor**: drop-in for contract consumers; plugin
  authors may need to update if a contract's method surface
  changed (rare; ADRs required).
- **Minor → Major**: planned migration. The release notes for
  the new major version contain a migration guide; the
  conformance fixtures are updated; the dev branch carries the
  migration code for at least 2 minor versions.

## Deprecation

Methods and contracts are deprecated via the SDP §16 deprecation
policy (see `deprecation.md`). A deprecated method:

- Continues to work in the current LTS.
- Logs a warning every time it is called.
- Is removed in the next major version.

The release notes for each minor version list newly-deprecated
methods and their planned removal version.

## Security CVE feed

See `cve-feed.md`. Aether's security policy is: every CVE that
affects a transitive dependency gets a patch release within 7 days
of upstream's fix for LTS branches, and within 30 days for the dev
branch.
