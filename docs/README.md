# Aether — Documentation

Aether is an air-gapped, hardware-agnostic, fully modular engineering
fabric for emerging aerostructures. This site is the canonical
documentation: the architecture overview, the contract catalog, the
operator's guide, the plugin author's guide, the maintenance model,
and the certification dossier.

## Table of contents

| Section | File | Audience |
|---|---|---|
| Architecture overview | `architecture/overview.md` | Architects, integrators |
| Twelve invariants | `architecture/invariants.md` | Architects, kernel engineers |
| AWL v1.3 (workflow language) | `architecture/awl.md` | Workflow authors |
| Engineering Graph (G0 contract surface) | `architecture/graph.md` | All engineers |
| Contract catalog (24 contracts) | `contracts/index.md` | All engineers |
| Operator's guide | `operator/site-install.md` | Site operators |
| Operator's guide (offline) | `operator/airgap.md` | Site operators |
| Operator's guide (HPC backends) | `operator/hpc-backends.md` | Site operators |
| Operator's guide (edge cells) | `operator/edge-cells.md` | Site operators |
| Operator's guide (envman strict mode) | `operator/envman.md` | Site operators |
| Operations runbook (release, restore, degraded modes) | `operator/runbook.md` | Site operators, release engineers |
| Plugin author's guide | `plugin-author/getting-started.md` | Plugin authors |
| Plugin author's guide (the contract surface) | `plugin-author/contract-surface.md` | Plugin authors |
| Plugin author's guide (TUF-signed bundles) | `plugin-author/signed-bundles.md` | Plugin authors |
| Maintenance model | `maintenance/release-cadence.md` | Site operators, integrators |
| Maintenance model (LTS) | `maintenance/lts-policy.md` | Site operators, integrators |
| Maintenance model (deprecation) | `maintenance/deprecation.md` | All engineers |
| Maintenance model (CVE feed) | `maintenance/cve-feed.md` | Site security |
| Certification dossier (S-1..S-8) | `certification/s-1.md` ... `certification/s-8.md` | Certification reviewers |

## Quick links

- The contract catalog lists every Aether contract with its method
  surface, schemas, and example documents: `contracts/index.md`.
- The operator's guide covers site install, offline operation, HPC
  backends, edge cells, and the envman strict-mode workflow:
  `operator/`.
- The plugin author's guide covers getting started, the contract
  surface, and TUF-signed bundles: `plugin-author/`.
- The maintenance model documents the release cadence, LTS, and
  deprecation policy: `maintenance/`.
- The certification dossier contains the S-1 through S-8 evidence
  rollups: `certification/`.

## Reading paths

| Role | Path |
|---|---|
| Principal architect / tech lead | Entire `architecture/` + `certification/` |
| Kernel / platform engineer | `architecture/invariants.md` + `operator/site-install.md` |
| Scientific computing / ML engineer | `architecture/awl.md` + `contracts/index.md` |
| Aerospace systems engineer (MBSE) | `architecture/graph.md` + `certification/` |
| Manufacturing / test engineer | `operator/` + `plugin-author/` |
| Security / air-gap engineer | `operator/airgap.md` + `maintenance/cve-feed.md` + `certification/s-1.md` |
| Plugin author | `plugin-author/` |
| Site operator | `operator/site-install.md` + `operator/envman.md` |
| Certification / quality auditor | `certification/` |

## Versioning

**Status:** the platform is substrate-complete for Phases 0–2, with early
Phase 3–7 vertical-slice artifacts where noted in
[`verification/STATUS.md`](verification/STATUS.md). It has **not** reached
the v1.0 final acceptance gate; an earlier claim to that effect was
retracted and the defensible, verification-backed status is maintained in
[`verification/STATUS.md`](verification/STATUS.md) and
[`workflows/paper-run/gaps.md`](../workflows/paper-run/gaps.md). The
described target architecture corresponds to the end-state plan, not
to shipped capability; where a capability is not yet real, the
operator and plugin-author guides state so explicitly.
