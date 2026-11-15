# SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
# SPDX-FileCopyrightText: 2026 Crocell Company
"""OpenFOAM adapter — thin shell-wrapper around cfd-reference-minimal.

PLUGIN IDENTITY
---------------
Manifest id: ``com.acme.cfd.opensfoam`` (declared by the example
``contracts/plugin.manifest/examples/openfoam-adapter.manifest.yaml``;
this is the public, contract-facing name).

Internal plugin id: ``org.aether.reference.fe.opencfd`` (this is the
name the kernel sees during process supervision and provenance — it
must be distinct from the underlying fixture to make the indirection
visible in the audit log).

WHAT THIS PLUGIN ACTUALLY DOES
------------------------------
This is **not** a real OpenFOAM wrapper. No ``simpleFoam``, ``pimpleFoam``,
or any OpenFOAM binary is invoked. The ``execute`` method shells out
to the ``cfd-reference-minimal`` stdio plugin and relays the
JSON-RPC envelope.

This deliberate two-layer setup is a **conformance-only fixture**
(SDP §5.6, §21): it lets the dark-build exercise two distinct
plugin ids under the same physics contract without claiming that
either one is a production solver. Production OpenFOAM integration
requires a real binary adapter that maps OpenFOAM case files to the
``physics.solver`` contract — that work is tracked in
``workflows/paper-run/gaps.md`` and is out of scope for the
Phase 0–2 substrate.

FIDELITY-HONEST LABEL (I-7)
---------------------------
Because no real OpenFOAM solver is invoked, the result bundle
inherits the fidelity class of the underlying fixture
(``cfd-reference-minimal``). When the upstream fixture is upgraded
to a real Poiseuille-flow case (see Priority 2 work in
``workflows/paper-run/gaps.md``), this wrapper relays the upgraded
fidelity automatically — the contract surface is unchanged.
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
import time
import uuid
from typing import Any


# The kernel sees this id. Distinct from the cfd-reference-minimal
# id so the provenance log makes the indirection visible.
PLUGIN_ID = "org.aether.reference.fe.opencfd"
PLUGIN_VERSION = "0.2.0"
CONTRACT = "aether.physics.solver"

# Path to the wrapped fixture, relative to this plugin. Resolved at
# execute() time so the wrapper works whether the kernel invokes it
# from the repo root, from a packaged bundle, or from a CAS.
WRAPPED_PLUGIN = (
    os.environ.get("AETHER_CFD_REF_MINIMAL_PLUGIN")
    or "reference_plugins/cfd-reference-minimal/python/stdio_plugin.py"
)


def _err(req_id: Any, code: int, message: str) -> dict[str, Any]:
    return {"jsonrpc": "2.0", "id": req_id, "error": {"code": code, "message": message}}


def _ok(req_id: Any, result: dict[str, Any]) -> dict[str, Any]:
    return {"jsonrpc": "2.0", "id": req_id, "result": result}


def method_discover(_params: dict[str, Any] | None) -> dict[str, Any]:
    return {
        "plugin_id": PLUGIN_ID,
        "plugin_version": PLUGIN_VERSION,
        "contract_id": CONTRACT,
        "contract_version_range": ">=0.1.0 <1.0.0",
        "kind": "reference-shell-wrapper",
        "wraps": "org.aether.reference.cfd.minimal",
        "wrapped_plugin_path": WRAPPED_PLUGIN,
        "isolation": "process",
        "determinism_class": "bit-reproducible",
        "note": (
            "Phase 0–2 conformance shell-wrapper; not a real OpenFOAM "
            "adapter. Production OpenFOAM integration is a G2/G3 deliverable "
            "tracked in workflows/paper-run/gaps.md."
        ),
    }


def method_capabilities(_params: dict[str, Any] | None) -> dict[str, Any]:
    # The shell-wrapper forwards the capabilities of the wrapped plugin.
    # The capabilities of cfd-reference-minimal are documented in that
    # plugin's source; we declare a conservative subset here so the
    # registry can match this plugin against workflow step requirements
    # that look for the openfoam manifest's declared capability set.
    return {
        "physics": {
            "fluid": True,
            "compressible_flow": False,
            "incompressible_flow": True,
            "turbulence": False,
            "heat_transfer": False,
            "multiphase": False,
        },
        "compute": {
            "cpu": True,
            "gpu": False,
            "mpi_distributed": False,
            "accelerator_optional": False,
        },
        "features": {
            "adjoint": False,
            "autodiff": False,
            "uncertainty_quantification": False,
            "restart": False,
            "checkpoint": False,
        },
    }


def method_validate(params: dict[str, Any] | None) -> dict[str, Any]:
    if params is None:
        return {
            "valid": False,
            "checks": [
                {"name": "input_present", "kind": "sanity", "result": "fail",
                 "diagnostic": "no problem spec provided"}
            ],
        }
    checks: list[dict[str, Any]] = []
    required = ["problem_id", "physics_class", "geometry", "boundary_conditions", "solver_settings"]
    for field in required:
        if field not in params:
            checks.append({
                "name": f"required:{field}",
                "kind": "sanity",
                "result": "fail",
                "diagnostic": f"missing required field: {field}",
            })
        else:
            checks.append({
                "name": f"required:{field}",
                "kind": "sanity",
                "result": "pass",
            })

    if params.get("physics_class") != "fluid":
        checks.append({
            "name": "physics_class_supported",
            "kind": "sanity",
            "result": "warn",
            "diagnostic": f"openfoam shell-wrapper only supports 'fluid'; got {params.get('physics_class')!r}",
        })

    overall = "pass"
    for c in checks:
        if c["result"] == "fail":
            overall = "fail"
            break
        if c["result"] == "warn" and overall == "pass":
            overall = "warn"

    return {"valid": overall != "fail", "checks": checks, "overall_status": overall}


def method_execute(params: dict[str, Any] | None) -> dict[str, Any]:
    """Shell out to the wrapped cfd-reference-minimal plugin and relay
    its JSON-RPC result.

    The wrapper does not invent a result: whatever the underlying
    fixture returns (including its own ``verification`` block, which
    now performs a real analytical check for the Poiseuille case —
    see ``cfd-reference-minimal/python/stdio_plugin.py``) is passed
    through with the wrapper's plugin_id stamped on it so the
    provenance log records the indirection.
    """
    if not params:
        return {"outputs": {}}
    # Locate the wrapped plugin: the dark-build runs from the repo
    # root; the relative path above is relative to the repo root.
    candidates: list[str] = []
    if os.path.isabs(WRAPPED_PLUGIN):
        candidates.append(WRAPPED_PLUGIN)
    candidates.extend(
        [
            WRAPPED_PLUGIN,
            os.path.join(os.getcwd(), WRAPPED_PLUGIN),
            # The plugin may be invoked from a packaged bundle where
            # the fixture sits next to it.
            os.path.join(os.path.dirname(__file__), "..", "..", "cfd-reference-minimal", "python", "stdio_plugin.py"),
        ]
    )
    wrapped_path: str | None = next((c for c in candidates if os.path.isfile(c)), None)
    if wrapped_path is None:
        return {
            "plugin_id": PLUGIN_ID,
            "plugin_version": PLUGIN_VERSION,
            "determinism_class": "bit-reproducible",
            "outputs": {},
            "verification": {
                "checks": [
                    {
                        "name": "wrapped_plugin_resolvable",
                        "kind": "sanity",
                        "result": "fail",
                        "diagnostic": (
                            f"could not locate wrapped fixture; tried: {candidates}"
                        ),
                    }
                ],
                "overall_status": "fail",
            },
        }

    # Construct the JSON-RPC request, swap plugin_id to the wrapper's
    # so the provenance log shows the indirection.
    request: dict[str, Any] = {
        "jsonrpc": "2.0",
        "id": uuid.uuid4().hex,
        "method": "execute",
        "params": params,
    }
    try:
        proc = subprocess.run(
            ["python3", wrapped_path],
            input=json.dumps(request) + "\n",
            capture_output=True,
            text=True,
            timeout=30,
            check=False,
        )
    except (OSError, subprocess.TimeoutExpired) as exc:
        return {
            "plugin_id": PLUGIN_ID,
            "plugin_version": PLUGIN_VERSION,
            "determinism_class": "bit-reproducible",
            "outputs": {},
            "verification": {
                "checks": [
                    {
                        "name": "wrapped_plugin_spawn",
                        "kind": "sanity",
                        "result": "fail",
                        "diagnostic": f"spawn failed: {exc!r}",
                    }
                ],
                "overall_status": "fail",
            },
        }

    # The wrapped plugin prints one JSON line on stdout for our request.
    response: dict[str, Any] | None = None
    for raw in (proc.stdout or "").splitlines():
        raw = raw.strip()
        if not raw:
            continue
        try:
            response = json.loads(raw)
        except json.JSONDecodeError:
            continue
        if response.get("id") == request["id"]:
            break
        response = None

    if response is None or "result" not in response:
        return {
            "plugin_id": PLUGIN_ID,
            "plugin_version": PLUGIN_VERSION,
            "determinism_class": "bit-reproducible",
            "outputs": {},
            "verification": {
                "checks": [
                    {
                        "name": "wrapped_plugin_response",
                        "kind": "sanity",
                        "result": "fail",
                        "diagnostic": (
                            f"no JSON-RPC result; rc={proc.returncode}; "
                            f"stdout={proc.stdout!r}; stderr={proc.stderr!r}"
                        ),
                    }
                ],
                "overall_status": "fail",
            },
        }

    inner: dict[str, Any] = response["result"]
    # Stamp the wrapper identity + the indirection record so the
    # audit log makes the relaying visible.
    inner["plugin_id"] = PLUGIN_ID
    inner["plugin_version"] = PLUGIN_VERSION
    inner["shell_wrapper"] = {
        "wraps": "org.aether.reference.cfd.minimal",
        "wrapped_plugin_path": wrapped_path,
        "inner_plugin_id": inner.get("plugin_id"),
    }
    return inner


def method_init(params: dict[str, Any] | None) -> dict[str, Any]:
    if not isinstance(params, dict):
        return {"ok": False, "reason": "params must be an object"}
    for field in ("token_id", "workflow_id", "step_id", "scope", "signature_b64"):
        if field not in params:
            return {"ok": False, "reason": f"missing field: {field}"}
    if not isinstance(params.get("scope"), dict):
        return {"ok": False, "reason": "scope must be an object"}
    return {"ok": True, "plugin_id": PLUGIN_ID, "plugin_version": PLUGIN_VERSION}


METHODS = {
    "init": method_init,
    "discover": method_discover,
    "capabilities": method_capabilities,
    "validate": method_validate,
    "execute": method_execute,
}


def handle(line: str) -> dict[str, Any] | None:
    line = line.strip()
    if not line:
        return None
    try:
        req = json.loads(line)
    except json.JSONDecodeError as exc:
        return _err(None, -32700, f"parse error: {exc.msg}")
    if not isinstance(req, dict) or req.get("jsonrpc") != "2.0":
        return _err(req.get("id") if isinstance(req, dict) else None, -32600, "invalid request")
    method = req.get("method")
    req_id = req.get("id")
    params = req.get("params")
    if not isinstance(method, str):
        return _err(req_id, -32601, "method must be a string")
    fn = METHODS.get(method)
    if fn is None:
        return _err(req_id, -32601, f"method not found: {method}")
    try:
        result = fn(params)
    except Exception as exc:  # noqa: BLE001 — boundary; map to JSON-RPC internal error
        return _err(req_id, -32603, f"internal error: {exc!r}")
    return _ok(req_id, result)


def main() -> int:
    started = time.time()
    correlation = uuid.uuid4().hex[:8]
    print(f"[{correlation}] plugin ready (pid={__import__('os').getpid()})", file=sys.stderr)
    for raw in sys.stdin:
        response = handle(raw)
        if response is None:
            continue
        sys.stdout.write(json.dumps(response) + "\n")
        sys.stdout.flush()
    print(
        f"[{correlation}] plugin shutdown after {time.time() - started:.3f}s",
        file=sys.stderr,
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
