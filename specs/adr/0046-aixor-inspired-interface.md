# ADR-0046: AIXOR-inspired console and native window

**Status:** Accepted
**Date:** 2026-11-25
**References:** [ADR-0045](0045-live-runs-and-native-workflow.md), [audit/REPORT.md](../../audit/REPORT.md)

## Context

The console was functionally complete but visually belonged to a different
design lineage. The request was to change the entire design of both the web
console and the macOS application, keep every function and every command, and
follow the artistic direction of the AIXOR template exactly in its visual
language. Only the visual language is borrowed; no agency copy, people,
awards or pricing are fabricated into Aether.

## Decision

### Visual language

The Obsidian palette and serif-forward hierarchy are replaced by the measured
AIXOR direction: a `#000` canvas, `#1f1f1f → #000 → #1f1f1f` gradient cards,
white ink, grey `#a7a7ad / #71717a`, periwinkle `#959efe` accent, deep navy
`#00031c` chrome, and pale `#f4f7ff` for the paper theme. Typography is
**Urbanist** (400–900) with **Arapey italic** for one display phrase, 10px
uppercase micro-labels with wide tracking, Roman numeral section indices,
pill controls, 18px cards, hairline dividers and oversized numerals.

Fonts are committed as WOFF2 files under `experimental/web/app/fonts/`, embedded in the
binary and served only from `/assets/fonts/*file`. There is no runtime font
fetch and no CDN. Licences and SHA-256 sums are recorded beside them.

### Layout

The left rail becomes a sticky pill navigation bar across the top, with a
separate dark titlebar for the desktop app. Content stays within a 1440px
measure with 72px gutters, hero sections occupy the viewport, and workspaces,
Atlas panels, assurance strips and health cards use the gradient-card
language. The light theme becomes a paper variant of the same system rather
than a separate design.

### Native window

The `wry` window is undecorated. A custom in-app titlebar carries macOS
traffic-light buttons, the project name and a drag region. Clicks travel
through wry's IPC handler to native actions (`drag`, `min`, `zoom`, `close`);
stable menu ids and accelerators remain. Window geometry and recents are
still stored locally, never secrets or engineering data.

### Functionality contract

Nothing is removed. All 124 discovered commands, 82 generated leaf forms,
42 group drill-downs, approval gates, long jobs, live output, cancellation,
project switching, Atlas tabs, terminal, ledger and recipes remain reachable.
The design change is presentation-only; the CLI boundary, APIs and job
registry are unchanged.

## Consequences

- The console looks and feels like a product rather than an adapter, in both
  the browser and the native window.
- Font binaries add ~78 KB to the embedded bundle; no network is required.
- An undecorated window needs custom drag and traffic lights; the IPC path is
  covered by source invariants and a bundle smoke check, while native menu
  behaviour remains the reliable fallback.
- Visual verification is done through computed styles, DOM checks, both
  themes and a narrow viewport; pixel comparison is not part of this repo.

## Verification

`cargo test -p aether_web` (font embedding and asset tests), Node tests,
`python3 infra/fitness/run.py` (font embedding, air-gap, custom titlebar),
`bash scripts/acceptance/web-ui.sh`, browser pass over all destinations and
both themes with no console errors, `cargo build --manifest-path experimental/desktop/Cargo.toml`
and `make app-smoke`.
