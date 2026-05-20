# Changelog

## 1.0.0a1 — 2026-05-19

Major rewrite. The core moves from pure Python to a Rust crate exposed via PyO3. Public API redesigned around criterion-style microbenchmark ergonomics.

### Added
- Rust core via PyO3 + maturin build backend.
- `pybench.black_box(value)` — opaque pass-through.
- Per-batch timing with auto-batch-sizing.
- Bootstrap 95% confidence intervals for mean.
- Tukey / MAD outlier detection (`clean_mean_ns`, `outliers`).
- Per-iteration harness overhead measurement and subtraction.
- `bench.iter_batched(setup, routine)` for setup-isolated benchmarks.
- `throughput=` for MB/s-style reporting.
- `params=[...]` for parameterized benchmarks.
- Opt-in HDR histogram via `Bench(histogram=True)`.
- `pybench.compare(baseline, current)` now classifies rows as unchanged / regressed / improved / new / removed using CI overlap.
- New reporters: HTML (self-contained, inline SVG sparklines) and JUnit-compatible XML (with `--xml-style raw` mirror).
- `pybench run … --profile` wraps each benchmark in `py-spy` and emits SVG flamegraphs.

### Changed
- Build backend is now `maturin`; wheels published for cpython 3.10–3.13.
- `BenchmarkResult` gains `batch_size`, `clean_mean_ns`, `outliers`, `ci95_low_ns`, `ci95_high_ns`, `throughput_per_sec`, `param`, `histogram`.

### Removed
- The pure-Python core. There is no fallback for platforms without a Rust toolchain (sdist + `pip install` requires Rust).

### Deprecated
- `pybench.legacy.*` provides v0.1.0-compatible shapes (decorator, context manager, `BenchmarkResult.to_dict`). Removed in v1.1.
