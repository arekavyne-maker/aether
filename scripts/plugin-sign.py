#!/usr/bin/env python3
# SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
# SPDX-FileCopyrightText: 2026 Crocell Company
"""plugin-sign — sign a plugin bundle with an Ed25519 key (ADR-0021).

This is the authoring half of the offline trust model. The kernel-side
verifier lives in `crates/aether_registry`; this script produces the
signature and the detached manifest a site operator can verify without
network access.

Usage:
    plugin-sign sign --key key.pem --bundle plugin.tar.gz --out plugin.sig
    plugin-sign verify --pubkey key.pub --bundle plugin.tar.gz --sig plugin.sig

Only the detached-signature form is supported. A bundle without a valid
signature is not installable.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path

try:
    from nacl.signing import SigningKey, VerifyKey
except ImportError as exc:  # pragma: no cover - environment dependent
    sys.stderr.write(
        "error: PyNaCl is required (pip install pynacl==1.6.2)\n"
    )
    raise SystemExit(2) from exc


def _hash(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def sign(key_path: Path, bundle: Path, out: Path) -> dict:
    signing_key = SigningKey(key_path.read_bytes())
    verify_key = signing_key.verify_key
    payload = bundle.read_bytes()
    signature = signing_key.sign(payload).signature
    out.write_bytes(signature)
    manifest = {
        "manifest_version": "1.0",
        "bundle": bundle.name,
        "bundle_sha256": hashlib.sha256(payload).hexdigest(),
        "public_key_sha256": hashlib.sha256(verify_key.encode()).hexdigest(),
        "signature": signature.hex(),
        "algorithm": "ed25519",
    }
    return manifest


def verify(pubkey_path: Path, bundle: Path, sig_path: Path) -> dict:
    verify_key = VerifyKey(pubkey_path.read_bytes())
    payload = bundle.read_bytes()
    raw_signature = sig_path.read_bytes()
    if raw_signature.startswith(b"hex:"):
        raw_signature = raw_signature[4:].strip()
        try:
            signature = bytes.fromhex(raw_signature.decode("ascii"))
        except (ValueError, UnicodeDecodeError) as exc:
            raise ValueError("signature is not valid hex") from exc
    else:
        signature = raw_signature
    verify_key.verify(payload, signature)
    return {
        "verified": True,
        "bundle": bundle.name,
        "bundle_sha256": _hash(bundle),
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)

    s = sub.add_parser("sign")
    s.add_argument("--key", type=Path, required=True)
    s.add_argument("--bundle", type=Path, required=True)
    s.add_argument("--out", type=Path, required=True)

    v = sub.add_parser("verify")
    v.add_argument("--pubkey", type=Path, required=True)
    v.add_argument("--bundle", type=Path, required=True)
    v.add_argument("--sig", type=Path, required=True)

    args = parser.parse_args(argv)
    if args.command == "sign":
        print(json.dumps(sign(args.key, args.bundle, args.out), sort_keys=True))
        return 0
    print(json.dumps(verify(args.pubkey, args.bundle, args.sig), sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())