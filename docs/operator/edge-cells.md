# Edge cells (SSH-batch, agent-pool)

Aether v1.0 ships two backends for edge deployments (test labs,
factory cells, remote sites). Both implement the same `Backend`
trait; the conformance fixture `all_backends_implement_the_same_contract`
asserts the contract surface is identical.

## SSH-batch backend

The SSH-batch backend is **zero-daemon SSH fan-out** for small
labs and factory cells. There is no central scheduler; each
machine runs a stock `sshd` and the kernel connects to it directly.
This is the simplest production deployment for a 1–10 machine
cell.

```rust
let backend = make_backend(BackendKind::SshBatch, workdir_root);
let handle = backend.submit(&spec, &placement)?;
// Production sites: replace `submit` with
//   Command::new("ssh").arg("user@host").arg("python3 /opt/aether/plugin.py")
// and track the per-host inventory via `node_capabilities()`.
```

### SSH inventory

The backend's `node_capabilities()` returns one entry per host in
the inventory. Production sites read the inventory from a
configuration file (e.g. `/etc/aether/inventory.toml`) populated
by an out-of-band discovery step. The conformance fixture returns
a single synthetic host.

## agent-pool backend

The agent-pool backend is for **edge cells with many machines** (a
factory with 100+ test rigs, a deployment with many edge nodes).
A long-running `aether agent` daemon on each cell receives lease
requests over a mutually-authenticated local socket, executes
the plugin, and reports back.

```rust
let backend = make_backend(BackendKind::AgentPool, workdir_root);
let handle = backend.submit(&spec, &placement)?;
// Production sites: replace `submit` with the lease-request RPC
// to the agent pool; the daemon authenticates the request with
// the RunToken and dispatches to the right cell.
```

### Lease model

The agent-pool lease is the same as the RunToken contract, but
extended with:

- **Cell id**: which cell the lease is targeted to.
- **Capability envelope**: the agent only accepts leases whose
  resource requirements fit the cell's declared envelope.
- **Heartbeat**: the agent emits periodic heartbeats; the kernel
  marks the cell unhealthy if heartbeats stop.

### Network policy

The agent pool uses a local socket (or local TCP). It does NOT
expose any inbound port; the kernel connects outbound to the
agent. Sites that run a DMZ between the kernel and the agents must
allow only that one connection.

## Choosing between SSH-batch and agent-pool

| Use SSH-batch when | Use agent-pool when |
|---|---|
| 1–10 machines, no central scheduler | 10+ machines, need central inventory |
| Test labs with stable network | Factory cells with rotating machines |
| Quick prototyping | Production deployment with TLS + heartbeats |
| Operator manages each host directly | Operator manages the cell, not each host |

In both cases, the envman StrictVerifier is the gate: every plugin
bundle, container image, and policy artifact must be present in
the artifact root with the matching content hash before the
workflow can start.
