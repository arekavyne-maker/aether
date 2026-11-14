# SPDX-License-Identifier: Aether-VPSL-1.0
# SPDX-FileCopyrightText: 2026 The Aether Consortium
"""TUF bundle author — Phase 0 walking skeleton (c).

Builds a minimal TUF-style offline bundle (SDP §7.2, ADR-0021):

- `root.json`     — offline root key signs the root metadata itself
- `targets.json`  — list of artifacts with hashes and length
- `snapshot.json` — frozen state of targets at a sequence number
- `timestamp.json` — validity window + current snapshot sequence
- `artifacts/...` — actual artifact bytes (optional; verifier reads them if present)

The author signs each metadata file with Ed25519; the verifier confirms
the chain. This script and the verifier in
`crates/aether_cli/src/tuf/verify.rs` together prove the offline-update
security substrate (I-12, fail-visible).
"""

from __future__ import annotations

import argparse
import base64
import hashlib
import json
import os
import sys
import uuid
from datetime import datetime, timedelta, timezone
from pathlib import Path
from typing import Any

from nacl.signing import SigningKey


def _b64(data: bytes) -> str:
    return base64.b64encode(data).decode("ascii")


def _now() -> str:
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def _future(days: int) -> str:
    return (datetime.now(timezone.utc) + timedelta(days=days)).strftime("%Y-%m-%dT%H:%M:%SZ")


def _canonical_payload(meta: dict[str, Any]) -> bytes:
    """Canonical payload for signing: metadata with `signatures` removed."""
    clone = {k: v for k, v in meta.items() if k != "signatures"}
    return json.dumps(clone, sort_keys=True, separators=(",", ":")).encode("utf-8")


def _sign_metadata(sk: SigningKey, keyid: str, meta: dict[str, Any]) -> dict[str, Any]:
    payload = _canonical_payload(meta)
    sig = sk.sign(payload).signature
    out = dict(meta)
    out["signatures"] = [{"keyid": keyid, "sig": _b64(sig)}]
    return out


def _hash_artifact(path: Path) -> dict[str, Any]:
    h = hashlib.sha256()
    size = 0
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
            size += len(chunk)
    return {"sha256": h.hexdigest(), "length": size}


def make_root(sk_root: SigningKey, keyid: str) -> dict[str, Any]:
    pub_b64 = _b64(sk_root.verify_key.encode())
    meta: dict[str, Any] = {
        "_type": "root",
        "version": 1,
        "expires": _future(days=365),
        "keys": {
            keyid: {
                "keytype": "ed25519",
                "scheme": "ed25519",
                "keyval": {"public": pub_b64},
            }
        },
        "roles": {
            "root": [keyid],
            "snapshot": [keyid],
            "targets": [keyid],
            "timestamp": [keyid],
        },
    }
    return _sign_metadata(sk_root, keyid, meta)


def make_targets(
    sk: SigningKey,
    keyid: str,
    artifacts: dict[str, dict[str, Any]],
    seq: int,
) -> dict[str, Any]:
    meta: dict[str, Any] = {
        "_type": "targets",
        "version": 1,
        "expires": _future(days=30),
        "targets": artifacts,
    }
    return _sign_metadata(sk, keyid, meta)


def make_snapshot(
    sk: SigningKey,
    keyid: str,
    targets_meta: dict[str, Any],
    seq: int,
) -> dict[str, Any]:
    payload_bytes = _canonical_payload(targets_meta)
    h = hashlib.sha256(payload_bytes).hexdigest()
    meta: dict[str, Any] = {
        "_type": "snapshot",
        "version": 1,
        "expires": _future(days=7),
        "seq": seq,
        "meta": {
            "targets.json": {
                "version": targets_meta["version"],
                "sha256": h,
            }
        },
    }
    return _sign_metadata(sk, keyid, meta)


def make_timestamp(
    sk: SigningKey,
    keyid: str,
    snapshot_meta: dict[str, Any],
    seq: int,
) -> dict[str, Any]:
    payload_bytes = _canonical_payload(snapshot_meta)
    h = hashlib.sha256(payload_bytes).hexdigest()
    meta: dict[str, Any] = {
        "_type": "timestamp",
        "version": 1,
        "expires": _future(days=1),
        "seq": seq,
        "snapshot": {"sha256": h},
    }
    return _sign_metadata(sk, keyid, meta)


def write(path: Path, data: Any) -> None:
    path.write_text(json.dumps(data, indent=2, sort_keys=True), encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--out", type=Path, required=True, help="Output bundle directory.")
    parser.add_argument(
        "--artifact",
        action="append",
        default=[],
        help="Add an artifact: NAME=PATH. May be repeated.",
    )
    parser.add_argument("--seq", type=int, default=1, help="Sequence number.")
    args = parser.parse_args()

    args.out.mkdir(parents=True, exist_ok=True)
    (args.out / "artifacts").mkdir(exist_ok=True)

    sk_root = SigningKey.generate()
    keyid = uuid.uuid4().hex[:16]
    print(f"[author] root keyid: {keyid}", file=sys.stderr)

    artifacts: dict[str, dict[str, Any]] = {}
    for spec in args.artifact:
        if "=" not in spec:
            print(f"[author] bad --artifact (need NAME=PATH): {spec}", file=sys.stderr)
            return 2
        name, src = spec.split("=", 1)
        src_path = Path(src)
        if not src_path.is_file():
            print(f"[author] artifact not found: {src_path}", file=sys.stderr)
            return 2
        info = _hash_artifact(src_path)
        artifacts[name] = info
        target = args.out / "artifacts" / name
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(src_path.read_bytes())
        print(f"[author] artifact: {name} ({info['length']} bytes)", file=sys.stderr)

    root = make_root(sk_root, keyid)
    targets = make_targets(sk_root, keyid, artifacts, seq=args.seq)
    snapshot = make_snapshot(sk_root, keyid, targets, seq=args.seq)
    timestamp = make_timestamp(sk_root, keyid, snapshot, seq=args.seq)

    write(args.out / "root.json", root)
    write(args.out / "targets.json", targets)
    write(args.out / "snapshot.json", snapshot)
    write(args.out / "timestamp.json", timestamp)
    print(f"[author] bundle written to {args.out}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
