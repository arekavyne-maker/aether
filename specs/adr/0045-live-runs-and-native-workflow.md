# ADR-0045: Live runs, project switching and native desktop workflow

**Status:** Accepted
**Date:** 2026-11-25
**References:** [ADR-0044](0044-atlas-and-desktop-shell.md), [ADR-0043](0043-interactive-web-console.md), [audit/REPORT.md](../../audit/REPORT.md)

## Context

The audit recorded in `audit/REPORT.md` found that the console exposed generated forms for **all 82** CLI leaf commands, but every invocation was a blocking HTTP request with no incremental output, no cancellation, and no refresh of derived views after a mutation. The desktop shell was a thin WebView with no project switcher, no real accelerators, no recents and no window state. Those gaps — not form coverage — prevented an engineer from using the platform as an application.

## Decision

### Live jobs

`POST /api/jobs` starts exactly one `aether` invocation and returns a job id. The child is spawned as a process-group leader with piped stdout and stderr. `GET /api/jobs/:id/events?after=N` is an authenticated long-poll feed of real lifecycle and output events; `POST /api/jobs/:id/cancel` terminates the process group. Jobs are capped at 2000 events and 4 MB of output; truncation is reported, never silent. The CLI remains the only backend and no shell is involved.

The browser follows the event cursor, renders a persistent live dock with real output, elapsed time and cancel, then opens the result and re-reads status, Atlas and health. An idle watcher compares the real chain head so commands run outside the console also refresh the views. No progress percentage is invented: when the CLI writes nothing, the UI shows only elapsed time.

### In-app project switching

`GET`/`POST /api/project` expose and change the runner's project root. The root is held in a shared lock inside `CliRunner`, so every cloned runner (surface, atlas, jobs) sees the switch. Caches and cached help are invalidated. The console offers a project screen with a path field and recent projects; the desktop File menu can drive the same endpoint.

### Simpler navigation

The rail has four primary destinations — Home, Run, Atlas, Verify — with the command index, terminal, ledger and project switcher as secondary library entries. The Run surface starts with ten argv-exact task recipes and then the full 124-command index. All routed views remain available by hash.

### Native desktop workflow

`muda` menus now use stable string ids and real accelerators (Open Project ⌘O, Run ⌘↵, Cancel ⌘., Terminal ⌘J, Reload ⌘R, Quit ⌘Q). Recent projects and window geometry are persisted under `~/Library/Application Support/Aether/`. View, Help and Diagnostics entries navigate the embedded console rather than duplicating its logic. The per-launch token and navigation allowlist are unchanged.

## Consequences

- Long commands now give immediate feedback and can be cancelled; their children are not orphaned.
- Command output is visible while it is produced, but there is still no synthetic progress — that would require additive CLI events, which are out of scope.
- Switching projects is in-process; no server restart or token rotation is needed.
- The job registry is in-memory: restarting `aether-web` discards old jobs, while the browser ledger remains.
- Desktop state files contain only paths and window geometry, never project data or tokens.

## Verification

`cargo test -p aether_web` (38 lib tests including a real streaming job), Node tests including `jobs.test.mjs`, `python3 infra/fitness/run.py` with live-job and native-menu invariants, `bash scripts/acceptance/web-ui.sh` with a job lifecycle assertion, browser passes over Run, recipes, live dock, terminal, project switching and populated Atlas, and `cargo build --manifest-path apps/desktop/Cargo.toml`.
