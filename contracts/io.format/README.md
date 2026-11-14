# `aether.io.format` Contract v0

Data format readers/writers contract. Wraps STEP AP242, CGNS, HDF5/XDMF, Zarr, VTK, IGES, STL, and any solver-native format behind a single read/write interface.

## Versioning

- **Contract id:** `aether.io.format`
- **Schema version:** `0.1.0` (Phase 0 v0; v1.0 freeze at Phase 2 G2)

## Methods

| Method | Description |
|---|---|
| `read(cas_hash)` | Returns typed in-memory model. |
| `write(model)` | Returns CAS hash. |
| `schema_of()` | Returns a JSON Schema describing the in-memory model. |

## Layout

```
io.format/
├── manifest.json
├── schema/
│   └── format-descriptor.schema.json
├── examples/
│   ├── step-ap242.descriptor.yaml
│   ├── cgns.descriptor.yaml
│   └── hdf5.descriptor.yaml
└── README.md
```

## Standards adopted (SDP §16)

| Standard | Role |
|---|---|
| STEP AP242 | Geometry + PMI + assembly exchange |
| CGNS | CFD data exchange |
| HDF5 + XDMF, Zarr | Field/array storage |
| VTK formats | Visualization exchange |

## See also

- SDP §10.3 (field data model)
- SDP §11.1 (geometry without a locked kernel)
- SDP §16 (standards strategy)
- ADR-0016 (contracts as data)
