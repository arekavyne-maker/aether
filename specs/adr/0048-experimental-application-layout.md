# ADR-0048: Experimental application layout and repository cleanup

**Status:** Accepted
**Date:** 2026-11-26
**References:** [ADR-0044](0044-atlas-and-desktop-shell.md), [ADR-0047](0047-cross-platform-desktop.md)

## Context

The repository mixed canonical engineering substrate, application surfaces,
platform packaging and scratch output at the root. That made it hard to tell
which code is versioned with the contracts and which code is a presentation
surface, and left stale documentation beside current instructions.

## Decision

- Application sources move under `experimental/`:
  - `experimental/web/` for the marketing preview and the embedded console.
  - `experimental/desktop/` for the single shared `wry` shell and app icon.
  - `experimental/macos/`, `experimental/windows/`, `experimental/linux/` for
    packaging, installers and platform icons only.
- The name `experimental/` marks independent versioning of the surfaces; the
  code inside is the same production code embedded in released bundles.
- Acceptance scripts remain at `scripts/acceptance/` so gate commands are
  stable. Shared package helpers remain at `scripts/package/`.
- Local build and test scratch (`dist/`, `target/`, `.pytest_cache`,
  `.hypothesis`, `.DS_Store`, local Aether state) is removed from the working
  tree. `vendor/` is retained because offline bundles need it.
- The root research brief is archived as
  `docs/research/AETHER-3.0-ADAPTIVE-AEROSTRUCTURES.md` and linked from the
  research register.
- Stale “Obsidian” and “Pulse” wording is replaced with “AIXOR-inspired”,
  “Home”, “Run” and “Verify” where it describes current behaviour.
- A `repo_layout` fitness check fails on stale paths and missing entry points.

## Consequences

- Cloning the repo gives a clean source tree; build outputs are regenerated on
  demand.
- macOS and Windows share one native shell source, so they cannot drift.
- All path references are centralised in `assets.rs`, packaging scripts,
  fitness checks, Makefile, CI and docs; the layout check prevents regression.
- Historical ADRs still describe the decisions in the order they were made;
  their current path references are updated.

## Verification

`cargo test --workspace`, `cargo build --manifest-path experimental/desktop/Cargo.toml`,
`pytest`, Node tests, `python3 infra/fitness/run.py` (including `repo_layout`),
all acceptance scripts, dark-build, docs links, browser pass and a rebuilt
macOS bundle with the logo.
