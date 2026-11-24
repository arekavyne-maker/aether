# Aether — Website Copy Deck

**Positioning:** Modern infrastructure for aerostructures.

**One-line:** Aether is the air-gapped, vendor-neutral engineering fabric that turns requirements, CAD, simulation, optimization, manufacturing, and test into one traceable digital thread.

**Audience:** Chief engineers, airframe programme leads, structural and certification engineers, platform architects, sovereign/defence engineering sites, and plugin authors at solver/CAD/factory vendors.

**Narrative arc:** *The chain is broken → Aether is the connective layer → here is the architecture → here is the loop running → here is the evidence → here is how you plug in.*

**Voice:** Precise, unsentimental, technically literate. Short declarative sentences. No "revolutionary", "game-changing", "AI-powered", or "seamless". Confidence comes from specificity, not adjectives. Keep the project's honesty: label synthetic data, fixtures, reference runs, and open items.

---

## Template mapping

| Aixor section | Aether section | Job |
|---|---|---|
| Hero quote / CEO | Manifesto quote | Establish the engineering doctrine before the pitch |
| "Imagination Meets Innovation" | "One graph. Every discipline." | State the architecture as a promise |
| About Us + 4 stats | About Aether + proof stats | Explain the broken chain and the layer that fixes it |
| Our Service (4 cards) | The Platform (4 layers) | Make the architecture concrete |
| Our Projects (3) | Reference Runs (3) | Show the closed loop actually running |
| Our Awards (6) | What the Build Enforces (6) | Replace vanity badges with machine-checked evidence |
| Our Members (5) | The Disciplines (5) | Show the team's range without inventing names |
| Become Our Member | Build on Aether | Recruit plugin authors and design partners |
| Pricing Plans (2) | Engagement Models (2) | Replace seat pricing with site-scoped evaluation/deployment |
| Our Reviews | The Three Questions | Replace testimonials with the questions a result must survive |
| Let's Connect | Start the Thread | Technical contact, not a lead magnet |

---

## 01 — Navigation

**Wordmark:** `AETHER`
**Descriptor (optional, under wordmark):** `INFRASTRUCTURE FOR AEROSTRUCTURES`

**Links:**
- Platform
- Contracts
- Thread
- Proof
- Deployment
- Contact

**CTA:** `Request access`

**Mobile CTA:** `Request access`

---

## 02 — Manifesto quote

> “An engineering result is not a number. It is a number, its inputs, its toolchain, its uncertainty, and the chain of decisions that produced it. Aether exists to make that chain the default — not an afterthought.”

**Attribution:** `AETHER PLATFORM PRINCIPLE`

*Note: Do not attribute to a named executive unless the team supplies one. The principle is stronger than a fake founder quote.*

---

## 03 — Hero

**Eyebrow:** `INFRASTRUCTURE FOR AEROSTRUCTURES`

**Headline:**
> *One graph.*
> Every discipline.

**Body:**
> Aether is modern infrastructure for aerostructures: an air-gapped, hardware-agnostic, vendor-neutral engineering fabric. One Engineering Graph connects the requirement, the geometry, the simulation, the factory plan, the inspection, and the test. One bidirectional digital thread carries the evidence between them.

**Primary CTA:** `Explore the platform`
**Secondary CTA:** `See the proof`

**Microline:** `Runs offline. Verifies offline. Replays offline.`

**Visual direction:** A single large "engineering sphere" — part airframe section, part graph. Thin orbital rings, nodes at intersections, a stylized wing-rib section inside, and one iridescent thread tracing through it. The thread is the only bright object. It should read as both a network and a structural section, not as generic AI art.

---

## 04 — About Aether

**Label:** `ABOUT AETHER`

**Heading:**
> Aerostructures are not designed by one tool. They are designed by a chain of them — and the chain is where truth gets lost.

**Body:**
> Requirements live in one system. Geometry in another. Load cases in a spreadsheet. Meshes in a scratch directory. Results in a report. Manufacturing intent in a PDF. Test evidence in someone's inbox. By the time the aircraft is built, the answer to "why is this number what it is?" depends on who is still at the company.

> Aether replaces that patchwork with one Engineering Graph and one bidirectional digital thread. It is not a CAD system, a solver, a PLM suite, or a notebook. It runs no production physics engine of its own. It is the connective, governing, and reproducibility layer that makes the tools you already trust interchangeable parts of one auditable engineering process.

**Status note (small, low-contrast):**
> Current build: substrate-complete for Phases 0–2, with the golden path and scientific loop working end to end. Production integrations are the next phase, delivered with design partners. Every capability claim here is tied to a run, a test, or a named open item.

---

## 05 — Proof stats

**Label:** `BY THE BUILD`

