# aether-geometry

Aether Python SDK — geometry package. The parametric recipe interpreter
that is the **design intent** in the geometry/recipe/kernel split
(SDP §11.1, MVR-06).

```python
from aether_geometry import Recipe
recipe = Recipe.from_yaml("wing-rib-014.recipe.yaml")
bound = recipe.bind_parameters({"chord_mm": 240.0})
results = bound.regenerate()
for r in results:
    print(r.feature_id, r.kind, r.geometry_handle[:16], r.success)
```

The interpreter is kernel-independent: it walks the feature tree and
produces a stable, content-hashed `FeatureResult.geometry_handle` for
each feature. Real B-rep construction is the geometry.kernel
implementation's job; this module is the typed intent the kernel
consumes.
