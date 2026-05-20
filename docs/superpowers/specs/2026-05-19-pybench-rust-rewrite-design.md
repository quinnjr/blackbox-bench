# pybench v1.0 — Rust Rewrite Design

**Status:** Approved design, ready for implementation planning
**Date:** 2026-05-19
**Author:** Joseph R. Quinn
**Supersedes:** `docs/plans/2026-02-13-pybench-design.md`

## 1. Motivation & Goals

pybench v0.1.0 is a ~250-line pure-Python benchmarking library. v1.0 reorients it
as **the lightest-overhead Python microbenchmarker** — a "criterion for Python"
— by rewriting the core in Rust and exposing it via PyO3.

The honest perf wins from going to Rust are:

- Skipping the Python `for _ in range(n): fn()` bytecode loop itself.
- Replacing `time.perf_counter_ns()` Python attribute lookups with direct
  `Instant::now()` in Rust.
- Storing samples in `Vec<i64>` instead of a Python `list` of boxed ints.
- Stats (`mean`/`median`/`stddev`/`bootstrap_ci`/`tukey`) on large arrays.
- JSON / XML / HTML serialization and comparison logic.

A Rust port cannot make calling a Python callable faster than Python can —
that overhead is intrinsic. The port is worthwhile only because it enables
features that a pure-Python implementation would be too slow to do well:
per-batch timing, bootstrap confidence intervals, outlier detection,
overhead-subtraction, and HDR histograms.

**Non-goals:**

- Profiling user code in production. pybench is a measurement harness.
- Cross-language benchmarking. pybench measures Python callables only.
- A pure-Python fallback (see §2).

## 2. Architecture & Packaging

### Repo layout (single Cargo crate, maturin-driven)

```
pybench/
├── Cargo.toml              # crate: pybench
├── pyproject.toml          # build-backend = "maturin"
├── src/                    # Rust sources
│   ├── lib.rs              # PyO3 #[pymodule] surface
│   ├── runner.rs           # calibration, batching, timing loop
│   ├── stats.rs            # mean/median/stddev/bootstrap_ci/tukey
│   ├── histogram.rs        # HDR samples (hdrhistogram crate)
│   ├── compare.rs          # CI-based classifier
│   ├── report.rs           # table / JSON / HTML / XML formatting
│   └── black_box.rs        # opaque sink
├── python/pybench/
│   ├── __init__.py         # re-exports from the extension
│   ├── _legacy.py          # v0.1.0 shim
│   ├── cli.py              # CLI — stays Python
│   └── py.typed
├── tests/                  # Python integration tests
└── benches/                # dogfood: pybench benching pybench
```

### Build & distribution

- **Build backend:** `maturin`. The current `setuptools` configuration in
  `pyproject.toml` is replaced.
- **Wheels:** cpython 3.10 / 3.11 / 3.12 / 3.13 × {linux x86_64+aarch64,
  macOS x86_64+aarch64, windows x86_64}. Built via maturin's GitHub Actions
  workflow.
- **Free-threaded cpython 3.13t:** deferred to v1.1 (PyO3 free-threaded
  support still stabilising).
- **No pure-Python fallback.** Direction "lightest-overhead microbenchmarker"
  makes a pure-Python parallel implementation a contradiction. sdist requires
  a Rust toolchain.
- **Single crate, not a workspace.** A `pybench-core` / `pybench-py` split
  would only matter if someone wanted to consume the engine from Rust; no
  current requirement. `#[cfg(feature = "extension-module")]` gates the PyO3
  surface if that need arises later.

### CLI stays Python

A Rust binary CLI would have to embed CPython to invoke user-defined
benchmark callables — strictly worse than starting from Python. The CLI is
~80 lines of importlib discovery + argparse and stays in
`python/pybench/cli.py`, importing the Rust extension.

### Cargo dependencies

- `pyo3` (with `extension-module` feature)
- `hdrhistogram`
- `fastrand`

