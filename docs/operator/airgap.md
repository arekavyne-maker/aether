# Air-gap operation

Aether's default deployment mode is air-gapped. The site has no
mandatory internet dependency; every network-aware code path
fails closed when `AETHER_OFFLINE=1` is set.

## The offline contract

A contract violation occurs when:

- The HTTP client wrapper attempts a network request in offline mode.
- The DNS resolver is invoked in offline mode.
- A TUF metadata refresh is attempted in offline mode.
- A plugin install attempts to fetch from a remote registry.

The kernel's network-aware wrappers are the choke point: every
network call site is gated by an `is_offline()` check that returns
`true` when `AETHER_OFFLINE=1` is set. A failed check is a
`ResourceUnavailable` error (SDP §4.5), surfaced as exit code 40.

## Verifying offline mode

```sh
export AETHER_OFFLINE=1
aether workflow run workflows/paper-run.awl.yaml --project /opt/aether/site
```

The CLI must complete without a network attempt. To assert this
structurally (not just by absence of failure), run the conformance
test:

```sh
python3 -m pytest python/aether_testing/tests/test_offline_soak.py
```

This test runs the contract validation CLI in offline mode and
asserts that the output contains no `http://`, `https://`, "DNS
lookup", or `socket.connect` tokens. A regression that introduces
a network call fails this test.

## 180-day offline soak substrate

The conformance test is the **substrate** for a 180-day calendar
timer; the real soak runs on a separate site (different physical
host, different operator) and asserts the same invariants on a
calendar basis. The substrate is the structural guarantee that the
real soak is meaningful: a regression in the network-aware
wrappers shows up immediately in CI, not after months.

To run the 180-day soak in production:

1. Deploy Aether on a sealed site.
2. Set `AETHER_OFFLINE=1` in `/etc/aether/env`.
3. Run the conformance soak once per day as a cron job.
4. The cron job asserts the same invariant on every run; a
   regression shows up at most 24 hours after the bad commit.

## Failure semantics in offline mode

| Failure | Behavior |
|---|---|
| HTTP client wrapper | `ResourceUnavailable` (exit 40) |
| DNS resolution | `ResourceUnavailable` (exit 40) |
| TUF refresh | `ResourceUnavailable` (exit 40) |
| Plugin install with remote URI | `ResourceUnavailable` (exit 40) |
| Local file operation (read, write, exec) | works as in online mode |
| Local CAS (SHA-256 over local files) | works as in online mode |

## What "offline" does NOT mean

- The site can have local services (NTP via local server, HTTP
  between local services, etc.). `AETHER_OFFLINE=1` only blocks
  *outbound* traffic to non-loopback addresses; local sockets and
  `localhost:` URLs are unaffected.
- The site can run network services (e.g. an internal MTConnect
  broker) — those are plugin-implemented and are local by
  definition.
- The site can have incoming traffic (e.g. a registry the site
  mirrors from) — incoming is the operator's concern, not the
  platform's.
