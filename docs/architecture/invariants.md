# The twelve invariants

The platform's constitution. Each invariant is enforced by a named
mechanism; an invariant without enforcement is a wish.

## I-1 Microkernel minimality

The Aether Core contains only:

- Identity
- ArtifactStore (content-addressed store)
- GraphEngine
- WorkflowEngine
- EventBus
- CapabilityRegistry
- ProvenanceService
- PolicyService
- PluginLifecycle
- ConfigService
- ExecService

The kernel stays under **~50k lines of Rust**. Adding anything else
requires an ADR proving the capability cannot exist as a plugin.

**Enforcement**: LOC budget tracker in CI; any kernel change that
exceeds the budget is blocked; ARB review required for exceptions.

## I-2 Everything else is a plugin

No solver, CAD kernel, mesh engine, database, AI framework,
manufacturing system, or scientific algorithm may reside in the
kernel.

**Enforcement**: kernel dependency scan rejects third-party domain
libraries; conformance tests prove each contract is swappable.

## I-3 Zero hard-coupling

Domain modules never import another domain's implementation. They
communicate only through contracts and Engineering Graph objects.
No `if solver == X` branching anywhere.

**Enforcement**: `import-linter` rules in CI; prohibited-token scan
catches `solver ==`, `cad_vendor`, `is_nvidia`, etc.

## I-4 Graph stability under implementation change

Replacing a solver, CAD system, optimizer, or compute backend does
not change the Engineering Graph schema, project structure, or
workflow definitions.

**Enforcement**: MVR-04/05/12 conformance tests assert the same
package produces equivalent results across backends; schema
frozen at the contract level.

## I-5 Air-gap by design

No mandatory cloud, internet, telemetry, or external SaaS. Every
capability must function fully offline; any optional online feature
must degrade silently and cleanly.

**Enforcement**: `AETHER_OFFLINE=1` env var; network-aware wrappers
fail closed; MVR-08 180-day offline soak conformance.

## I-6 Hardware-agnostic execution

Workflows and plugin manifests declare **computational
requirements** (memory_gb, cores, parallelism, accelerator_class,
architecture), never hardware identifiers.

**Enforcement**: resource-descriptor schema rejects hardware names
in workflow documents; prohibited-token scan in CI.

## I-7 Physics-first, fidelity-honest

Every quantitative claim carries an explicit fidelity, validity
envelope, and uncertainty statement. Surrogates never silently
replace physics; the result schema distinguishes high-fidelity
results, ROM outputs, and surrogate predictions.

**Enforcement**: result schema requires `fidelity_class` +
`uncertainty` fields; CI rejects artifacts missing them.

## I-8 Provenance completeness

Every artifact, decision, and execution is traceable to inputs,
tool versions, parameters, environment, and authorization.

**Enforcement**: provenance record is mandatory output of every
plugin contract method; graph rejects orphan artifacts; hash-chained
audit log.

## I-9 Reproducibility

An engineering package pins exact versions and content hashes of
plugins, environments, and inputs; historical re-execution is
supported, not best effort.

**Enforcement**: envman lockfile; MVR-09/11/12 conformance;
StrictVerifier refuses to start a workflow if any pin is unresolved.

## I-10 CLI-canonical interface

The CLI is the canonical interface; every CLI capability is available
programmatically through the same underlying APIs; any GUI is a
client of those APIs, never a parallel implementation.

**Enforcement**: GUI/CLI parity tests; CLI command coverage tracked
per release.

## I-11 Contracts versioned + 2-impl rule

Every contract has SemVer, a published schema, a conformance suite,
and two independent implementations.

**Enforcement**: contract packages versioned independently; no
contract released without its conformance suite; two-implementation
badge awarded by the Contract Certification Board.

## I-12 Fail-visible, never fail-silent

Failed validation, unverifiable provenance, policy violations, and
plugin contract breaches halt with explicit diagnostics — never
degrade into plausible-looking output.

**Enforcement**: typed error taxonomy; kernel treats contract
violations as fatal to the run, recorded as evidence.

## What this means in practice

Each invariant is **structural**, not procedural. It is enforced by
a code path, not by a checklist. The dark-build CI runs every
invariant through its enforcement point on every commit; a green
dark-build is the structural guarantee that the platform still
satisfies its constitution.
