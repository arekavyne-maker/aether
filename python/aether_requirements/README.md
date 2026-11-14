# aether-requirements

Aether Python SDK — requirements package. SysML v2 textual subset
(import/export) plus verification-coverage rollup (SDP §11.3, §3.6
invariant 7).

```python
from aether_requirements import SysMLFragment, VerificationReport

fragment = SysMLFragment.from_yaml("wing-rib.sysml.yaml")
report = fragment.verify_report()
print(f"Total: {report.requirements_total}")
print(f"Verified: {report.requirements_verified}")
for gap in report.gaps:
    print(f"  gap: {gap.requirement_id} ({gap.reason})")
```

The package produces a verification report that drives the
"verification-coverage" invariant in the graph engine (SDP §3.6
invariant 7): a `System` cannot transition to `released` if it has
`requirements` without a `verifies` path.