No `serde` (JSON / XML / HTML are built via string concatenation — small,
fixed schemas), no `rayon` (the timing loop is deliberately single-threaded
forever).

## 3. Public Python API (v1.0)

### Top-level exports

```python
from pybench import (
    Bench, BenchmarkResult, benchmark, black_box, compare,
)
```

### `Bench` configuration

```python
Bench(
    warmup: int = 5,
    target_time_ns: int = 1_000_000_000,   # total measurement budget per benchmark
    min_iterations: int = 10,
    max_iterations: int | None = None,
    confidence_level: float = 0.95,
    outlier_method: Literal["tukey", "mad", "none"] = "tukey",
    overhead_subtract: bool = True,        # measure & subtract harness overhead
    histogram: bool = False,               # populate HdrHistogram on each result
)
```

### Registration

Four registration flavors are supported:

```python
# 1. Bare callable, auto-batched
@bench.benchmark
def fib_10():
    fib(10)

# 2. Configured
@bench.benchmark(name="hash", throughput=1024, params=[10, 100, 1000])
def hashing(n):
    hashlib.sha256(b"x" * n).digest()

# 3. Setup-isolated (criterion's iter_batched)
@bench.benchmark
def sort_random():
    return bench.iter_batched(
        setup=lambda: random.sample(range(1000), 1000),
        routine=lambda xs: sorted(xs),
    )

# 4. Inline context manager (kept from v0.1.0)
with bench.measure("section"):
    do_work()
```

Module-level `@benchmark` decorator (registering into a global registry that
the CLI consumes) is preserved.

### `black_box`

`pybench.black_box(x)` returns `x` opaquely. Implemented as a PyO3
`#[pyfunction]` that the optimizer/peephole cannot see through (volatile read
on the underlying `PyObject` pointer before returning).

### `BenchmarkResult` (frozen `#[pyclass]`)

| Field                  | Type                  | Notes |
|------------------------|-----------------------|-------|
| `name`                 | `str`                 |       |
| `times_ns`             | `list[int]`           | one entry per sample (already divided by batch size) |
| `iterations`           | `int`                 | number of samples |
| `batch_size`           | `int`                 | calls per sample |
| `mean_ns`              | `float`               |       |
| `clean_mean_ns`        | `float`               | mean with outliers removed |
| `median_ns`            | `float`               |       |
| `stddev_ns`            | `float`               |       |
| `min_ns`               | `int`                 |       |
| `max_ns`               | `int`                 |       |
| `ops_per_sec`          | `float`               |       |
| `outliers`             | `int`                 | count detected by `outlier_method` |
| `ci95_low_ns`          | `float`               | bootstrap lower bound (10k resamples) |
| `ci95_high_ns`         | `float`               | bootstrap upper bound |
| `throughput_per_sec`   | `float \| None`       | if `throughput` was set on the benchmark |
| `param`                | `Any \| None`         | for parameterized benchmarks |
| `histogram`            | `HdrHistogram \| None`| populated only when `Bench(histogram=True)` |

`to_dict()` and `to_json()` instance methods preserved. Parameterized
benchmarks produce one `BenchmarkResult` per param value, sharing a `name`
and differing `param`.

### `compare`

`compare(baseline_json: str, current_json: str) -> ComparisonReport`. Each
row classified as `unchanged | regressed | improved | new | removed` using
CI overlap (disjoint CIs → regression or improvement; overlapping → noise).
The old `format_comparison` becomes `ComparisonReport.format()`.

### v0.1.0 compatibility shim

`pybench.legacy` re-exports the old `Bench` / `benchmark` / `BenchmarkResult`
shapes and emits a `DeprecationWarning` on import. Shim is dropped in v1.1.

## 4. Rust Internals

### Runner (`src/runner.rs`)

- `calibrate(fn)`: doubling loop finds the smallest batch size `N` where one
  batch takes at least `min_batch_time_ns` (default 5µs, roughly 250× timer
  resolution). Returns `(batch_size, estimated_sample_count)` for the
  configured `target_time_ns`.
