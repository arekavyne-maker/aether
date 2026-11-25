# Aether — Local Web Console

**Status:** deployable local console (ADR-0042, ADR-0043).
**Scope:** an Obsidian-editorial local interface over the untouched CLI.

## Start

```bash
cargo build -p aether_web
./target/debug/aether-web
```

On an interactive terminal the server prints:

```
Open http://127.0.0.1:8787/#token=<64-hex-token>
```

Open that exact URL once. The token is moved into browser storage and removed
from the address bar, so afterward even a plain `http://127.0.0.1:8787/`
link works in this and later tabs. Every API request carries the token as
`Authorization: Bearer`. If the server restarts with a new token, the console
offers a **Clear saved token** action so a fresh printed URL can be used.
When stdout is not a terminal (service or CI), the URL is printed with a
reference to `~/.aether/web.token` instead — logs never contain the secret.

## Navigation

The rail has four primary destinations: **Home** (project health and
attention items), **Run** (task recipes plus the full command index), **Atlas**
(graph, schema, coverage and command map) and **Verify** (assurance and
evidence). The command index, CLI terminal (`⌘J`), session ledger and project
switcher sit in the Library section. `⌘K` opens the launcher across commands,
workspaces, graph nodes and actions. The project chip at the foot of the rail
opens the project switcher.

## Two surfaces, one session

- **Web workspace** — ten workspaces, all 124 discovered commands, grouped by
  CLI family. Selecting a command loads its real `aether <cmd> --help`
  contract and generates typed fields, a live argv preview and a result
  inspector with summary, JSON and stderr tabs.
- **CLI terminal** (`⌘J`) — a command line that accepts `aether` syntax. It
  tokenizes quotes and escapes into discrete argv elements, keeps history,
  completes command names and flags, and rejects shell operators with an
  explanation. It is not a shell and cannot invoke one.
- **Session ledger** — every invocation from either surface is recorded in
  this browser, with state, exit code, duration, rerun, inspect, copy and
  JSON export.

`⌘K` opens the palette across commands, workspaces, recent runs and actions.
`?` shows the full keyboard map.

## Configuration

| Variable | Default | Purpose |
|---|---|---|
| `AETHER_WEB_BIND` | `127.0.0.1:8787` | Loopback bind. Non-loopback requires `AETHER_WEB_ALLOW_REMOTE=1`. |
| `AETHER_WEB_TOKEN` | generated 0600 file | Explicit token; must be 64 lower-case hex characters. |
| `AETHER_WEB_PROJECT` | server working directory | Project root for child CLI processes and the path picker. |
| `AETHER_WEB_TIMEOUT_SECS` | `600` | Default command deadline; a request may clamp 1–3600. |
| `AETHER_WEB_FILES` | enabled | Set to `0` to disable the path picker. |

Every web invocation adds `--offline`. Structured mode adds `--json` unless
the caller supplied `--format`. Exit `30` is always rendered as `blocked`,
never as success.

## Security

- Loopback-only by default; bearer token with constant-time verification;
  0600 token file; strict CSP with no `unsafe-inline` or `unsafe-eval`.
- Static assets are embedded in the binary via `include_str!`; no CDN,
  external font, telemetry or network egress.
- No shell and no PTY. `/api/exec` resolves the longest discovered command
  prefix and caps argv count and size.
- The path picker is read-only, confined to the project root, skips symlinks
  and ignores generated/hidden directories.
- Streaming (SSE) is not implemented; long commands are synchronous with a
  bounded deadline.

## Atlas endpoints

| Route | Purpose |
|---|---|
| `GET /api/atlas?offset=` | Aggregate project, status, chain, graph, schema and coverage plus `sources` provenance |
| `GET /api/atlas/graph?offset=` | One graph snapshot |
| `GET /api/atlas/schema` | Node/edge type catalog |
| `GET /api/atlas/health` | `aether doctor` checks |

All are bearer-gated and shell-free. The graph is capped at 5000 nodes and
10000 edges; the response reports `truncated` rather than silently dropping
data.

## Live runs and project switching

| Route | Purpose |
|---|---|
| `POST /api/jobs` with `{"argv":[...]}` | Start one CLI invocation; returns a job id immediately |
| `GET /api/jobs/:id` | Current state, exit code, duration and final result |
| `GET /api/jobs/:id/events?after=N` | Authenticated long-poll of real stdout/stderr events |
| `POST /api/jobs/:id/cancel` | Terminate the job's process group |
| `GET /api/project` | Current project root |
| `POST /api/project` with `{"path":"..."}` | Switch the project root in-process |

The live dock shows real output, elapsed time and cancel. When the CLI writes
nothing, the console shows elapsed time rather than an invented progress bar.
After each run the console re-reads status, Atlas and health; an idle watcher
compares the chain head so external CLI runs also refresh the views. Jobs are
in-memory and do not survive a server restart.

A native macOS application is built from `apps/desktop`. See
[`desktop-app.md`](desktop-app.md).

## Verification

```bash
cargo test -p aether_web
node --test site/app/js/*.test.mjs site/app/js/viz/*.test.mjs site/app/js/data/*.test.mjs
python3 infra/fitness/run.py web_surface desktop_shell
bash scripts/acceptance/web-ui.sh
```

The acceptance script starts a loopback server and checks auth, CSP, the
merged surface, help parsing, positional argv execution, fail-closed exit
30, file-root confinement, the raw-console allowlist and the tokenizer.

## Files

| Path | Purpose |
|---|---|
| `crates/aether_web/src/api.rs` | Routes, auth, surface, describe, exec, files |
| `crates/aether_web/src/help.rs` | Defensive parser for clap help text |
| `crates/aether_web/catalog.json` | Workspaces, family map, curated presentation metadata |
| `site/app/js/` | Zero-build ES modules for shell, composer, terminal and palette |
| `site/app/app.css` | Obsidian night / Paper design system |
