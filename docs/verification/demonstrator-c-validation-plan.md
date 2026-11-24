# Demonstrator C — Validation plan (synthetic fixtures)

**Status:** validation plan; physical test data not yet available.
**Demonstrator:** morphing trailing-edge subassembly.

## What the synthetic fixtures prove

The `morphing-te` plugin executes four real coupled models against synthetic
inputs: multi-state geometry, compliant flexure stiffness, lifting-line
aero-structural coupling, and Palmgren-Miner transition-cycle fatigue. Each
run is content-addressed and replayable. This proves the engineering chain,
not the physical fidelity of a specific mechanism.

## Claimed maturity

- Geometry: parametric camber states, monotone transition check.
- Structure: linear elastic Euler-Bernoulli flexure; no shear deformation.
- Aero: lifting-line with declared camber effectiveness; no CFD.
- Fatigue: linear damage accumulation; no crack growth.
- Manufacturing: inspection-port note; no process simulation here.

## Validation needed before production use

1. Coupon tests of the flexure material at representative cycle counts.
2. Wind-tunnel measurement of camber effectiveness across the state range.
3. Actuator authority and failure-mode testing.
4. Thermal and humidity exposure of the skin.
5. Manufacturing and inspection access demonstration on a physical article.

Until these exist, the demonstrator output is labelled `synthetic` and must
not be used for a released baseline. Promotion follows ADR-0038.