- `sample(fn, batch_size, n_samples)`: tight Rust loop. Per sample:
  ```rust
  let start = Instant::now();
  for _ in 0..batch_size { fn.call0(py)?; }
  let elapsed = start.elapsed().as_nanos() as i64 / batch_size as i64;
  samples.push(elapsed);
  ```
- `iter_batched`: the user's benchmark function returns
  `bench.iter_batched(setup=..., routine=...)`. That helper returns a
  tagged `IterBatched` `#[pyclass]` instance that the Runner recognises:
  it calls `setup` once per sample (untimed) to produce `state`, then
  times `routine(state)` × `batch_size`.
- **Overhead subtraction (`overhead_subtract=True`):** at `Bench`
  construction, sample `black_box(())` once and store the median harness
  overhead. Subtract from every later per-call sample.

### Stats (`src/stats.rs`)

- `mean / median / stddev` over `&[i64]` — plain math.
- `tukey_outliers`: Q1 / Q3 via `select_nth_unstable`, filter outside
  `[Q1 - 1.5·IQR, Q3 + 1.5·IQR]`.
- `mad_outliers`: median absolute deviation, threshold 3.5.
- `bootstrap_ci(samples, level, n_resamples=10_000)`: percentile bootstrap
  using `fastrand`. Seed defaults to deterministic; future `Bench(seed=...)`
  can override.

### HDR histogram (`src/histogram.rs`)

Thin wrapper over the `hdrhistogram` crate, exposed as `#[pyclass]
HdrHistogram` with `record()`, `percentile()`, `to_dict()`. Populated only
when `Bench(histogram=True)`.

### Compare (`src/compare.rs`)

```rust
enum Classification { Unchanged, Regressed, Improved, New, Removed }

fn classify(baseline: &Result, current: &Result) -> Classification {
    // If CIs overlap, Unchanged.
    // If disjoint and current.mean > baseline.mean, Regressed.
    // If disjoint and current.mean < baseline.mean, Improved.
}
```

The result is a `ComparisonReport` `#[pyclass]` holding a `Vec<DiffRow>`,
with `format(format="table" | "json" | "html" | "xml")`.

### Reporters (`src/report.rs`)

All four formats live in Rust. `Bench.to_html()`, `Bench.to_xml()`,
`Bench.to_json()`, `Bench.to_table()` return `String`. `Bench.report()`
takes `format=...` and an optional `path=...`.

#### Table
Identical layout to v0.1.0 console table, with extra columns: `CI 95%`,
`Outliers`.

#### JSON
Same shape as v0.1.0 plus the new `BenchmarkResult` fields. `metadata`
block carries python version, platform, timestamp, pybench version, and
the `Bench` config (warmup, target_time_ns, confidence_level,
outlier_method, overhead_subtract).

#### HTML

Single self-contained `.html` file. No external CSS, JS, fonts, or CDN
references. Inline `<style>` and inline `<svg>` only.

- Per-run header: metadata block (python version, platform, timestamp,
  total samples, pybench version).
- Results table — same columns as the console table, plus CI 95% and
  Outliers.
- One inline SVG sparkline per row showing the sample distribution
  (small histogram, ~120×30px). Computed in Rust, emitted as `<path d=…/>`.
- When the source is a `ComparisonReport`: side-by-side baseline/current
  columns with the classifier badge (`regressed`/`improved`/`unchanged`)
  styled via inline class.

HTML is built entirely from Rust via a `String` builder; no templating
engine. HTML escaping handled by a 5-line inline helper.

#### XML

**Default style: JUnit-compatible**, so CI systems (Jenkins, GitLab,
Azure DevOps) can ingest pybench output as test results.

