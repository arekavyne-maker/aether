# ADR-0044: Atlas visualization and native desktop shell

**Status:** Accepted
**Date:** 2026-11-25
**References:** [ADR-0042](0042-local-web-interface.md), [ADR-0043](0043-interactive-web-console.md), [ADR-0010](0010-cli-canonical-interface.md) (I-10)

## Context

The interactive console of ADR-0043 is command-centric: a user browses ten
workspaces and 124 commands, then composes one invocation at a time. That is
a capable adapter, but it is not yet an application for understanding an
engineering project. The request was for a far more dynamic, easier-to-
navigate, highly visual interface that can also run as a native macOS app.

The canonical CLI had to remain untouched. Geometry and mesh commands expose
hashes and provenance, not tessellated shape; drawing a fabricated 3D model
would violate the accuracy requirement.

## Decision

### Atlas

A new **Atlas** view visualizes only data produced by the CLI:

- **Graph** — nodes and edges from `aether graph as-of <offset> --json`, with
  deterministic force, layered and radial layouts; pan, zoom, filters,
  search, selection and a table fallback.
- **Time** — an event scrubber and exact two-offset diff.
- **Schema** — the 78 node types and 38 edge types from `aether schema list`,
  with definitions fetched through `aether schema show`.
- **Coverage** — gap reasons and counts quoted verbatim from `validation report`
  or `vv coverage`; nothing is recomputed in the browser.
- **Command map** — a treemap of ten workspaces, 55 families and 124 commands.

Every view carries a lineage footer naming the source argv, exit code,
duration, event offset and chain head. Derived values are labelled. Empty
states are explicit. Geometry is represented as provenance only.

### API

`GET /api/atlas` aggregates `status`, `graph chain`, `graph as-of`,
`schema list`, `discover project-context` and `validation report` in one call
and returns a `sources` array. `/api/atlas/graph`, `/api/atlas/schema` and
`/api/atlas/health` expose the same data in smaller pieces. Responses are
cached briefly, capped at 5000 nodes and 10000 edges, and report truncation.

### Desktop shell

A separate Cargo workspace, `apps/desktop`, provides a native shell using
`wry`, `tao` and `muda`. The Axum server runs **in-process** on an ephemeral
loopback port with a per-launch in-memory token. WebView navigation is
restricted to that exact origin. The bundled `aether` CLI remains the only
backend. The desktop workspace requires Rust 1.92 so the main workspace can
keep its 1.75 minimum.

The macOS bundle includes the CLI, contracts, reference plugins, Python
sources and a relocatable CPython runtime with pinned wheels. Runtime network
access remains unnecessary; fetching Python and wheels is a build-time step
recorded with SHA-256 sums.

## Consequences

- New CLI commands and graph events appear in the Atlas without UI changes.
- `wry` adds desktop dependencies but keeps one UI codebase and no CDN.
- The bundle is larger (roughly 150 MB core, 500–700 MB with OCP/gmsh).
- A true 3D geometry viewer would require an additive read-only CLI export
  and is explicitly deferred.
- Windows and Linux packaging are later work; the shell itself is portable.

## Verification

`cargo test -p aether_web`, Node tests for layouts/treemap/atlas data,
`python3 infra/fitness/run.py` (including `web_surface` and `desktop_shell`),
`bash scripts/acceptance/web-ui.sh`, `bash scripts/acceptance/desktop.sh`,
`make app-smoke`, and a browser pass over Pulse, Atlas, Assurance, Build,
Govern, command index and terminal.
