# Deprecation policy

A contract method, contract itself, plugin, or AWL feature is
**deprecated** rather than removed. The deprecation lifecycle:

1. **Announce**: a deprecation is announced in the release notes
   for the minor version that introduces it. The release notes
   specify the planned removal version.
2. **Warn**: deprecated methods log a warning on every call. The
   warning identifies the deprecation, the recommended replacement,
   and the planned removal version.
3. **Continue**: deprecated methods continue to work in the current
   LTS branch.
4. **Remove**: deprecated methods are removed in the next major
   version. The release notes for that major version document the
   removal.

## What can be deprecated

- A specific method on a contract.
- An entire contract.
- A specific plugin (the kernel still loads the bundle, but flags
  it as deprecated in the registry).
- An AWL feature (e.g. a v1.2 feature that v1.3 replaces).

## What cannot be deprecated

- A contract's structural invariants (the schema). If a schema
  must change incompatibly, a new contract id is required (or the
  contract is bumped to a new major version with the old one
  retained as deprecated).
- The platform's twelve invariants (I-1 through I-12). Deprecating
  an invariant is a fork, not a release.
- The provenance chain. Every artifact ever produced is part of
  the chain; the chain is append-only.

## How to deprecate

For a method:

```yaml
# In contracts/<contract>/README.md or schema/<name>.schema.json
deprecated:
  method: "execute_v1"
  replaced_by: "execute"
  removal_version: "v2.0"
  since: "v1.2"
  migration: |
    Replace execute_v1 with execute. The v2 input/output schemas are
    backward-compatible at the JSON level; the change is that
    execute now returns the full v1.2 result shape.
```

For a contract:

```yaml
deprecated:
  contract: "aether.legacy.foo"
  replaced_by: "aether.bar"
  removal_version: "v2.0"
  since: "v1.0"
```

For a plugin (the kernel marks the bundle as deprecated in the
registry):

```yaml
# In contracts/plugin.manifest/examples/<plugin>.manifest.yaml
plugin:
  ...
  deprecated:
    replaced_by: "org.example.successor"
    removal_version: "v2.0"
    since: "v1.2"
```

The conformance test for the deprecated bundle continues to pass
(deprecation does not change behavior); the dark-build CI
additionally asserts that the deprecation metadata is present and
consistent with the deprecation policy.

## How to handle a deprecation in your code

- **Plugin authors**: when a contract method is deprecated, ship
  a new plugin version that uses the replacement. Maintain the
  old plugin version for the LTS duration.
- **Site operators**: when running a workflow that uses a deprecated
  method, the warning identifies the replacement. Plan the upgrade.
- **Workflow authors**: the AWL validator flags deprecated methods
  in `aether workflow validate`.

## What "removal" means

When a deprecated method is removed:

- The contract schema no longer includes the method.
- The conformance test asserts the method is no longer present.
- The plugin manifest cannot declare the method.
- A workflow that uses the method fails to validate.

The dark-build CI is the structural guarantee: a removed method
in a new major version triggers a conformance failure on any
plugin that still declares it.