| # | Number | Label | Description |
|---|---|---|---|
| I. | `24` | Versioned contracts | Each with a schema, examples, and a conformance suite. |
| II. | `12` | Enforced invariants | Each with a named mechanism that fails the build. |
| III. | `11` | Benchmarks | Every number from actual execution; absent optional tools skip explicitly. |
| IV. | `0` | Mandatory cloud dependencies | No egress required to build, verify, or replay. |

*Use these as written. Do not add "+" or round up. The precision is the point.*

---

## 06 — The Platform

**Label:** `THE PLATFORM`

**Heading:**
> Four layers. *One source of truth.*

**Intro:**
> Aether is deliberately small at the center and open at the edges. The kernel knows identity, content-addressed storage, graph mechanics, workflow execution, provenance, policy, and plugin lifecycle. Everything domain-specific arrives as a contract-bound plugin.

### 01 — Engineering Graph

> One typed, event-sourced graph for requirements, systems, parts, geometry revisions, meshes, load cases, results, decisions, process plans, inspections, and tests. Not a folder tree. Not a file share. A queryable record of how the aircraft came to be.

**Features:**
- Typed nodes and edges
- Append-only event log
- AS-OF replay
- AQL queries

### 02 — Contract-bound plugins

> Every solver, CAD kernel, mesher, optimizer, and factory system sits behind a versioned contract. Swap the implementation without touching the graph schema or the workflow. The same AWL program runs against whichever plugin satisfies the contract.

**Features:**
- 19 versioned contracts
- JSON-RPC over stdio
- Two-implementation rule
- Language-agnostic SDK

### 03 — Provenance by construction

> Every contract call emits a provenance record: actor, tool version, input and output hashes, placement, and the authorization decision that admitted it. Records chain into a SHA-256 Merkle log. Export as PROV-O, or replay the whole project offline.

**Features:**
- Hash-chained audit log
- Environment manifests
- Signed offline bundles
- Deterministic replay

### 04 — Air-gapped execution

> Built for restricted networks, sovereign programmes, and factory floors without a cloud. No sockets, no daemon, no telemetry. A plugin is a child process whose entire world is stdin and stdout — which is exactly what survives on a locked-down HPC node.

**Features:**
- Egress-free dark build
- Local, HPC, and edge backends
- Ed25519 signing
- TUF-adapted offline trust

---

## 07 — The digital thread

**Label:** `THE DIGITAL THREAD`

**Heading:**
> From requirement to evidence — *and back.*

**Body:**
> The graph is not a filing cabinet. It is a loop. Measured data from inspection and test flows back as a model update, and the update is linked to the evidence that justified it.

**Loop steps:**
`Requirement` → `System` → `Part` → `Geometry` → `Mesh` → `Simulation` → `Result` → `Optimization` → `Manufacture` → `Inspection` → `Test` → `Model update`

**Caption:** `Every link is a typed node or edge. Every transition carries provenance.`

---

## 08 — Reference runs

**Label:** `REFERENCE RUNS`

**Heading:**
> The closed loop, *not a slide.*

**Intro:**
> Aether is proven by runs, not claims. These reference loops execute offline on the bundled implementations. Every number is produced by the code that just ran.

### Run 01 — UAV wing rib

**Tag:** `01 / UAV WING RIB`
**Title:** `Requirement to model update, in one chain.`

> R-STR-011 requires the rib to sustain 1.5 limit load factor across −1g to +4g with no failure. The run traces requirement → system → part → geometry revision → material → load case → simulation → result → optimization → process plan → physical part → inspection → test → measurement dataset → model update → evidence. Nothing is re-keyed. Nothing is orphaned.

**Meta:**
- Domain: `UAV wing rib`
- Chain: `14 artifacts`
- Verdict: `graph-recorded`

### Run 02 — Structural sizing

**Tag:** `02 / STRUCTURAL SIZING`
**Title:** `Fidelity that escalates when it must.`

> The SI governor reads uncertainty, margin proximity, and decision sensitivity, then decides whether to stay or escalate. When it escalates, the workflow re-executes the quantity of interest at the next fidelity rung and records the measured model-form difference — not an assumed one.

**Meta:**
- Contract: `si.governor`
- Decision: `recorded as a graph node`
- Verdict: `measured difference`

### Run 03 — As-built twin

**Tag:** `03 / AS-BUILT TWIN`
**Title:** `A twin that finds the weak station.`

> An 8-station spanwise stiffness distribution is estimated over a real beam FEM. The observation matrix is measured by finite differences, not asserted. A localized root-region deficit is recovered to 2.4/3.8% while healthy stations return to ~1.0 — 3.4× tighter than any uniform scale factor.

**Meta:**
- Domain: `Digital twin`
- Data: `SYNTHETIC, labeled`
- Verdict: `3.4× better than scalar`

**CTA:** `Read the benchmark suite →`

