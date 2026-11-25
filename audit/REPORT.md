# Aether audit — web console and macOS application

**Date:** 2026-11-25
**Method:** live probes against the running console and CLI, source review, and the existing gate suite.
**Rule:** no finding is inferred from a claim; every item below has a source or measurement.

## Verdict

The platform is internally consistent and evidence-native, but the **interaction layer does not match the platform's capability**. All 82 leaf commands already have generated argument forms, and every view carries CLI provenance. What is missing is *liveness*, *task framing*, and *native desktop behaviour*.

## Confirmed strengths

- **82/82 leaf commands** expose a generated form through `GET /api/describe` (measured; see [`task-coverage.md`](task-coverage.md)). The curated 41-command catalog is metadata, not a form limit.
- Every action is discrete argv against the canonical CLI; no shell, no network, no parallel implementation.
- Strict CSP without `unsafe-inline`/`unsafe-eval`; all assets embedded; air-gap fitness check passes.
- `exit 30` is surfaced as `blocked`, never as success; provenance footers name argv, exit code, duration, offset and chain head.
- Empty states and explicit truncation are present; geometry is provenance-only.
- Keyboard focus rings and reduced-motion support exist in `app.css`.
- The desktop shell keeps its per-launch token in memory and restricts navigation to the exact loopback origin.

## Findings

| ID | Severity | Finding | Evidence | Fix |
|---|---|---|---|---|
| F-01 | **P0** | Every invocation is a blocking request (600 s default). No streaming output, elapsed time, or cancellation. Long work (`benchmark`, `uq sobol`, `optimize`, `dark-build-check`) appears frozen. | `crates/aether_web/src/api.rs` `run_and_render*`, `cli.rs` `run_with_timeout`; no job route | Add an in-process job registry with long-poll event feed, incremental stdout/stderr, elapsed, and process-group cancel (Phase 2). |
| F-02 | **P1** | Atlas, Pulse and Assurance are fetched once at boot. After a command mutates the graph, the UI shows stale counts until a manual reload. | `site/app/js/main.js` `boot()`, `runComposer()` only calls `showResult()` | Invalidate and re-fetch derived views on every `job.finished`. |
| F-03 | **P1** | Project cannot be changed in-app. `AETHER_WEB_PROJECT` / a desktop relaunch is required. | `cli.rs` `CliRunner::project`, desktop `resolve_project()` at launch | Add an authenticated project switch endpoint with shared runner state; expose it in both UIs. |
| F-04 | **P1** | Desktop shell is a thin WebView: no project picker, recents, window state, real accelerators, notifications, health panel, or About. Menus are keyed by display label with no accelerators. | `apps/desktop/src/main.rs` `build_menu()`, `parse_args()`, `main()` | Native menu ids/accelerators, project picker, recents, window state, health and About (Phase 4). |
| F-05 | **P2** | Nine navigation targets with mixed vocabulary (six destinations plus index, terminal, ledger). Tasks such as "verify a requirement" or "export evidence" require assembling flags manually. | `main.js` `renderShell()` | Four primary surfaces, palette-first navigation, and argv-exact task recipes (Phase 3). |
| F-06 | **P2** | No task-level guidance for approval commands (`evidence gate`, `model verify`, `agent-run`, `release prepare`); the generic authorization checkbox does not capture an approver identity. | `main.js` `renderComposer()`, `updateArgvPreview()` | Dedicated approval flow with explicit identity and audit entry. |
| F-07 | **P2** | Bearer token is persisted in `localStorage` so plain links work. Strict CSP mitigates script injection, but the secret is readable by any script on the origin. The desktop keeps its token only in memory, which is preferable. | `site/app/js/api.js` `store()` | Keep behaviour (explicitly requested), but document exposure and add a one-shot mode for shared machines. |
| F-08 | **P2** | No long-poll/SSE events, so external CLI runs (typed in a real terminal) are invisible to the console. | `main.js` has no watcher | Watch the real chain head and refresh when it changes. |
| F-09 | **P3** | Large-graph behaviour is bounded server-side (5000/10000 caps) but the client renders the whole SVG; no virtualization benchmark is recorded. | `viz/graph.js` | Add a measured performance budget and a table fallback for oversized graphs. |

## Scope correction

The pre-audit estimate of "50% command coverage" counted only the 41 curated catalog entries. The composer is driven by `discover` + `describe`, so **form coverage is already 100% of leaf commands**. The real gap is that those forms are buried in a command-centric index and cannot run live. Remediation therefore targets liveness and task framing rather than form generation.

## Gate baseline

`cargo test --workspace` 100 binaries; `pytest` 304; fitness 18/18 (1 optional skip); 23 acceptance scripts; dark-build all checks; 17 Node tests. CLI source unchanged from `634618a`.

## Remediation status

| Finding | Status |
|---|---|
| F-01 | **Fixed** — job registry, authenticated long-poll events, process-group cancel, elapsed and live stdout/stderr; acceptance asserts the lifecycle. |
| F-02 | **Fixed** — `refreshDerived()` re-reads status, Atlas and health after every job. |
| F-03 | **Fixed** — shared `CliRunner` project root plus `GET`/`POST /api/project` and a console project screen. |
| F-04 | **Fixed** — stable menu ids with accelerators, recents, window state, project menu, diagnostics and About. |
| F-05 | **Fixed** — Home / Run / Atlas / Verify rail, palette-first library, ten argv-exact recipes. |
| F-06 | **Partial** — approval commands still use the existing explicit-authorization checkbox; a dedicated approver-identity flow remains deferred. |
| F-07 | **Accepted** — localStorage token behaviour was explicitly requested; documented; desktop token remains in memory. |
| F-08 | **Fixed** — idle chain-head watcher with automatic refresh. |
| F-09 | **Deferred** — server caps remain; the client still renders the full SVG (table fallback exists). |
