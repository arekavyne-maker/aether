# Aether 3.0 — Research Register

**Status:** source register, 2026-11-24.
**Rule:** a source informs a research direction; it does not by itself prove
production fitness. Bibliographic metadata must be re-verified before a
citation is used in an engineering claim. Entries marked *metadata pending*
are listed from the target-architecture brief and must be resolved against the
publisher record.

| ID | Source | Evidence class | Applicability to Aether | Limitations / validation needed | Status |
|---|---|---|---|---|---|
| R-01 | IASDM technical topics, https://www.iasdm.net/technical-topics/ | Community taxonomy | Confirms adaptive/multifunctional structures, metamaterials, SHM, AI-assisted analysis, uncertainty, evidence are active aerospace topics | Not a peer-reviewed result; use for vocabulary only | metadata pending |
| R-02 | Applications of Machine Learning in Analysis and Design of Aerospace Composite Structures, *Thin-Walled Structures*, 2026, doi:10.1016/j.tws.2025.113914 | Peer-reviewed review | ML in composite analysis/design; physics-informed approaches and limitations | Review-level evidence; no single method validated for certification | metadata pending |
| R-03 | Recent advancements in morphing applications, *Aerospace Science and Technology*, 2025, doi:10.1016/j.ast.2025.110102 | Peer-reviewed review | Morphing architecture, AI integration, challenges | Case-specific; manufacturing and fatigue validation still needed | metadata pending |
| R-04 | Aerodynamic coefficient prediction of bio-inspired camber morphing wings using an explainable transformer, *AST*, 2026, doi:10.1016/j.ast.2025.111557 | Peer-reviewed case study | Explainable, physics-aware prediction for morphing wings | One geometry family; not generalizable without external validation | metadata pending |
| R-05 | From bio-inspired concepts to intelligent design, *Advanced Bionics*, 2026, doi:10.1016/j.abs.2026.08.008 | Peer-reviewed review | Digital representation, structure-property databases, inverse design | Bio-inspired transfer needs manufacturability and scale-up evidence | metadata pending |
| R-06 | Data-driven inverse design of load-bearing 3D metamaterials, *Advanced Engineering Informatics*, 2026, doi:10.1016/j.aei.2026.104536 | Peer-reviewed method | Physics-embedded, multi-fidelity inverse design | Coupon-scale; aircraft-scale transfer unproven | metadata pending |
| R-07 | The Future of Mechanical Metastructures, 2026 review | Peer-reviewed review | Structural genomes, AI-assisted design, autonomous experimentation | Vision-level; no implementation recipe | metadata pending |
| R-08 | Functional nano-architected mechanical metamaterials and devices, *Nature*, 2025, doi:10.1038/s44455-025-00010-9 | Peer-reviewed review | Multifunctional architected structures, programmable behavior | Nanoscale; scale-up and manufacturing validation required | metadata pending |
| R-09 | Passive Morphing in Insect Wings, 2026 review, PMC13702232 | Peer-reviewed review | Distributed compliance, anisotropy, hierarchy, multifunctionality | Biological analogy; engineering transfer needs structural evidence | metadata pending |
| R-10 | Neural operators for accelerating scientific simulations and design, *Nature Reviews Physics*, 2024, doi:10.1038/s42254-024-00712-5 | Peer-reviewed perspective | Operator learning, PDE solution maps, inverse design | Not a universal solver replacement; OOD and uncertainty required | metadata pending |
| R-11 | Attaining physics-driven convolutional operators by architecture design, *Communications Physics*, 2026, doi:10.1038/s42005-026-02613-8 | Peer-reviewed method | Physics-driven operator learning with reduced data dependence | Benchmark-specific; needs external validation | metadata pending |
| R-12 | Machine learning for discovery of governing equations, 2026 review | Peer-reviewed review | SINDy, neural ODEs, operator learning, Koopman, reservoir computing | Discovered equations are hypotheses; dimensional/conservation checks required | metadata pending |
| R-13 | Toward quality-assured intelligent welding processes for aerospace structures, *J. Manufacturing Processes*, 2026, doi:10.1016/j.jmapro.2026.09.081 | Peer-reviewed review | Multimodal sensing, adaptive control, uncertainty, certification gap | Process-specific; no certification-grade evidence implied | metadata pending |
| R-14 | Emerging processes, machine learning and applications in composite additive manufacturing, *Materials Advances*, 2026, doi:10.1039/d6ma00219f | Peer-reviewed review | Hybrid physics/data twins, digital thread, edge/cloud | Process and material specific | metadata pending |
| R-15 | Multi-layer knowledge and data-driven integrated framework for smart manufacturing process planning, *JIII*, 2026, doi:10.1016/j.jii.2025.101045 | Peer-reviewed case study | Semantic integration across CAD/ERP/MES/twin/process planning | One aerospace sheet-metal application; performance not generalizable | metadata pending |
| R-16 | Digital twin-enabled system for real-time monitoring of aircraft tooling, *J. Manufacturing Systems*, 2026, doi:10.1016/j.jmsy.2026.02.010 | Peer-reviewed case study | Assembly tooling monitoring and displacement reconstruction | Tooling-specific; sensor and calibration dependent | metadata pending |
| R-17 | Automated Assembly of Large-Scale Aerospace Components, *Sensors*, 2026, doi:10.3390/s26082294 | Peer-reviewed survey | Pose adjustment, positioning, digital measurement, trajectory control | Survey-level; safety and certification not established | metadata pending |

## Interpretation rules

- Distinguish peer-reviewed results from vendor claims, preprints, conference
  abstracts, and conceptual proposals.
- Record scale, material, process, geometry, environment, and validation
  method.
- Do not extrapolate coupon results to a full airframe without a scaling
  argument and validation plan.
- Do not treat speed-up as accuracy evidence.
- Do not treat connected sensor data as a validated digital twin.
- Record negative results and known limitations.
- Re-check standards, regulations, and certification guidance with qualified
  specialists before any compliance claim.

## Promotion rule

A research method may enter a controlled workflow only after:

1. a contract exists;
2. a benchmark suite exists;
3. independent validation is recorded;
4. uncertainty and OOD behavior are characterized;
5. a model card is approved for the intended use;
6. rollback is tested.
