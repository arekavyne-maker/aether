# Plugin author: TUF-signed bundles

In production, every plugin bundle is TUF-signed. The site
verifies the signature chain before unpacking; the envman
StrictVerifier then asserts the bundle's content hash matches the
declared pin in the envman lockfile.

## The TUF chain

The TUF metadata is rooted in a site-level `root.json` that
declares the trusted keys. The chain is:

```
root.json (offline, pinned at site install)
└── targets.json (lists every artifact with hashes + lengths)
    └── snapshot.json (freezes the targets at a sequence number)
        └── timestamp.json (asserts the snapshot is fresh)
```

The kernel's `aether tuf verify` command checks the chain. A
validation failure is a hard error; the site is in a bad state
and must be rebuilt from a known-good bundle.

## Authoring a signed bundle

The plugin author's workflow:

1. **Build the bundle**: a tarball with the manifest, the plugin
   binary, and any required libraries.
2. **Compute the hash**: `sha256sum <bundle>.tar.gz`.
3. **Update `targets.json`**: add an entry with the hash and length.
4. **Sign `targets.json`**: the site operator (not the plugin
   author) signs with the site key. The plugin author never has
   the site key.
5. **Update `snapshot.json`**: freeze the targets at a new
   sequence number.
6. **Sign `snapshot.json`**: same site key.
7. **Update `timestamp.json`**: assert the snapshot is fresh.
8. **Sign `timestamp.json`**: same site key.

The site operator (a human or a CI pipeline) holds the site key;
the plugin author produces only the unsigned bundle. The two are
deliberately separate: a compromised plugin author cannot ship a
bundle to a site that hasn't approved the bundle through the
operator's TUF chain.

## Local development (no TUF)

In dev, sites skip the TUF chain and use the local StrictVerifier
directly:

```sh
aether envman install /path/to/<bundle>.tar.gz /path/to/<env_manifest>.yaml \
    --artifact-root /path/to/artifacts
```

The CLI copies the bundle into the artifact root and reports
whether the bundle's hash matches the pin. In dev, a hash mismatch
is a warning; in production it's a contract violation.

## The envman + TUF relationship

TUF provides the **signature chain**; envman provides the **hash
pinning**. The two are complementary: TUF says "this bundle was
signed by the site key"; envman says "this bundle's content
matches the declared hash". Both must hold for the bundle to be
trusted.

In strict mode:

1. The site operator has signed `targets.json` (TUF).
2. The bundle's hash in `targets.json` matches the bundle's
   computed hash (TUF integrity).
3. The envman pin's hash matches the bundle's computed hash
   (envman integrity).
4. The plugin's `discover` response matches the envman pin's
   version (envman version pin).

All four must hold. A failure at any step is a contract
violation; the workflow does not start.

## Why this matters

Without TUF, a compromised plugin author could ship a malicious
bundle and the site would install it. Without envman, a
compromised site operator could swap a bundle for a different one.
With both, neither attack succeeds: the plugin author's bundle is
hashed and signed by the operator; the operator's pin matches the
author's hash. **Both halves of the trust chain are required** —
TUF alone is insufficient (the operator can still swap bundles),
envman alone is insufficient (the author can still ship malware
that the operator pins).
