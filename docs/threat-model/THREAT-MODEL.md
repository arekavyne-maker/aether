# Aether 3.0 — Threat Model

**Status:** v2 working model, 2026-11-24.
**Supersedes:** the abridged Phase 0 model in `specs/threat-model/README.md`.
**Scope:** platform substrate, plugins, workflows, artifacts, agents,
federated nodes, and industrial interfaces.

---

## 1. Trust boundaries

| Boundary | What crosses it | Trust posture |
|---|---|---|
| User/CLI → graph | Commands, identities, mutations | Authenticated principal; authorize every operation |
| Core → plugin | Contract requests, scoped artifacts, run tokens | Untrusted process; signed manifest, least privilege, sandbox |
| Plugin → artifact store | Reads/writes of input/output data | Scoped handles; content hashes; no raw DB credentials |
| Workflow → compute | Job specs, inputs, outputs | Signed job envelope; idempotency key; quota |
| Agent → tools | Proposals, retrieval, tool calls | No direct mutation; typed tools; policy gate; human approval |
| Federated node → local node | Bundles, manifests, evidence | Mutual authentication; signed manifests; data classification |
| Industrial gateway → machine | Programs, parameters, telemetry | Read-only default; authorized write path; separate safety case |
| Model registry → runtime | Weights, model cards, approval state | Signed/hashed model; approved intended use; rollback |
| Research sandbox → production | Candidate methods/results | Quarantine; benchmark; independent validation; promotion ADR |

## 2. Threat classes and mitigations

### T-01 Supply-chain compromise

**Threat:** malicious crate, wheel, container, plugin bundle, or model weight.

**Mitigation:** vendored/locked dependencies; SBOM; signed plugin manifests;
TUF-adapted offline verification; hash-pinned environments; model weight
digests; egress-free dark-build; provenance of build inputs.

**Residual risk:** a compromised upstream release that is correctly signed but
malicious. Mitigate with independent builds, reproducible builds, and review.

### T-02 Plugin escape or overreach

**Threat:** plugin reads unrelated project data, writes outside its scope,
opens a network connection, or consumes unbounded resources.

**Mitigation:** capability declarations; scoped filesystem mounts; network
allowlist/deny-all; CPU/memory/time quotas; signed identity; audit log;
no raw credentials. **Current gap:** sandbox is not implemented.

**Residual risk:** a plugin with a legitimate capability can still produce
wrong engineering data. Sandboxing is not correctness.

### T-03 Forged or replayed evidence

**Threat:** attacker edits a result, reuses an old evidence package, or forges
an approval.

**Mitigation:** immutable revisions; content hashes; Merkle event chain;
signed provenance records; snapshot-bound runs; gate evaluation re-checks
hashes and revisions; approval records bind principal, role, timestamp, and
evidence set.

**Residual risk:** compromised signing key. Mitigate with rotation, revocation,
threshold roles, and offline root.

### T-04 Prompt injection through retrieved content

**Threat:** a PDF, CAD metadata field, issue, web page, or test log contains
instructions that cause an agent to bypass policy.

**Mitigation:** retrieved content is data, never instructions; typed tools;
allowlisted actions; no shell on production; agent proposals require
deterministic validation; prompt-injection regression tests; policy decisions
outside the model.

**Residual risk:** a persuasive document can still mislead a human reviewer.
Surface provenance and uncertainty; require independent review.

### T-05 Surrogate or AI overreach

**Threat:** a surrogate or learned model is used outside its validity domain
and produces a plausible but wrong result.

**Mitigation:** model cards; validity envelopes; OOD detection; uncertainty
components; fallback solver; benchmark evidence; promotion workflow; no AI
release authority.

**Residual risk:** undetected distribution shift. Monitor drift and require
revalidation triggers.

### T-06 Malicious or malformed engineering files

**Threat:** crafted STEP, CGNS, HDF5, mesh, or image file exploits a parser.

**Mitigation:** parser fuzzing; sandboxed import; size/format limits; schema
validation; no implicit execution; quarantine on parse failure.

**Residual risk:** zero-day parser vulnerability. Isolate import workers and
patch promptly.

### T-07 Insider exfiltration

**Threat:** authorized user exports controlled data or tampers with audit
history.

**Mitigation:** data classification; export policy; audit export; two-person
rule for controlled release; append-only chain; least privilege.

**Residual risk:** authorized but malicious action. Detect via anomaly
monitoring and review.

### T-08 Reproducibility attack

**Threat:** inputs or environment silently change, so a recorded result cannot
be reproduced.

**Mitigation:** configuration snapshots; environment manifests; content
hashes; canonical JSON; offline replay; dependency locks.

**Residual risk:** non-deterministic solver or hardware. Record seeds,
tolerances, and nondeterminism class.

### T-09 Industrial control compromise

**Threat:** a compromised workflow or agent changes machine parameters.

**Mitigation:** industrial gateway is a separate trust boundary; read-only by
default; signed, authorized programs; no autonomous parameter changes; safety
case and rollback required for any write path.

**Residual risk:** physical process hazards. This is a separate safety-critical
programme, not a software feature.

### T-10 Federation and multi-party governance

**Threat:** a federated node receives data it should not see, or injects
untrusted evidence.

**Mitigation:** mutual authentication; per-object access control; data
classification; signed bundles; local policy enforcement; export review.

**Residual risk:** conflicting governance regimes. Require contractual and
legal review.

## 3. Current security gaps

| Gap | Severity | Phase |
|---|---|---|
| `PolicyEngine` always allows | critical | 1 |
| No plugin sandbox/resource limits | high | 7 |
| No key rotation/revocation operations | high | 7 |
| No tested restore/RPO/RTO drill | high | 7 |
| No model registry or model-card approval | high (AI) | 6 |
| No agent runtime permission matrix | high (AI) | 6 |
| No observability metrics/traces | medium | 7 |
| No worker attestation | medium | 7 |

## 4. Security review gates

- Phase 1: policy deny tests, audit decision recording, evidence fail-closed.
- Phase 3: manufacturing/quality access boundaries.
- Phase 6: agent permission matrix, prompt-injection tests, model promotion.
- Phase 7: sandbox, restore drill, key rotation, federation review.

## 5. Explicit non-claims

Aether does not claim compliance with export-control, defence, aviation, or
cybersecurity regimes. Such claims require a separate legal and technical
assessment.
