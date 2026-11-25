# Aether — Local Web Interface

**Status:** deployable local console (ADR-0042).
**Scope:** a modern, minimal local UI over the untouched CLI.

## Start

```bash
cargo build -p aether_web
./target/debug/aether-web
```

The server prints a URL such as:

```
aether-web listening on http://127.0.0.1:8787
Open http://127.0.0.1:8787/#token=<64-hex-token>
```

Open that exact URL. The token is moved into `sessionStorage` and removed
from the address bar. Every API request carries it as a bearer token.

## What it exposes

- Overview: version, capability status, graph health, honest disclaimer.
- Nine curated workspaces with 45 commands and typed argument forms.
- Command palette (`⌘K` / `Ctrl-K`) across all 124 discovered commands.
- Raw command console, restricted to the CLI discovery surface.
- JSON inspector with syntax highlighting; stderr shown separately.
- Fail-closed states: exit 30 renders as `blocked` with the CLI's reason.

## Security

- Loopback only (`127.0.0.1`); no egress, no CDN, no telemetry.
- Static assets embedded in the binary with strict CSP.
- Bearer token with constant-time verification, 0600 on disk.
- No shell: argv elements are passed directly to the CLI process.

## Verification

```bash
bash scripts/acceptance/web-ui.sh
```

The script starts the server, checks auth, discovery, catalog, status, a
curated invocation, the blocked state, and raw-command validation.

## Limitations

- One CLI process per request; discovery and status are cached briefly.
- Long-running commands are synchronous; SSE streaming is future work.
- Curated workspaces require a catalog entry; all other commands remain
  available through the raw console and palette.