```xml
<testsuite name="pybench" tests="3" failures="1" timestamp="...">
  <testcase name="fib_10" time="0.000042">
    <system-out>{"mean_ns": 42000, "ci95_low_ns": 41800, ...}</system-out>
  </testcase>
  <testcase name="sort_random" time="0.000891">
    <failure message="regression: +18.4% (CI disjoint from baseline)"/>
    <system-out>...</system-out>
  </testcase>
</testsuite>
```

- One `<testcase>` per benchmark.
- `<failure>` appears only on `ComparisonReport` output and only for rows
  classified `regressed`.
- `<system-out>` carries the per-result JSON payload for tools that want
  the full data.

A `--xml-style raw` flag emits a non-JUnit XML mirror of the JSON
structure for users who want pure data interchange. JUnit is the default.

### Profiler integration (`--profile` CLI flag)

The CLI's `pybench run … --profile` flag wraps each benchmark in a
sampling profile by shelling out to `py-spy record -o <name>.svg --
python -c "<bootstrap import + call>"` per benchmark, and emits SVG
flamegraphs alongside the JSON output.

We do not link the `py-spy` crate — it would inflate the wheel
substantially and the subprocess boundary is fine here. `py-spy` is
declared as a documented external dependency the user installs
themselves.

## 5. CLI

`python/pybench/cli.py` — preserved Python entrypoint, imports the Rust
extension.

```
pybench run [PATH] [--warmup N] [--iterations N]
            [--format {table,json,html,xml}] [--output FILE]
            [--xml-style {junit,raw}]
            [--profile]
            [--save FILE]
pybench compare BASELINE CURRENT [--format {table,json,html,xml}] [--output FILE]
```

Discovery (`bench_*.py`, `*_bench.py`) and arg parsing remain in Python.

## 6. Testing

### Rust unit tests
- Stats: `mean` / `median` / `stddev` invariants; bootstrap CI contains the
  true mean ≥ 95% of the time on synthetic data (proptest).
- Outlier detection: known-good fixtures with hand-checked Tukey / MAD
  outputs.
- Batch sizing: synthetic callable with controlled cost — assert batch
  size scales inversely with cost.
- Reporters: golden tests for table / JSON / XML / HTML against fixed
  `BenchmarkResult` inputs.

### Python integration tests
The existing `tests/` suite adapts 1:1 to the new API except:
- `tests/test_runner.py` — rewritten against the new Runner.
- `tests/test_init.py` — re-exports change.
- `tests/test_compare.py` — extended to cover CI-based classification.
- New: `tests/test_reporters.py` covering HTML / XML golden outputs from
  the Python side.

### Dogfood (`benches/`)
A pybench script benching pybench itself. Validates that
`overhead_subtract` works (benchmarking a `pass` function should report
~0ns ± a few ns) and that recorded times for a known-cost function (e.g.
`time.sleep(0.001)`) land within CI of the expected value.

## 7. CI

`.github/workflows/`:

- `test.yml` — matrix `{3.10, 3.11, 3.12, 3.13} × {ubuntu, macos,
  windows}`. Runs `maturin develop --release` + `pytest`.
- `release.yml` — triggered on tag push. `maturin build --release` for
  each target; `cibuildwheel` cross-compiles aarch64 wheels; `twine
  upload` publishes to PyPI.

## 8. Migration from v0.1.0

- v0.1.0 → v1.0 breaking changes are documented in `CHANGELOG.md` and
  `MIGRATION.md`.
- `pybench.legacy` shim re-exports v0.1.0 shapes for one minor version
  (v1.0). Importing it emits `DeprecationWarning`.
- Shim is removed in v1.1.
- The docs site (`docs/index.html`) is regenerated against the v1.0 API
  surface.

## 9. Open Issues / Future Work (post-v1.0)

- Free-threaded cpython 3.13t wheels.
- Parallel benchmark scheduling (rejected for v1.0 — fundamentally at
  odds with microbenchmark fidelity).
- `pybench-core` Rust-only crate split, if anyone asks.
- `Bench(seed=...)` for deterministic bootstrap CI runs.
