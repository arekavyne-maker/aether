# AWL v1.3 — Aether Workflow Language

AWL is the platform's declarative workflow surface (SDP §4.2,
§17, §18). An AWL document is a YAML file declaring a DAG of steps,
each with typed inputs, outputs, and resource requirements. G2 ships
AWL v1; G4 adds `compose:`, `si_hooks`, `ladder:`; G5 closes the
optimization-twin loop; G6 is the G6 closed-loop manufacturing
workflow.

## Document structure

```yaml
awl: "1.3"                # schema version (v1.0, v1.2, v1.3 supported)
study: "..."              # human-readable name
description: |
  Free-form prose; the dossier entry for this workflow.
inputs:                   # workflow-level variables
  geometry: { ... }
steps:                    # DAG nodes, in declared order
  - id: <step_id>
    type: run | sweep | gate | noop
    plugin: <reverse-DNS plugin id>
    plugin_version: <semver range>
    contract: <contract id>
    inputs: { ... }       # contract-shaped payload
    depends_on: [<step_id>, ...]
    retries: { max_attempts: N, backoff_seconds: S }
    si_hooks: { ... }      # G4+
outputs: [ ... ]          # declared outputs (dossier entries)
evidence:
  attach: true
  verification_relevant: true
reproducibility:
  mode: strict | equivalence
  env_manifest: env/<lockfile>.lock
```

## Step types

- **run**: a single execution of a plugin (the most common).
- **sweep**: a parameter sweep over a plugin (G4+).
- **gate**: a conditional that selects the next step.
- **noop**: a control-flow step (no execution).

## Resource requirements

Per-step `requires`:

```yaml
requires:
  memory_gb: 64
  cores: 32
  parallelism: distributed      # single | multi_thread | multi_process | distributed
  accelerator: optional          # none | optional | required
  accelerator_class: any         # any | nvidia_cuda | amd_rocm | opencl
  architecture: [x86_64, aarch64]
  max_runtime_h: 48
```

The executor routes to the right backend (Local / Slurm / Apptainer /
SSH-batch / agent-pool) based on these declarations.

## SI hooks (G4+)

```yaml
si_hooks:
  fidelity_governor: true      # the FidelityGovernor may adjust the sampling plan
  surrogate_trust: true        # the SurrogateTrustAssessor may substitute a surrogate
  data_value: true            # the DataValueEstimator may suggest additional data/compute
  autonomy_max: A2              # A0..A3; A4 is structurally forbidden
```

## Composition (G4+)

```yaml
compose:
  name: "aeroelastic"
  participants:
    - { name: flow, plugin: com.acme.cfd.opensfoam, plugin_version: "2.4.1", physics_class: fluid }
    - { name: structure, plugin: org.oss.feather.calculix, plugin_version: "2.21.0", physics_class: structural }
  exchanges:
    - { from: flow, to: structure, name: pressure, units: Pa, mapping: conservative, tolerance: 1.0e-4 }
  scheme:
    type: partitioned_implicit
    accelerator: iqn_ils
    max_iterations: 30
    tolerance: 1.0e-4
    divergence_abort: true
```

## Fidelity ladder (G4+)

```yaml
ladder:
  tiers:
    - { tier: surrogate, plugin: org.aether.surrogate, si_hooks: { surrogate_trust: true } }
    - { tier: medium, plugin: com.acme.cfd.opensfoam }
    - { tier: high, plugin: com.acme.cfd.opensfoam }
```

## Reproducibility

`mode: strict` refuses to start if any pin in `env_manifest` is
unresolved (the MVR-11 substrate). `mode: equivalence` records the
delta between expected and observed outputs and continues.

## Executable examples in the repo

- `workflows/paper-run.awl.yaml` — G0/G1 paper-run chain
- `workflows/mvr-01-02.awl.yaml` — G2 plugin-replacement
- `workflows/agard-445-flutter.awl.yaml` — G4 multiphysics
- `workflows/rib-optimization-closed-loop.awl.yaml` — G5 closed loop
- `workflows/wing-rib-closed-loop-ga2.awl.yaml` — G6 closed loop
