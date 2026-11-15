# SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
# SPDX-FileCopyrightText: 2026 Crocell Company
"""Fuzz harness for the contract validation parser (alignment pass).

The SDP §3.5 and §19.5 call out the *manifest* and *policy* parsers
as needing fuzz coverage, alongside the AP242/CGNS format parsers
(Phase 3+ deliverables). In this codebase the only JSON parser
substrate that exists today is the contract validation harness
(``aether_sdk.validation``) — it is what every contract in
``contracts/`` is validated through, and it is what
``aether_contract_validate`` (the Rust half of the two-impl rule)
is paired with.

This fuzz harness:

1. Generates arbitrary JSON-compatible Python objects via
   ``hypothesis`` strategies.
2. Feeds each to ``ContractValidator.validate()`` for every
   contract in the catalog.
3. Asserts the harness does not raise an unhandled exception
   (the harness must reject malformed input gracefully, returning
   a ``ValidationResult`` with ``valid=False`` and error messages
   — not crashing).
4. Records the number of inputs tried and any crash for triage.

HONEST SCOPE
------------
The AP242/CGNS format parsers the SDP §3.5 / Phase 3 call out are
NOT in this codebase yet (they're G3 deliverables). The contract
validation harness is the *closest* parser substrate we have; this
harness exercises it under adversarial inputs as a stand-in. A
real AP242/CGNS fuzz harness is recorded as a G3 gap in
``workflows/paper-run/gaps.md``.

Bounded run: ``@settings(max_examples=200, deadline=5000)`` keeps
the harness from blocking CI on slow paths.
"""

from __future__ import annotations

import json
import sys
import traceback
from dataclasses import dataclass
from pathlib import Path
from typing import Any

# hypothesis is optional; we degrade gracefully if not present.
try:
    from hypothesis import HealthCheck, given, settings, strategies as st
    HYPOTHESIS_AVAILABLE = True
except ImportError:
    HYPOTHESIS_AVAILABLE = False


REPO_ROOT_DEFAULT = Path(__file__).resolve().parents[2]


@dataclass
class _Result:
    name: str
    ok: bool
    summary: str
    details: list[str]


# A strategy that generates arbitrary JSON-compatible Python
# objects. We use ``recursive`` to allow nested structures.
if HYPOTHESIS_AVAILABLE:
    json_value = st.recursive(
        st.none() | st.booleans() | st.floats(allow_nan=True, allow_infinity=True) | st.text(max_size=64),
        lambda children: st.lists(children, max_size=8) | st.dictionaries(st.text(max_size=16), children, max_size=8),
        max_leaves=64,
    )
else:
    json_value = None


# A list of contracts to fuzz. We pick the ones whose contract
# package has at least one schema (so the validate() call has a
# non-empty schema to test against).
def _eligible_contracts() -> list[tuple[str, str]]:
    """Return ``[(contract_id, schema_id), ...]`` for every contract
    in the catalog. ``schema_id`` is the first schema of the
    contract; the validator finds the schema by id.
    """
    out: list[tuple[str, str]] = []
    # Degrade gracefully when the SDK is not on sys.path (bare
    # `python3 infra/fitness/run.py`): add the in-repo SDK to sys.path
    # so the check never fails merely because of interpreter setup.
    repo_root = Path(__file__).resolve().parents[2]
    sdk_path = str(repo_root / "python" / "aether_sdk")
    if sdk_path not in sys.path:
        sys.path.insert(0, sdk_path)
    try:
        from aether_sdk.validation import ContractValidator  # type: ignore
    except ImportError:
        return out
    # Iterate via the same manifest catalog the validator uses.
    from aether_sdk.validation import CONTRACT_MANIFESTS  # type: ignore

    for cid in CONTRACT_MANIFESTS:
        try:
            v = ContractValidator(cid)
        except Exception:
            continue
        schemas = v.schemas()
        if not schemas:
            continue
        first = schemas[0]
        schema_id = first.get("id") if isinstance(first, dict) else str(first)
        if schema_id:
            out.append((cid, schema_id))
    return out


def check(repo_root: Path) -> _Result:  # type: ignore[name-defined]
    from run import CheckResult  # local import; avoid circular at module import

    if not HYPOTHESIS_AVAILABLE:
        return CheckResult(
            name="fuzz_contracts",
            ok=False,
            summary="hypothesis not installed; cannot run contract-parser fuzz harness",
            details=[
                "fix: `pip install hypothesis` in the Python venv (already installed in this environment)",
            ],
        )

    eligible = _eligible_contracts()
    if not eligible:
        return CheckResult(
            name="fuzz_contracts",
            ok=False,
            summary="no contracts with schemas found; cannot fuzz",
            details=[],
        )

    # Track crashes by contract/schema.
    crashes: list[str] = []

    @settings(max_examples=200, deadline=5000, suppress_health_check=[HealthCheck.too_slow, HealthCheck.filter_too_much])
    @given(value=json_value)
    def _fuzz_one(value: Any) -> None:
        from aether_sdk.validation import ContractValidator  # type: ignore

        for cid, schema_id in eligible:
            try:
                v = ContractValidator(cid)
                # The contract's schema may have a contract-specific
                # shape; the harness is what we are fuzzing, not
                # the contract — pass the value through and require
                # the harness to return a ValidationResult, not
                # raise an unhandled exception.
                _ = v.validate(value, schema_id=schema_id, document_path="<fuzz>")
            except Exception as exc:  # noqa: BLE001 — fuzz harness must catch all
                tb = traceback.format_exc(limit=3)
                crashes.append(f"{cid} / {schema_id}: {type(exc).__name__}: {exc} | {tb[-200:]}")

    try:
        _fuzz_one()
    except Exception as exc:  # noqa: BLE001
        # hypothesis raises the underlying assertion failure
        # wrapped in an ExceptionGroup or single Exception; record
        # the message and any captured crashes.
        msg = f"{type(exc).__name__}: {exc}"
        return CheckResult(
            name="fuzz_contracts",
            ok=False,
            summary=f"contract parser fuzz harness crashed: {msg}",
            details=crashes[:5] + [
                "fix: investigate the crash in the contract validation harness; "
                "the harness must reject malformed input without raising"
            ],
        )

    if crashes:
        return CheckResult(
            name="fuzz_contracts",
            ok=False,
            summary=f"contract parser fuzz harness captured {len(crashes)} unhandled exceptions in 200 examples",
            details=crashes[:5] + [
                "fix: the contract validation harness must catch its own exceptions and return ValidationResult instead of raising"
            ],
        )

    return CheckResult(
        name="fuzz_contracts",
        ok=True,
        summary=(
            f"contract parser fuzz harness: 200 examples × {len(eligible)} contract(s) — no unhandled exceptions; "
            f"scope: JSON contract validation harness (NOT the AP242/CGNS format parsers, which are G3+ deliverables)"
        ),
        details=[
            f"contracts fuzzed: {[c for c, _ in eligible]}",
            "scope note: this is the JSON contract validation parser; the AP242/CGNS format parsers the SDP envisions are not in this codebase and are a G3 gap",
        ],
    )


if __name__ == "__main__":
    import argparse
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--repo-root", type=Path, default=REPO_ROOT_DEFAULT)
    args = parser.parse_args()
    r = check(args.repo_root)
    print(f"[{'OK ' if r.ok else 'FAIL'}] {r.name}: {r.summary}")
    for d in r.details:
        print(f"       {d}")
    sys.exit(0 if r.ok else 1)
