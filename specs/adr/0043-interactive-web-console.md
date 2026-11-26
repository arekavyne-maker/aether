# ADR-0043: Interactive web console and dynamic command surface

**Status:** Accepted
**Date:** 2026-11-25
**Supersedes:** the curated-only surface of [ADR-0042](0042-local-web-interface.md)
**References:** [ADR-0010](0010-cli-canonical-interface.md) (I-10)

## Context

ADR-0042 delivered a local web adapter over the CLI, but the first interface
had three limits:

1. Its hand-maintained catalog covered 41 of 124 discovered commands and had
   already drifted (`configuration verify` no longer exists; `config verify`
   does).
2. The "raw console" joined all tokens into one command name and therefore
   could not pass positional arguments.
3. It offered forms, not a command line; the user still had to leave the
   browser to use the canonical CLI directly.

The request was to revamp the interface so that a user has both a CLI mode
and a web interface from which Aether can be used fully, without copy-paste.

## Decision

The web layer gains a **single command surface** shared by both interaction
modes:

- `GET /api/surface` merges CLI discovery with interface metadata into ten
  workspaces and all 124 commands, marking group commands and curated ones.
- `GET /api/describe?name=` runs `aether <cmd> --help` and parses clap's
  output into positionals, options, defaults and subcommands. Forms are
  generated from the CLI, not from hand-maintained metadata.
- `POST /api/exec` accepts a discrete argv array, resolves the longest
  discovered command prefix, and passes the remaining tokens as arguments
  and flags. This is the terminal's API; there is no shell and no PTY.
- `GET /api/files` offers read-only, root-confined, symlink-free path
  discovery for path arguments.
- The browser renders an embedded argv terminal (history, completion,
  quote-aware tokenization, shell-operator rejection), a master-detail
  workspace with a live argv composer, domain-aware result renderers, and a
  session ledger shared by both surfaces.
- Every web invocation passes `--offline`; structured mode adds `--json`.
  The web surface is deliberately stricter than the CLI.
- `AETHER_WEB_BIND` is limited to loopback unless an explicit remote opt-in
  is set; env-supplied tokens must be 64 lower-case hex characters; the
  token URL is printed only on an interactive terminal.

Long-running commands remain synchronous with a bounded, configurable
deadline. SSE streaming is deferred.

## Consequences

- New CLI commands appear in the console automatically; only presentation
  metadata needs curation.
- Help parsing is coupled to clap's text format. It is defensive: unknown
  lines are preserved as raw help, managed flags and the terminal remain
  available, and an `aether_cli` dependency is forbidden.
- `/api/exec` widens the command surface, but stays inside the discovery
  allowlist with argv count/size caps and a process timeout.
- A browser-side session ledger records both surfaces, making the
  "CLI underneath" relationship visible and auditable.
- Fitness check `web_surface` enforces parity, air-gap assets, strict CSP
  and the absence of a parallel CLI dependency.

## Verification

`cargo test -p aether_web`, `node --test experimental/web/app/js/argv.test.mjs`,
`python3 infra/fitness/run.py`, `bash scripts/acceptance/web-ui.sh`, and a
browser smoke pass covering overview, composer, terminal, palette, file
picker, themes and the session ledger.
