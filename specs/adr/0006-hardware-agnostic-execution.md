# ADR-0006: I-6 Hardware-Agnostic Execution

- **Status:** Accepted
- **Date:** 2026-08-30
- **Deciders:** Aether Architecture Review Board
- **Supersedes:** —
- **Superseded by:** —

## Context

The same engineering package must run on a workstation, a GPU workstation, an HPC cluster, and a factory/edge system (S-2, MVR-04/05/12). If workflows hard-code hardware references, portability is lost.

## Decision

Workflows and plugin manifests declare **computational requirements** (memory, cores, parallelism, accelerator class, architecture), never hardware identifiers.

The resource-descriptor schema (defined in the `compute.backend` contract; see `contracts/compute.backend/`) **forbids** hardware-specific fields in workflow documents. Specifically: no GPU model names, no CPU vendor strings, no machine hostnames, no queue names, no scheduler-specific paths.

Hardware-specific data appears **only**:

- Inside backend adapter implementations (`crates/aether_exec/src/backends/*`)
- In node inventory profiles discovered at runtime
- In provenance records (for audit, not for control flow)

A workflow that requires GPU acceleration declares `accelerator: optional` and `accelerator_class: any` (or `nvidia_cuda`, `amd_rocm`, `opencl`, `generic_tensor`); the registry matches the workflow to a plugin that supports the required class, and the scheduler places it on a node that exposes that class.

## Consequences

- Positive: same engineering package runs unmodified on heterogeneous hardware (S-2)
- Positive: vendor lock-in at the hardware level is structurally impossible
- Negative: a workflow that *needs* a specific GPU model for correctness (e.g., a solver only certified on a particular arch) must encode this as a *requirement*, not a hardware name
- Negative: the resource-descriptor schema is the integration point; it must evolve carefully

## Enforcement

- Resource-descriptor schema validation rejects hardware-specific fields in workflow documents
- MVR-04/05/12 executed at each pilot site before GA-2
- Forbidden-token scan extended for GPU model names, CPU vendor strings, queue names, hostnames

## References

- SDP §2 I-6
- SDP §6.1 (declare requirements, not hardware)
- SDP §6.5 (accelerator abstraction)
- SDP §17.2 (engineering model invariance)
- SDP §20 MVR-04/05/12
- ADR-0003 (zero hard-coupling)
