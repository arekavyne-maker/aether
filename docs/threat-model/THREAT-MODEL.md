# Aether 3.0 — Threat Model

**Status:** living document, 2026-11-24.
**Scope:** the Aether platform as implemented in this repository, including
the new Phase 1–7 primitives. It is not a certification security assessment.

---

## 1. Trust boundaries

| Boundary | Inside | Outside | Primary control |
|---|---|---|---|
| User ↔ CLI | Engineer workstation | Filesystem, graph DB | RBAC, fail-closed policy |
| Kernel ↔ plugin | Kernel process | Plugin process | Signed run token, sandbox policy |
| Plugin ↔ artifact store | CAS paths | Rest of filesystem | Scoped read/write paths |
| Project ↔ export bundle | Project state | Anyone with the bundle | Manifest hash, workflow hashes |
| Site ↔ federation peer | Signed bundle | Remote site | Ed25519 signature, TUF-adapted trust |
| Agent ↔ platform | Advisor process | Engineering authority | Permission matrix, human approval |

## 2. Assets

- Engineering graph and its hash chain.
- Configuration snapshots and release packages.
- Evidence items and credibility records.
- Material allowables, process parameters, and machine programs.
- Signing keys and the trust root.
- Plugin bundles and their signatures.

## 3. Threats and mitigations

| ID | Threat | Mitigation | Residual risk |
|---|---|---|---|
| T-01 | Unauthorized graph mutation | Fail-closed RBAC; unknown role/action denied | Local OS account compromise |
| T-02 | Silent unit or frame error | `aether units check` before solver submission | Adapters must call it |
| T-03 | Tampered configuration snapshot | Content hashes; `config verify` fails closed | Hash collision (SHA-256, negligible) |
| T-04 | Release without evidence | `release prepare` requires every gate; no execution-only path | Human approver collusion |
| T-05 | Unauthorized waiver | Waiver must name gate + snapshot and an authorized role | Approver identity spoofing outside the platform |
| T-06 | Plugin escapes its sandbox | `SandboxPolicy` + `spawn_sandboxed`; env cleared and filtered | Kernel-level isolation is an adapter, not yet enforced |
| T-07 | Plugin calls undeclared methods | Run-token method allowlist | Plugin may ignore its token |
| T-08 | Malicious agent self-approval | `aether_agents` prohibits approve/policy/machine tools | None within the platform |
| T-09 | Agent exfiltrates data | Tool permission matrix; no network tool for agents | Human-approved workflow with network plugin |
| T-10 | Tampered export bundle | Manifest hash + workflow hashes verified on replay | Bundle author holds the signing key |
| T-11 | Stale evidence reused | Evidence status must be `current` and bound to subject revision | Evidence metadata can be forged only by an authorized writer |
| T-12 | Model promoted without evidence | Role-gated promotion with required classes and independence | Registry operator compromise |
| T-13 | Candidate accepted without review | Ordered feasibility pipeline with reason codes | Reviewer rubber-stamping |
| T-14 | Supply-chain substitution | SBOM, pinned artifacts, Ed25519 signatures | Key theft |
| T-15 | Fabricated maturity claim | `claim_scan` + status page as source of truth | Docs can drift; review required |

## 4. Attack surfaces

1. **CLI** — arbitrary user input, file paths, YAML/JSON parsing.
2. **Graph storage** — SQLite file, event log, AQL subset.
3. **Plugin stdio** — JSON-RPC over pipes; a plugin controls its stdout.
4. **Artifact store** — content-addressed paths; a plugin can write any allowed path.
5. **Export/replay** — bundles are portable and can be edited.
6. **Agent proposals** — untrusted model output, treated as data.
7. **Fidelity/model inputs** — declared errors are claims, not measurements.

## 5. Security properties that are enforced

- Unknown role or action is denied (`aether_policy`).
- Unit and frame mismatches fail before solver submission (`aether units check`).
- Snapshot verification fails closed on content drift (`aether config verify`).
- Release gates fail closed on missing or stale evidence (`aether evidence gate`).
- Agents cannot approve, edit policy, or control machines (`aether agent check`).
- Sandbox requests cannot escape read/write roots, env allowlist, or network posture.
- Export replay verifies the manifest and workflow hashes before executing.

## 6. Security properties that are *not* yet enforced

- Kernel-level isolation (seccomp, namespaces, containers, cgroups).
- Plugin attestation beyond a signed run token.
- Key rotation and revocation ceremonies.
- Centralized audit aggregation across sites.
- Rate limiting and denial-of-service protection for agent proposals.
- Full supply-chain provenance for third-party plugins.
- Independent penetration test.

## 7. Residual risk statement

Aether reduces the risk of silent, unevidenced engineering change. It does
not eliminate the need for authorized human judgement, independent
verification, or a qualified safety process. No statement in this repository
constitutes a certification or airworthiness approval.