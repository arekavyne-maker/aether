# HPC backends (Slurm, Apptainer)

Aether v1.0 ships five `Backend` implementations. The conformance
fixture asserts all five implement the same contract surface; this
guide covers the HPC backends (Slurm and Apptainer).

## Slurm backend

The Slurm backend produces `sbatch` scripts and tracks a local
job-id table. The conformance fixture writes the script but does
not invoke `sbatch`; production sites replace the `submit` body with
`Command::new("sbatch")` and a `squeue`-driven status check.

```rust
let backend = make_backend(BackendKind::Slurm, workdir_root);
let handle = backend.submit(&spec, &Placement::default())?;
// Production sites: poll squeue to update handle.state.
```

### slurm.conf (production)

The `aether` user is a Slurm account; the Slurm cluster has the
node features declared in `node_capabilities()`. The kernel
submits each job with `--account=aether` and the resource
specification derived from the step's `requires` block.

## Apptainer backend

The Apptainer backend produces a `srun apptainer exec` script. The
container image is content-hash-pinned in the envman lockfile; the
backend verifies the image hash before submitting. The image is
expected at `/opt/aether/containers/<name>.sif`; the envman
StrictVerifier asserts that the image's actual hash matches the
manifest's `image_hash`.

```rust
let backend = make_backend(BackendKind::Apptainer, workdir_root);
let handle = backend.submit(&spec, &placement)?;
// Production sites: invoke `srun apptainer exec <sif> python3 /opt/aether/plugin.py`.
```

### Image pinning

The envman `containers` block pins the SIF image:

```yaml
containers:
  opensfoam.sif:
    image: "opensfoam.sif"
    engine: "apptainer"
    image_hash: "abc123..."
```

The `StrictVerifier` walks the artifact root (`containers/`), finds
`opensfoam.sif`, computes its SHA-256, and asserts the value matches
`image_hash`. A mismatch is a hard error in strict mode.

## Common pitfalls

1. **Stale Slurm job table**: production sites implement a periodic
   reconciliation against `squeue` to keep the local job-id table
   in sync. The conformance fixture uses a synthetic state
   (`JobState::Queued` or `Succeeded`); real sites refresh from the
   scheduler.
2. **Apptainer image cache**: the image is expected at a fixed path
   (e.g. `/opt/aether/containers/`). Sites that use a different
   cache (e.g. `~/apptainer/`) must symlink or copy the image
   into the expected path; the envman verifier asserts the path.
3. **License servers**: CFD and FEA solvers often need license
   servers. Sites that run on HPC must route license traffic to
   an internal server; the envman `toolchains` block documents
   which solver version expects which license server.
4. **Filesystem mounts**: HPC sites often have a shared filesystem
   for plugin bundles; the envman StrictVerifier must point at
   the shared mount, not at a per-job scratch directory.

## Cross-hardware conformance (MVR-04/05)

The dark-build CI runs the conformance fixture `mvr_04_05_12.rs`
on every commit. The fixture asserts the same package produces
equivalent results across backends (Local, Slurm, Apptainer, ...) and
across architectures (`x86_64`, `aarch64`). A green dark-build is
the structural guarantee that the cross-hardware claim is real.
