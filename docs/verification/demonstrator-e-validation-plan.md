# Demonstrator E — Validation plan (synthetic fixtures)

**Status:** validation plan; physical test data not yet available.
**Demonstrator:** high-temperature atmospheric structural panel.
**Explicit exclusion:** spacecraft, launch vehicles, orbital structures.

## What the synthetic fixtures prove

The `high-temp-panel` plugin executes three real models: interpolation of
temperature-dependent properties between qualification points, through-
thickness heat flux and gradient, and fully-restrained thermal stress with
an oxidation assumption. Each run is content-addressed and replayable. This
proves the engineering chain, not the qualification of a specific material.

## Claimed maturity

- Material: interpolated properties; no rate dependence or creep.
- Thermal: steady conduction; no transient or radiative heating.
- Structure: linear elastic upper-bound stress; no plasticity or buckling.
- Environment: declared oxidation assumption; no kinetics model.
- Manufacturing: declared route and inspection note, not a qualified process.

## Validation needed before production use

1. Temperature-dependent property measurements with statistical basis.
2. Thermal-structural test under a representative temperature-time history.
3. Oxidation/erosion exposure testing in the declared atmosphere.
4. Manufacturing route qualification and NDT capability demonstration.
5. Uncertainty quantification from measured scatter.

Until these exist, the demonstrator output is labelled `synthetic` and must
not be used for a released baseline. Promotion follows ADR-0038.
