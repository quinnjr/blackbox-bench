# pybench

A lightweight Python microbenchmarking library with a Rust core (PyO3 + maturin). Designed as a "criterion for Python" — minimal harness overhead, statistically rigorous output, and a small CLI that drops into CI.

```python
import pybench

bench = pybench.Bench()

@bench.benchmark
def hash_1kb():
    import hashlib
    hashlib.sha256(b"x" * 1024).digest()

bench.run()
bench.report()                # human-readable table
bench.report(format="json", path="results.json")
```

```
pybench results
─────────────────────────────────────────────────────────────────────────────────
Name        Mean   Median  StdDev    Min    Max  Ops/sec          CI 95%  Outliers
─────────────────────────────────────────────────────────────────────────────────
hash_1kb  1.4 µs   1.4 µs  12.0 ns 1.4 µs 1.5 µs  710,221  [1.4 µs, 1.4 µs]        3
─────────────────────────────────────────────────────────────────────────────────
```

## Features

- **Per-batch timing with auto-batch-sizing** — sub-microsecond functions are batched until each sample takes ≥5 µs, well above the timer resolution floor.
- **Per-iteration overhead measurement and subtraction** — the harness probes itself at startup so reported nanoseconds attribute time to your code, not pybench.
- **Bootstrap 95% confidence intervals** for the mean, plus Tukey or MAD outlier detection.
- **`pybench.black_box(value)`** — opaque pass-through that the optimiser can't see through.
- **`bench.iter_batched(setup, routine)`** — setup runs once per sample (untimed); routine runs the timed batch.
- **`throughput=`** — `@bench.benchmark(throughput=1024)` reports MB/s alongside ops/sec.
- **`params=[...]`** — parameterised benchmarks; one result per parameter.
- **Opt-in HDR histograms** — `Bench(histogram=True)` attaches a percentile-queryable `HdrHistogram` to each result.
- **`pybench.compare(baseline_json, current_json)`** — classifies each row as `unchanged` / `regressed` / `improved` / `new` / `removed` using CI overlap, not raw `change_pct`.
- **Four reporters** — table, JSON, self-contained HTML (with inline SVG sparklines), JUnit-compatible XML (with a raw alternative).
- **`pybench run --profile`** — wraps each benchmark in [`py-spy`](https://github.com/benfred/py-spy) and emits SVG flamegraphs alongside the results.

## Install

```bash
pip install pybench
```

Pre-built abi3 wheels are published for cpython 3.10 / 3.11 / 3.12 / 3.13 on linux (x86_64 + aarch64), macOS (x86_64 + aarch64), and windows x86_64. Building from source requires a Rust toolchain.

Optional extras:

```bash
pip install pybench[profile]   # adds py-spy for `pybench run --profile`
```

## Usage

### Decorator API

```python
import pybench

bench = pybench.Bench(
    warmup=5,
    target_time_ns=1_000_000_000,
    outlier_method="tukey",        # or "mad" / "none"
    overhead_subtract=True,
    histogram=False,
)

@bench.benchmark
def quick(): sum(range(100))

@bench.benchmark(name="hash_kb", throughput=1024, params=[10, 100, 1000])
def hashing(n):
    import hashlib
    hashlib.sha256(b"x" * n).digest()

results = bench.run()
bench.report(format="html", path="results.html")
```

### `iter_batched` for setup-isolated timing

```python
import random

@bench.benchmark
def sort_random():
    return bench.iter_batched(
        setup=lambda: random.sample(range(1_000), 1_000),
        routine=lambda xs: sorted(xs),
    )
```

`setup` runs once per *sample* and isn't timed; `routine` is the timed call inside the batch.

### Context manager

```python
with bench.measure("payload_build"):
    payload = build_huge_payload()

bench.run()  # appends measured contexts to results
```

### Module-level decorator + CLI

```python
# bench_hashing.py
import pybench

@pybench.benchmark
def sha256_1kb():
    import hashlib
    hashlib.sha256(b"x" * 1024).digest()
```

```bash
pybench run bench_hashing.py --warmup 5 --iterations 100
pybench run benches/ --format html --output report.html
pybench run benches/ --save baseline.json
pybench compare baseline.json current.json   # CI-classified diff
```

### Comparing runs

```python
import json
report = pybench.compare(
    open("baseline.json").read(),
    open("current.json").read(),
)
for row in report.rows:
    print(row.name, row.classification, row.change_pct)
```

`ComparisonReport.format("xml")` emits JUnit with `<failure>` on regressed rows — drop the file into Jenkins/GitHub Actions test reporters.

## Why Rust under the hood

The harness has to stay small relative to the user's function:

- **Tight sampling loop** — `Instant::now()` and the per-batch call dispatch are raw FFI (`PyObject_CallNoArgs` / `PyObject_CallObject`), skipping PyO3's higher-level wrappers inside the timed window.
- **GIL released during stats** — bootstrap CI, Tukey/MAD, and the histogram run inside `py.detach(...)` so other Python threads aren't blocked while pybench crunches its samples.
- **Reused scratch buffers** — `median`, `tukey`, `mad`, and `bootstrap_ci_mean` share two `Vec`s owned by the `Runner`; a 100-benchmark suite still allocates only once for the lot.

The criterion benchmarks at `benches/rust_internals.rs` measure these primitives directly. `benches/bench_dogfood.py` measures the assembled harness end-to-end (the empty-pass benchmark should report ~0–1 ns after overhead subtraction).

## Migrating from 0.1.0

See [MIGRATION.md](MIGRATION.md). Most v0.1.0 code only needs an import swap; the one surface that changed without alias is `Bench.report(json_output=True)` → `Bench.report(format="json")`. For exact v0.1.0 semantics:

```python
from pybench.legacy import Bench, benchmark, BenchmarkResult
```

The legacy shim is removed in v1.1.

## License

MIT — see [LICENSE](LICENSE).
