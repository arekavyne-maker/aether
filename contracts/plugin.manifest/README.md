# `aether.plugin.manifest` Contract v0

Plugin manifest schema (SDP §5.2). Every plugin declares its identity, capabilities, resources, isolation profile, runtime, and provenance template. The manifest is the only way a plugin enters the registry.

## Versioning

- **Contract id:** `aether.plugin.manifest`
- **Schema version:** `0.1.0` (Phase 0 v0; v1.0 freeze at Phase 2 G2)
- **Semver discipline:** see ADR-0011

## Layout

```
plugin.manifest/
├── manifest.json          # contract package manifest
├── schema/
│   └── manifest.schema.json
├── examples/
│   └── openfoam-adapter.manifest.yaml
└── README.md
```

## Two-implementation rule (ADR-0022)

The contract is not stable until two independent implementations parse and validate it:

- **Rust parser:** `python/aether_testing/` (cross-language) + Rust via `jsonschema` crate (Phase 2 binding)
- **Python parser:** `jsonschema` Python package

Both parsers are run on every contract schema in CI.

## See also

- SDP §5.2
- SDP §5.4
- SDP §5.5
- `../../specs/adr/0011-contracts-versioned-tested.md`
- `../../specs/adr/0022-two-implementation-rule.md`