---

## 09 — What the build enforces

**Label:** `WHAT THE BUILD ENFORCES`

**Heading:**
> Evidence, *not adjectives.*

**Intro:**
> Aether treats its own architectural claims as testable. These are not principles on a wall. They are checks that fail the build.

| # | Claim | Explanation | Tag |
|---|---|---|---|
| I. | Two implementations, or it isn't a contract. | No contract is stable until two independent implementations pass the same conformance suite. Three real physics implementations are verified today, and two are cross-checked against each other. One implementation only proves a vendor's accident. | `ADR-0022` |
| II. | No network. No exceptions. | A 24-step dark build compiles, tests, and verifies the entire substrate with zero egress. Air-gap is not a deployment option; it is the build environment. | `I-5` |
| III. | Fidelity is a field, not a claim. | Every result carries `fidelity_class` and `uncertainty`. A solver that cannot verify convergence cannot label a result high-fidelity. | `I-7` |
| IV. | The log is the audit. | Every event carries the authorization decision that admitted it and a SHA-256 link to the event before it. Tamper with history and the chain says so. | `I-8` |
| V. | Reproducibility is executable. | Export a sovereign bundle, move it to an air-gapped machine, replay it. The recorded results re-execute from pinned inputs and tool versions. | `I-9` |
| VI. | Failure is loud. | Typed errors. Fatal contract violations. Divergence aborts. A silently wrong answer is the only unacceptable outcome. | `I-12` |

---

## 10 — The disciplines

**Label:** `THE DISCIPLINES`

**Heading:**
> Small kernel. *Serious surface.*

**Intro:**
> Aether is built by a small platform team and an expanding contract surface. These are the disciplines that keep it honest.

| Monogram | Discipline | Copy |
|---|---|---|
| `KR` | Kernel & runtime | Identity, content-addressed storage, graph mechanics, workflow execution, and plugin lifecycle. The center stays under budget so the edges can grow. |
| `CC` | Contracts & conformance | Versioned schemas, conformance suites, and the two-implementation rule. The contract is the product. |
| `PN` | Physics & numerics | Reference solvers, uncertainty quantification, optimization, surrogates, and twins — checked against closed-form solutions before they are trusted. |
| `PS` | Provenance & security | Hash chains, Ed25519 signing, policy decisions recorded on the events they admit, and an offline trust model that never phones home. |
| `CO` | Certification & operations | Evidence dossiers, environment manifests, site installs, and replay. The work of making a result defensible years later. |

*Names and portraits to be supplied by the team. Do not invent them.*

---

## 11 — Build on Aether

**Label:** `BUILD ON AETHER`

**Heading:**
> Bring your solver. *Keep your truth.*

**Body:**
> Aether is designed to be extended, not adopted wholesale. If you build a CAD connector, a solver, a factory cell, a certification workflow, or a surrogate model, the contract surface is where you plug in. Your implementation can be replaced. That is the point — and it is also the guarantee that your work is judged on capability, not lock-in.

**Bullets:**
- Reference plugins are Python today. The protocol is language-blind.
- Signed bundles install offline.
- Conformance is the entry ticket.

**CTA:** `Request the plugin SDK`

---

## 12 — Engagement models

**Label:** `ENGAGEMENT`

**Heading:**
> Start with a run, *not a slide deck.*

**Intro:**
> Aether is deployed as a site capability, not a seat license. Every engagement begins with an offline run on your problem and a verifiable evidence bundle.

### Evaluate

**Price:** `Offline`
**Subhead:** `For a first honest run.`

- Install and `aether doctor`
- Five deterministic demos
- Benchmark suite, 11 (optional-tool skips explicit)
- Evidence export with graph-chain verdict
- Architecture review session

**CTA:** `Request an evaluation`

### Deploy

**Price:** `Site-scoped`
**Subhead:** `For a site that needs its own truth.`

- Air-gapped site install
- Environment manifest and strict verification
- Policy, RBAC, and signed plugin bundles
- Design-partner integrations: CAD, solver, factory, test
- Certification evidence workflow

**CTA:** `Scope a deployment`

**Footnote:** `From a single workstation to an HPC allocation. Pricing follows scope, not seats.`

*Do not invent prices. "Offline" and "Site-scoped" are honest and preserve the template's two-tier rhythm.*

---

## 13 — The three questions

**Label:** `THE THREE QUESTIONS`

**Heading:**
> What a result *has to survive.*

### “Can I reproduce it?”

> Export the project, move it to an air-gapped machine, replay it. The recorded results re-execute from pinned inputs and tool versions.

### “Why should I trust this number?”

> `aether explain` returns the value, units, analytical reference, relative error, fidelity class, validation status, and the plugin that produced it.

### “Can I replace this tool?”

> The graph schema and the workflow definition do not change when the implementation does. That is what the contract is for.

