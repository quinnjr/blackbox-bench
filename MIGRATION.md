# Migrating from pybench 0.1.0 to 1.0

## Drop-in shim

For one minor release (v1.0 only), the old API is available at:

```python
from pybench.legacy import Bench, benchmark, BenchmarkResult
```

Importing it emits a `DeprecationWarning`. The shim is removed in v1.1.

## Recommended upgrade

The v1.0 API is mostly a superset of v0.1.0. Most code only needs an import swap:

```python
# v0.1.0
from pybench import Bench, benchmark

# v1.0
from pybench import Bench, benchmark  # same names — same behaviour
```

What changed:

- `BenchmarkResult` gained fields. Existing field names are preserved.
- `Bench.__init__` accepts new kwargs (`confidence_level`, `outlier_method`, `overhead_subtract`, `histogram`, `seed`); defaults match v0.1.0 semantics where they overlap.
- `compare()` now returns a `ComparisonReport` object, not a `list[dict]`. Call `.rows` for the structured data or `.format("table")` for output.

If you depended on `compare()` returning a list, use `[row for row in pybench.compare(...).rows]` or switch to `report.format("json")`.
