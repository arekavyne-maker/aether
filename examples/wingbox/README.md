# Wing-box vertical slice

This is the Phase 2 demonstrator: a conventional metallic wing-box
subassembly that exercises the substrate end to end without claiming
certification.

It is deliberately small:

- ASM object set with stable ids, released revisions, and content hashes
  (`configuration.asm.yaml`).
- A configuration snapshot that pins those exact revisions.
- A release gate that requires current verification, credible validation,
  and inspection evidence (`release-gate.yaml`).
- A structural baseline executed by the existing beam-FEM reference
  plugin (`aether demo structure`).

Run the acceptance script:

```bash
bash scripts/acceptance/phase2-wingbox.sh
```

The script asserts:

1. the structural baseline passes its own closed-form check;
2. the snapshot reconstructs exactly and rejects a changed object set;
3. the release gate passes only with credible current evidence;
4. an incomplete evidence set fails closed with exit code 30;
5. the validation report separates covered requirements from gaps.

What this does **not** prove: real B-rep geometry, composite failure,
full-airframe validation, certification, or production manufacturing.
Those remain later phases with their own evidence.
