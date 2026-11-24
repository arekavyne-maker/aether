# Aether — wing-box vertical slice

This example is the **Phase 2 demonstrator**: a conventional metallic
wing-box subassembly that exercises the current substrate end to end without
claiming certification.

It is deliberately small:

- An ASM object set with stable ids, released revisions, and content hashes
  (`configuration.asm.yaml`).
- A configuration snapshot that pins those exact revisions.
- A release gate that requires current verification, credible validation,
  and inspection evidence (`release-gate.yaml`).
- A manufacturing genealogy that traces each rib build back to its material
  lot, process plan revision, inspection results, and dispositions
  (`genealogy.yaml`, `genealogy-incomplete.yaml`).
- A model card with a declared validity envelope and promotion evidence
  (`model-card.yaml`, `model-card-incomplete.yaml`).
- A fidelity plan that selects a structural analysis level against a
  declared error target and compute budget (`fidelity.yaml`,
  `fidelity-tight.yaml`).
- A structural baseline executed by the existing beam-FEM reference plugin
  (`aether demo structure`).

## Run the acceptance suites

```bash
# Phase 1/2 substrate + wing-box vertical slice
bash scripts/acceptance/phase2-wingbox.sh

# Phase 3 manufacturing genealogy and as-built traceability
bash scripts/acceptance/phase3-manufacturing.sh

# Phase 4 deterministic multi-fidelity selection
bash scripts/acceptance/phase4-fidelity.sh

# Phase 6 model-card promotion governance
bash scripts/acceptance/phase6-model-governance.sh

# Phase 7 partial hardening: chain, export/replay, tamper detection, DR drill
bash scripts/acceptance/phase7-hardening.sh
```

Each script asserts:

1. the structural baseline passes its own closed-form check;
2. the snapshot reconstructs exactly and rejects a changed object set;
3. the release gate passes only with credible current evidence;
4. an incomplete evidence set fails closed with exit code 30;
5. the validation report separates covered requirements from gaps;
6. manufacturing genealogy requires material lots, qualified operators,
   inspections, and authorized dispositions for nonconformances;
7. revision impact analysis names every affected build and whether it needs
   revalidation;
8. fidelity selection explains the chosen level, the escalation path, and
   any binding constraint;
9. model promotion is role-gated and requires current independent
   validation and benchmark evidence;
10. graph chain, export/replay, tamper detection, and the DR-drill fixture
    remain operational.

## What this does **not** prove

Real B-rep geometry, composite failure, full-airframe validation,
certification, production manufacturing, real machine control, agent
planning, plugin sandboxing, federation, or production hardening. Those
remain later phases with their own evidence.