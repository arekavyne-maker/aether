# Plugin author: getting started

This guide covers the minimum a plugin author needs to ship a
working Aether plugin. The full contract surface is in
`contract-surface.md`; the TUF-signed bundle workflow is in
`signed-bundles.md`.

## What a plugin is

A plugin is a process that:

1. Speaks newline-delimited JSON-RPC 2.0 over stdio (ADR-0015).
2. Implements one or more contract methods (declared in its
   manifest).
3. Is signed by the offline TUF registry (or the local test
   registry in dev).
4. Has a content-addressed bundle in the artifact root.

The kernel spawns the plugin per workflow run, exchanges `init`
then `execute` (or the contract's primary method) over stdio, and
tears the process down.

## A minimal plugin (Python)

The fastest path to a working plugin is the reference SDK
(`aether_executor.handlers.StdioPluginHandler` for Rust, or the
Python conformance fixture in
`reference_plugins/<your_plugin>/python/stdio_plugin.py`).

A minimal example (a 5-line plugin):

```python
import json, sys

def method_execute(params):
    return {"status": "succeeded", "result": {"answer": 42}}

for line in sys.stdin:
    req = json.loads(line)
    method = req.get("method")
    if method == "init":
        result = {"ok": True, "plugin_id": "org.example.minimal", "plugin_version": "0.1.0"}
    elif method == "execute":
        result = method_execute(req.get("params", {}))
    elif method == "discover":
        result = {"plugin_id": "org.example.minimal", "plugin_version": "0.1.0"}
    else:
        result = {"error": f"unknown method: {method}"}
    sys.stdout.write(json.dumps({"jsonrpc": "2.0", "id": req.get("id"), "result": result}) + "\n")
    sys.stdout.flush()
```

The conformance fixture for any contract asserts the plugin
implements the contract's full method surface. The example above
is a **stub**; the conformance test will fail on it.

## Plugin manifest

Every plugin has a manifest (aether.plugin.manifest) declaring:

- `id`: reverse-DNS, immutable
- `version`: semver
- `contracts`: which contracts the plugin implements
- `capabilities`: structured facts the registry can match
- `isolation`: process / container / vm
- `runtime`: entry point + env_manifest_ref
- `signature`: Ed25519 over the canonical manifest (TUF-signed)

See `contracts/plugin.manifest/examples/openfoam-adapter.manifest.yaml`
for a complete example.

## Workflow

The standard plugin-author workflow:

1. **Design the contract interface**: which contract does the
   plugin implement? Which methods?
2. **Implement against the conformance fixture**: the
   `python/aether_testing/tests/test_contract_conformance.py`
   test must pass for every example document.
3. **Author the manifest**: id, version, contracts, capabilities,
   isolation, runtime, env_manifest_ref.
4. **Build the bundle**: a tarball containing the manifest, the
   plugin entry point, and any required libraries.
5. **Sign the bundle**: TUF signs the bundle; the envman
   StrictVerifier asserts the content hash on every workflow run.
6. **Install on the dev site**: `aether envman install <bundle>
   <env_manifest> --artifact-root <root>`.
7. **Verify reproducibility**: `aether envman verify <env_manifest>
   --artifact-root <root>`; the verifier reports `all_resolved:
   true`.
8. **Run a smoke workflow**: `aether workflow run <smoke> --project
   <project>`; the workflow must complete with all steps
   `succeeded`.

## What the conformance fixture gives you

- 19 contract manifests, each with method surface + schemas
- One or more example documents per contract
- A Rust validator (`aether-contract-validate`) that loads every
  schema and validates every example
- A Python validator that does the same (the two-implementation
  rule; the conformance test passes only when both agree)

A green `cargo test -p aether_contract_validate` is the structural
guarantee that your plugin fits the contract.

## The two-implementation rule (ADR-0022)

A contract is not stable until two independent plugins pass the
conformance suite. When you ship the first plugin, you are
demonstrating the contract; when you ship the second, you are
demonstrating that the contract is *implementable* (not just a
description of one vendor's plugin).

If your plugin is the only one for a contract, the contract is
provisional. The dark-build CI tags it as "G0" (one-impl) until
the second plugin ships.
