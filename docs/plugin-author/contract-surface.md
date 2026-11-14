# Plugin author: the contract surface

Every Aether contract declares a method surface (the JSON-RPC
methods the plugin must implement), a set of JSON Schemas (the
input/output types), and example documents (the conformance
fixture). This guide covers the universal parts of every
contract; the per-contract guide is in each contract directory
(`contracts/<contract_id>/README.md`).

## The universal envelope

Every contract has these four methods:

- `init(params) -> {ok, plugin_id, plugin_version}` — receives
  the RunToken, performs handshake (signature check, scope
  validation). The kernel calls this first.
- `discover(params) -> {plugin_id, plugin_version, contract_id,
  contract_version_range, kind, determinism_class, ...}` —
  returns the plugin's identity and capabilities. The kernel
  caches this for registry queries.
- `capabilities(params) -> {methods, distributions, machines, ...}` —
  returns a structured facts dict the registry matches against.
- (contract-specific primary method) — `execute` for
  physics.solver, `propagate` for uq.method, `train` for
  surrogate.builder, `run` for test.rig, `acquire` for
  inspection.device, `plan` for manufacturing.process, `recommend`
  for si.governor, `optimize` for optimizer, `estimate` for
  twin.estimator, `import` for geometry.kernel, `generate` for
  mesh.engine.

## The RunToken

The `init` method receives a `RunToken`:

```json
{
  "token_id": "uuid",
  "workflow_id": "uuid",
  "step_id": "string",
  "actor": "string",
  "issued_at": "rfc3339",
  "expires_at": "rfc3339",
  "scope": {
    "read_paths": ["..."],
    "write_paths": ["..."],
    "allowed_methods": ["execute", "init"]
  },
  "keyid": "aether-prod-2026-03",
  "signature_b64": "ed25519-sig"
}
```

The plugin must verify the Ed25519 signature against the
site's pinned public key (the envman `toolchains` block holds the
key). The conformance fixture's reference plugins verify
signature presence and key id; production plugins verify the full
Ed25519 chain.

## The discovery response

```json
{
  "plugin_id": "org.example.foo",
  "plugin_version": "0.1.0",
  "contract_id": "aether.physics.solver",
  "contract_version_range": ">=0.1.0 <1.0.0",
  "kind": "reference",
  "determinism_class": "bit-reproducible",
  "notes": "..."
}
```

The capability registry matches the plugin's declared
capabilities against workflow step requirements. The
`determinism_class` is critical: `bit-reproducible` means strict
mode can replay the run; `documented-equivalent` means strict mode
flags the run as "equivalent only".

## The execution envelope

When the kernel calls the primary method (e.g. `execute`), it
sends:

```json
{
  "jsonrpc": "2.0",
  "method": "execute",
  "params": {
    "step_id": "string",
    "inputs": { ... contract-shaped payload ... }
  },
  "id": "uuid"
}
```

The plugin's response is:

```json
{
  "jsonrpc": "2.0",
  "id": "uuid",
  "result": {
    "status": "succeeded" | "failed",
    "result": { ... contract-shaped result ... },
    "outputs": { ... CAS-pointered outputs ... },
    "plugin_id": "string",
    "plugin_version": "string",
    "determinism_class": "string"
  }
}
```

A `failed` result with a populated `error` field is a contract
violation (I-12, fail-visible). The kernel records the error and
stops the workflow; the failure is itself provenance.

## Stream JSON-RPC, not async

Aether uses **newline-delimited JSON-RPC 2.0** over stdio. The
plugin reads one line at a time, processes it, writes one response
line. The kernel flushes after every line. Plugins must do the
same; a buffered stdout breaks the protocol.

Reference implementations in
`reference_plugins/<plugin>/python/stdio_plugin.py` show the
idiomatic Python pattern. Production plugins in C++ or Rust follow
the same pattern; the kernel does not assume a language.

## Lifecycle

A plugin's lifetime is one workflow run. The kernel spawns a fresh
process per run (or pools across runs with timeouts). Plugins
should not assume state persists across `init` calls; the kernel
re-spawns on each run for I-5 (air-gap) and I-12 (fail-visible).

## Error taxonomy

- **NetworkFailure** (`ResourceUnavailable` in the kernel
  taxonomy): the plugin tried to access a network resource in
  offline mode.
- **SchemaViolation** (`ContractViolation`): the plugin's response
  does not match the contract schema.
- **ComputationFailure** (`PluginCrash`): the plugin raised an
  unhandled exception.
- **Timeout** (`ResourceUnavailable`): the plugin did not respond
  within the heartbeat.

All errors are fatal to the run and recorded as provenance. The
kernel does not retry (retries are an AWL step-level concern,
handled by the workflow executor, not the dispatcher).
