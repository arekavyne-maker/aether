# ADR-0042: Local web interface as a CLI adapter

**Status:** Accepted
**Date:** 2026-11-25

## Context

Spec §12 makes the CLI the canonical interface and permits GUI/TUI clients
provided they call the same APIs. A user asked for a comprehensive local web
interface that leaves the CLI untouched. A previous proposal to refactor
`cmd_*` functions into value-returning APIs would have modified the CLI
source, so it was rejected.

## Decision

`crates/aether_web` is a separate local HTTP service that:

- binds to 127.0.0.1 only;
- spawns the `aether` binary with discrete argv elements, never a shell;
- passes `--format json` and maps exit 0/2/30 into `ok`/`usage_error`/`blocked`;
- uses a declarative, interface-only catalog of curated commands and
  argument templates;
- exposes all discovered commands through a validated raw-console endpoint;
- requires a bearer token stored in `~/.aether/web.token` mode 0600;
- embeds static assets with strict CSP and no external requests.

The CLI source, command tree, golden tests, and JSON contract are unchanged.

## Consequences

- No duplicate engineering logic: every action is a real CLI invocation.
- A process is spawned per request; acceptable for a local engineering
  console and mitigated by caching discovery/status.
- New CLI commands appear automatically in the raw console; curated
  workspaces need a catalog entry.
- The web layer cannot bypass policy: exit 30 is rendered as Blocked.
- Long-running commands are currently synchronous; SSE is future work.
