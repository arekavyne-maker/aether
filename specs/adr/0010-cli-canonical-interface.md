# ADR-0010: I-10 CLI Canonical Interface

- **Status:** Accepted
- **Date:** 2026-08-30
- **Deciders:** Aether Architecture Review Board
- **Supersedes:** —
- **Superseded by:** —

## Context

Multiple interfaces (CLI, GUI, programmatic API) inevitably drift unless one is canonical and the others are derived from it. Drift is where bugs hide.

## Decision

The CLI is the **canonical interface** to the platform. The CLI command tree is a **published schema**, versioned, consumed by documentation generators, conformance tests, and GUI clients.

Every CLI capability SHALL be available programmatically through the same underlying API surface. There SHALL NOT be a parallel implementation behind the GUI.

GUI clients (e.g., the Phase-6 graph browser; SDP §15.4) consume only:

- The CLI command JSON schemas
- The graph API
- Event streams

The GUI renders; it does not compute. Any GUI capability missing from the CLI is a **bug in the GUI**, not the other way around.

CLI conventions are normative (SDP §15.1):

- `aether <domain> <verb> [target] [--flags]`
- `--json` for machine-readable output (stable per-command schema, versioned)
- `--format table|json|yaml|csv`
- Deterministic exit codes: `0` ok · `10` validation · `20` policy denial · `30` contract violation · `40` resource · `50` internal
- `--as-of`, `--project <urn>` universally supported where meaningful
- Config precedence: flags > env (`AETHER_*`) > project (`.aether/config.toml`) > user > defaults
- `AETHER_AGENT=1` marks non-interactive contexts (no pagers, no color, strict errors)
- Shell completions for bash/zsh/fish generated from the command spec

**GUI/CLI parity tests** (in `infra/fitness/gui_cli_parity.py`) exercise the same service layer from both surfaces. CLI command coverage metric is tracked per release.

## Consequences

- Positive: GUI drift is mechanically detected
- Positive: scripts, SDKs, and GUIs all share one implementation
- Negative: a GUI feature must be expressed as a CLI command first
- Negative: shell completion generation must be maintained

## Enforcement

- GUI/CLI parity tests in fitness pipeline
- CLI command coverage metric tracked per release
- Command tree schema published (`docs/cli/spec.json`); generated docs and completions are CI artifacts

## References

- SDP §2 I-10
- SDP §15 (CLI Architecture)
- SDP §15.4 (Optional GUI Contract)
- SDP §19.5
- ADR-0001 (microkernel minimality)
