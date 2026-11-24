# Aether — Production runbook (Phase P7)

**Status:** operational runbook for a production-grade deployment.
**Rule:** no compliance or certification claim is made. All procedures are
executable from this repository.

---

## 1. Objectives (ADR-0040)

SLOs are declared per deployment profile after measurement, not invented.
The benchmark registry (`infra/benchmarks_registry.py`) records runtime and
accuracy separately. Run:

```bash
python3 infra/benchmarks_registry.py --run
```

Results land under `docs/verification/baseline/<date>/benchmarks.json`.

| Profile | Measured SLO source |
|---|---|
| workstation / offline | benchmark registry performance group |
| cluster | requires real `sbatch`; otherwise unavailable |
| federated | loopback harness; real SLO requires a second site |

An absent optional dependency is reported as `unavailable` by `aether_health`
equivalent probes, never as healthy.

## 2. Recovery objectives

| Objective | Value | Evidence |
|---|---|---|
| RPO | one completed export bundle | `aether export` writes a self-verifying bundle |
| RTO | time to replay one bundle | measured by `restore_duration` benchmark |
| Backup frequency | after every release gate | `aether release prepare` |
| Immutable backup | signed release manifest | `aether_crypto::ReleaseManifest` |
| Restore drill | project-level replay | `test_dr_drill.py` |

These are engineering objectives, not a business continuity guarantee.

## 3. Restore drill

```bash
# 1. Export a self-verifying bundle from a healthy project.
aether export --project <project> --output /tmp/bundle --json

# 2. Simulate loss by working in a fresh directory.
aether replay /tmp/bundle --json

# 3. Verify the graph chain after restore.
aether graph chain --project <project> --json
```

The drill passes when `verified: true`, `manifest_sha256_ok: true` and the
chain reports `valid: true`.

## 4. Key rotation

1. Record the current fingerprint (`aether_identity::key_fingerprint`).
2. Revoke known-compromised token ids (`revoke`).
3. Rotate the site secret (`rotate`).
4. Re-issue tokens for active principals.
5. Verify old tokens now fail with `BadSignature`.
6. Record the ceremony in the evidence ledger.

## 5. Signed releases

```bash
# Rust API: aether_crypto::ReleaseManifest::sign / verify
```

A release with a bad signature or malformed artifact hash is refused.
No release is valid because an LLM or a successful run says so.

## 6. Data classification

- Every artifact carries a classification label.
- Unclassified objects are denied read and export (fail closed).
- Export-controlled data may not be exported or federated under the default
  handling policy.
- No export-control, defence, aviation or cybersecurity compliance is claimed.

## 7. Degraded modes

| Condition | Behaviour |
|---|---|
| `sbatch` absent | Slurm backend returns an error; local mode unaffected |
| OCP absent | geometry reports labelled fixture; no real B-rep claim |
| gmsh absent | meshing reports unavailable |
| CalculiX absent | BENCH-08 reports SKIP |
| network unavailable | `--offline` forbids remote operations; local work continues |
| AI runtime outage | deterministic workflows continue; agents are advisory only |

## 8. Failure domains

Graph state, artifact storage, workflow scheduling, plugin processes, model
inference and identity are separate. A solver crash cannot corrupt the graph;
an agent outage cannot block deterministic workflows.

## 9. Incident response

1. Stop the affected workflow or release.
2. Verify the graph chain.
3. If invalid, restore from the last signed bundle.
4. Record root cause and corrective action in the evidence ledger.
5. Re-run the gate before resuming controlled work.

## 10. What this runbook does not cover

Certification, airworthiness, real machine control, a real second
federation site, and an independent penetration test.
