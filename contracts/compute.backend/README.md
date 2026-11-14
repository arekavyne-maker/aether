# `aether.compute.backend` Contract v0

Compute backend contract — abstracts HPC and edge schedulers behind a single SPI (SDP §6.2). The kernel uses this contract to place jobs on `local`, `slurm`, `pbs`, `apptainer`, `ssh-batch`, and `agent-pool` backends.

## Versioning

- **Contract id:** `aether.compute.backend`
- **Schema version:** `0.1.0` (Phase 0 v0; v1.0 freeze at Phase 2 G2)

## Methods

| Method | Description |
|---|---|
| `node_profile()` | Static + live inventory (CPU, RAM, accel, disk, MPI). |
| `submit(job, placement)` | Returns `SchedulerJobRef`. |
| `status(ref)` | `queued | running | succeeded | failed | preempted`. |
| `stage_data(ref, staging_plan)` | Pull/push CAS chunks to node-local scratch. |
| `reap(ref)` | Exit code, peak resources, timing, node identity. |

## Layout

```
compute.backend/
├── manifest.json
├── schema/
│   ├── job.schema.json
│   ├── placement.schema.json
│   ├── node-profile.schema.json
│   └── resource-requirement.schema.json
├── examples/
│   └── cfd-fine-mesh.job.yaml
└── README.md
```

## I-6 (Hardware-Agnostic) is operationalized here

Workflows and plugin manifests declare **requirements** (memory_gb, cores, parallelism, accelerator_class, architecture), never hardware identifiers. The resource-requirement schema is the integration point.

## See also

- SDP §6 (Compute Fabric)
- SDP §6.2 (Scheduler SPI)
- SDP §17.2 (engineering model invariance)
- ADR-0006 (hardware-agnostic execution)
