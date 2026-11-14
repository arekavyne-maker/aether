# Certification dossier index

The S-1 through S-8 evidence rollups, one per SDP §23 success
criterion. Every rollup ties the criterion to:

- The specific code path or conformance test that establishes it
- The dark-build CI step that runs that test
- The contract or fixture name that documents the behavior
- The release version where the rollup was first signed

A green dark-build is the structural guarantee that every S-criterion
is satisfied; the rollups are the human-readable evidence.

| Criterion | Document | Dark-build step | Contract / fixture |
|---|---|---|---|
| S-1 | `s-1.md` | step 2 (`cargo test --workspace`) | The kernel stays under 50k LOC and has no third-party domain libraries |
| S-2 | `s-2.md` | step 3 (contract conformance) | The two-implementation rule is in force |
| S-3 | `s-3.md` | step 2 (`cargo test --workspace`) | `import-linter` rules + prohibited-token scan |
| S-4 | `s-4.md` | step 15 (MVR-04/05/12 conformance) | Cross-hardware: same package on workstation and HPC |
| S-5 | `s-5.md` | step 19 (offline soak) + step 6 (TUF) | Air-gap: `AETHER_OFFLINE=1` enforced |
| S-6 | `s-6.md` | step 2 (`cargo test --workspace`) | Hardware-agnostic: resource-descriptor schema forbids hardware names |
| S-7 | `s-7.md` | step 2 (engine conformance) | Fidelity-honest: result schema requires `fidelity_class` + `uncertainty` |
| S-8 | `s-8.md` | step 2 (engine conformance) | Provenance: hash-chained audit log + per-method record emission |

## How to read a rollup

Each rollup follows the same structure:

- **Criterion**: the verbatim SDP §23 statement.
- **Mechanism**: the code path or fixture that establishes it.
- **Evidence**: a specific test or assertion that demonstrates it.
- **Dark-build step**: which step in `infra/ci/dark-build.sh`
  exercises the evidence.
- **Reference**: the path to the test file or fixture.

## Sign-off

The certification dossier is signed by the Aether Architecture
Review Board on each major release. The signature is recorded in
the release tag's notes; the artifacts are in the `certification/`
directory of the release tag.

A site is production-ready when:

1. The dark-build is green (all 21 steps pass).
2. The S-rollups for the target version are signed.
3. The site's envman StrictVerifier reports `all_resolved: true`.
4. The site has run a smoke workflow end-to-end.

The 180-day offline soak (MVR-08) is the long-term confirmation;
the dark-build is the short-term guarantee.
