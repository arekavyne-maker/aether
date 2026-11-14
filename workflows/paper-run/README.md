# Paper Run — Composite UAV Wing (Phase 0 G0 acceptance)

This directory holds the **paper run** for the Aether G0 gate: the
composite UAV wing narrative (SDP §3.7) walked end-to-end through every
interface, with concrete examples for every contract and every node type.

The purpose is to **catch interface gaps before code** — to expose
mismatches between the SDP's promised surface and the actual contract
schemas, node types, and edge constraints. Phase 0 ships this as
documents and JSON examples; Phase 1+ replaces the static examples
with executable workflows.

## The chain (SDP §3.7, expanded)

```
Requirement R-STR-011 (limit load, no failure, factor 1.5)
   ▲ satisfies ▲                          ▲ verifies ▲
System: UAV Wing
   └─ decomposes ── Wing Assembly
        └─ decomposes ── Wing Rib #014
             ├─ has_geometry ── GeometryRevision #7 (STEP AP242 + parametric recipe)
             ├─ has_material ── Material: CFRP unidirectional tape M21E
             ├─ subject_to ── LoadCase LC-04 (maneuver, −1g→+4g)
             ├─ simulated_by ── SimulationStudy SS-112 (aeroelastic, coupled)
             │      └─ produced_by ── SimulationRun #2031 (input snapshot 9f3a…)
             │             └─ produced ── Result: tip deflection, margin of safety
             ├─ optimized_by ── OptimizationStudy DS-018 (ply-thickness, NSGA-II + GP surrogate)
             ├─ planned_by ── ProcessPlan PP-201 (AFP layup + autoclave + 5-axis trim)
             │      └─ executed_on ── Machine: AFP-CELL-2 (capability record)
             ├─ realized_as ── PhysicalPart WING-RIB-014-003 (serial, batch genealogy)
             │      ├─ measured_by ── InspectionRecord IR-9987 (laser scan, 98.7% coverage)
             │      └─ measured_by ── TestRun TR-114 (buckling test to 120% LL)
             │             └─ produced ── MeasurementDataset (120 ch, 2.1 MSa)
             │                    └─ updates ── Model: buckling ROM rev 3
             └─ traces_to ── Evidence EV-441 (compliance claim, chain of 14 artifacts)
```

## Files in this directory

| File | Purpose |
|---|---|
| `chain.yaml` | The full node + edge document in YAML, every value schema-validated |
| `awl/wing-screening.awl.yaml` | AWL v1 workflow exercising the chain (Phase 1+ executable) |
| `dossier.md` | The narrative from requirement to model update, in prose |
| `interfaces.md` | The interfaces touched by the paper run, with gaps surfaced |
| `gaps.md` | Open issues that the paper run surfaced for Phase 1+ |

## How to use this

Phase 0 review (G0): read `dossier.md` and `interfaces.md`, then run the
contract conformance harness against `chain.yaml` (this will be wired in
Phase 1+; for Phase 0, the chain is hand-checked against the contract
schemas).

```bash
# Phase 0 manual check
python3 -c "
import json, yaml
from aether_sdk.validation import ContractValidator
v = ContractValidator('aether.graph.model')
schema_id = 'https://aether.local/contracts/graph.model/schema/node-type.schema.json'
data = yaml.safe_load(open('workflows/paper-run/chain.yaml'))
for node in data['nodes']:
    r = v.validate(node, schema_id=schema_id)
    print(node['type_id'], r.valid, r.errors)
"
```

## See also

- SDP §3.7 (Engineering Graph example projection)
- SDP §18 (Phase 0 acceptance)
- `../../contracts/graph.model/catalog/nodes/` — node-type schemas
- `../../contracts/graph.model/catalog/edges/` — edge-type schemas
