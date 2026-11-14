# aether-testing

Conformance runner and contract validator. Phase 0: drives the contract validation harness across all `contracts/` packages. Phase 2: extends to plugin lifecycle, MVR execution, and offline TUF bundle verification.

## Phase 0 usage

```bash
cd python/aether_testing
pip install -e ".[dev]"
pytest
```

The conformance suite enforces that every contract in `contracts/` validates and that example documents in each contract's `examples/` directory pass schema validation. A Rust implementation of the same harness is added in Phase 2; both must agree on every document (the two-impl rule, ADR-0022).