*This replaces the template's review/testimonial section without fabricating customers.*

---

## 14 — Contact

**Label:** `LET'S CONNECT`

**Heading:**
> Start *the thread.*

**Body:**
> Tell us what you are building, what tools you cannot replace, and what your deployment constraint is. We will reply with a technical path — not a brochure.

**Fields:**
- Name
- Work email
- Organisation
- Discipline (select: Airframe / Structures / Aerodynamics / Manufacturing / Test / Certification / Platform / Other)
- Deployment (select: Air-gapped / On-prem / HPC / Edge / Cloud-adjacent / Not sure)
- What needs to connect? (textarea)

**Submit:** `Start the thread`

**Microcopy:** `No newsletter. No drip sequence. A person who has read the architecture.`

---

## 15 — Footer

**Wordmark:** `AETHER`
**Tagline:** `Modern infrastructure for aerostructures.`
**Line:** `Air-gapped by design. Reproducible by construction. Evidence by default.`

**Columns:**
- **Platform:** Engineering Graph, Contracts, Workflows, Deployment
- **Proof:** Benchmarks, Architecture, Invariants, Status
- **Connect:** Request access, Plugin SDK, Documentation, License

**Legal:**
> © 2026 Aether Platform Team. Aether is distributed under the Aether Proprietary License (Crocell-Proprietary). Authorised recipients only.

---

## SEO & metadata

**Title:** `Aether — Infrastructure for Aerostructures`

**Description:**
> Aether is the air-gapped, vendor-neutral engineering fabric for aerostructures. One Engineering Graph, one bidirectional digital thread, and evidence for every decision.

**OG title:** `Aether — One graph. Every discipline.`

**OG description:**
> Modern infrastructure for aerostructures. Requirements, CAD, simulation, optimization, manufacturing, and test in one traceable digital thread.

**Canonical:** `https://aether.example/` *(replace with the production domain)*

---

## Claims ledger

**Safe to say:**
- 24 versioned contracts; 48 JSON schemas
- 12 enforced architectural invariants
- 11 benchmarks from actual execution, with optional-tool skips reported explicitly
- 24-step egress-free dark build
- 15 reference plugins
- 3 real physics implementations of `physics.solver` (beam FEM, Poiseuille CFD, CalculiX adapter)
- 61 node types and 32 edge types in the graph catalog
- 10M-node graph insert + replay demonstrated
- Air-gapped, offline-verifiable, deterministic replay
- Two-implementation rule, hash-chained provenance, Ed25519 signing, TUF-adapted offline trust

**Do not say:**
- “Certified”, “flight-proven”, “production-grade OpenFOAM”, “real B-rep”
- “Customer”, “client”, “award-winning”
- “Open source” or “free download”
- “EU-funded” / “EIC Pathfinder” unless the funding status is confirmed for the site
- “G8” / “v1.0” / “full platform”
- Any specific price, team name, testimonial, logo, or customer count not supplied by the team

**Always label:**
- The spanwise twin demonstration data as `SYNTHETIC`
- The geometry and manufacturing plugins as reference fixtures where relevant
- The current status as Phase 0–2 substrate + golden path + scientific loop, not the end-state platform

---

## Visual direction

**Palette:** near-black `#050505`, warm off-white `#F4F2EC`, muted graphite `#9A9A94`, hairline borders at 8–14% white. One iridescent accent: ice cyan `#8FE9FF` → violet `#B9A4FF` → warm amber `#FFC78A`.

**Type:** A geometric grotesque for body and headlines (Inter or similar), with an italic serif for one phrase per section (Instrument Serif or similar). The serif is the "human" note against the technical grid.

**Layout:** generous negative space, 1px outlined cards with 24–36px radii, roman numerals for sequences, section labels with four small vertical bars (a direct nod to the template), oversized numerals in the stats.

**Imagery:** no stock photos, no generic AI renders, no fake aircraft. Use one bespoke SVG "engineering sphere" in the hero, and abstract line diagrams for the loop. The only photographic content should be real team portraits and real hardware, when supplied.

**Motion:** restrained. A single animated thread in the hero, subtle card hover, section reveals. Respect `prefers-reduced-motion`.

---

## Open items before launch

1. Replace `https://aether.example/` with the production domain.
2. Add a 1200×630 Open Graph image (the hero sphere on black is the obvious candidate).
3. Wire the contact form to the real endpoint; add privacy note and consent.
4. Supply team names, portraits, and any real customer/design-partner logos.
5. Confirm the funding/affiliation language (EIC Pathfinder vs. none).
6. Confirm which claims are current as of the launch date; the benchmark/test counts are build-specific.
7. Add real documentation links for Architecture, Contracts, Benchmarks, and License.
8. Legal review of the proprietary-license footer line.
