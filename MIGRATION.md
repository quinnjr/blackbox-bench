# Migrating from blackbox_bench 0.1.0 to 1.0

## Drop-in shim

For one minor release (v1.0 only), the old API is available at:

```python
from blackbox_bench.legacy import Bench, benchmark, BenchmarkResult
```

Importing it emits a `DeprecationWarning`. The shim is removed in v1.1.

The legacy shim preserves the v0.1.0 shapes that changed in v1.0:

- `legacy.Bench.report(json_output: bool = False)` — accepts the v0.1.0 kwarg and dispatches to the new `format="json"` path internally. (The v1.0 `Bench.report` signature is `report(format, path, xml_style)`; calling it with `json_output=True` directly raises `TypeError`.)
- `legacy.Bench.__init__(warmup, iterations, target_time_ns)` — same v0.1.0 keyword set; the new kwargs (`confidence_level`, `outlier_method`, `overhead_subtract`, `histogram`, `seed`) are unused.

## Recommended upgrade

The v1.0 API is mostly a superset of v0.1.0. Most code only needs an import swap:

```python
# v0.1.0
from blackbox_bench import Bench, benchmark

# v1.0 — same imports
from blackbox_bench import Bench, benchmark
```

What changed:

- `BenchmarkResult` gained fields. Existing field names are preserved.
- `Bench.__init__` accepts new kwargs (`confidence_level`, `outlier_method`, `overhead_subtract`, `histogram`, `seed`); defaults match v0.1.0 semantics where they overlap.
- **`Bench.report(json_output=True)` → `Bench.report(format="json")`.** The `json_output` kwarg is gone in v1.0. Either switch to `format="json"` or use the legacy shim.
- **CLI `--json` → `--format json`.** The old `--json` flag is kept as a deprecated alias in v1.0 (emits a stderr warning) and is removed in v1.1.
- `compare()` returns a `ComparisonReport`, not `list[dict]`.

## Migrating `compare()` consumers

`ComparisonReport.rows` is a `list[DiffRow]`, where `DiffRow` is a frozen class exposing the per-row fields as attributes (`row.name`, `row.classification`, `row.baseline_mean_ns`, `row.current_mean_ns`, `row.change_pct`):

```python
# v0.1.0 — list[dict]
for row in compare_results(baseline, current):
    print(row["name"], row["change_pct"])

# v1.0 — DiffRow attribute access
report = blackbox_bench.compare(baseline_json, current_json)
for row in report.rows:
    print(row.name, row.change_pct)
```

If you want a dict-shaped payload (e.g. for serialising), call `.format("json")`:

```python
import json
data = json.loads(report.format("json"))
for row in data["rows"]:
    print(row["name"], row["change_pct"])
```
