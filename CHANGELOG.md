# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed
- **Project renamed from `pybench` to `blackbox-bench`.** The PyPI distribution is now `blackbox-bench`; the Python module is `blackbox_bench`; the CLI is `blackbox-bench`; the Rust extension is `_blackbox_bench`. The XML reporter's root element changes from `<pybench>` (and `<testsuite name="pybench">`) to `<blackbox-bench>` (and `<testsuite name="blackbox-bench">`). No behaviour changes — only names.

### Migration
- `pip uninstall pybench && pip install blackbox-bench`
- `import pybench` → `import blackbox_bench`
- `from pybench.legacy import …` → `from blackbox_bench.legacy import …`
- `pybench …` CLI invocations → `blackbox-bench …`
- CI consumers that parse `<pybench>` or `<testsuite name="pybench">` from the XML reporter need to update to `<blackbox-bench>`.

## [1.0.0] — 2026-05-20

First stable release of the Rust-core rewrite. Folds in everything from the `1.0.0a1` pre-release plus the post-alpha hardening (dependency bumps, perf wins, security fixes, and review feedback). Drop-in for `1.0.0a1`; users who installed the alpha can upgrade with no code changes.

### Added
- `CLAUDE.md` — build, test, and architecture notes for Claude Code working in the repo.
- Expanded `README.md` with quick-start, feature overview, and CLI usage.
- `develop` branch is now the integration target; `main` is the release branch.
- Criterion 0.8 benchmark target at `benches/rust_internals.rs` covering stats, histogram, and reporter hot paths.

### Changed
- Bumped `pyo3` from 0.22.6 → 0.28.3 in two steps (Dependabot PR #2 → 0.24.1; PR #3 → 0.28.3). All deprecation warnings cleared (`import_bound`/`new_bound`/`eval_bound`/`downcast` renames; `allow_threads` → `detach`).
- `bootstrap_ci_mean` now uses partial-partition selection (`select_nth_unstable_by`) instead of a full sort — ~15% faster per benchmark.
- HTML / XML / comparison reporters write directly into the destination `String` with `write!` instead of `push_str(&format!(...))` — ~13% faster on `format_html` at n=100.
- The `iter_batched` timed loop hoists its args tuple outside `Instant::now()` and calls `PyObject_CallObject` via raw FFI; the non-batched loop similarly uses `PyObject_CallNoArgs`. Removes refcount churn from the measurement window.
- Stats functions (`median`, `tukey`, `mad`, `bootstrap_ci_mean`) accept a caller-owned scratch buffer; `Runner` owns the buffers and reuses them across every benchmark in a `Bench.run()` session.
- The overhead probe runs once per `Bench.run()` (cached on the first `Runner`) instead of once per registered benchmark.
- Stats computation runs inside `py.detach(...)` so the GIL is released during the bootstrap.

### Fixed
- HDR histogram samples above 60 s are now saturated to the histogram max rather than silently dropped.
- `Bench.run()` assigns to `self._results` incrementally so an exception from a later benchmark no longer loses the earlier results.
- `Bench.report(json_output=True)` works again on `pybench.legacy.Bench` (the v0.1.0-compat shim was missing the override).
- `bootstrap_ci_mean`'s `partial_cmp().unwrap()` is now `total_cmp` — NaN-safe even if the i128 sum ever produced a non-canonical f64.
- CDATA payload in the JUnit XML reporter escapes `]]>` so a benchmark name can't terminate the CDATA section early.
- CLI `--profile` writes its harness to a `tempfile.NamedTemporaryFile` and passes the benchmark path + name via environment variables — no string-interpolated `python -c` source any more.
- `cli.main` reconfigures `sys.stdout`/`sys.stderr` to UTF-8 so Windows cp1252 consoles can print `─` and `µ`.
- `_discover` warns on a per-file import failure and continues, instead of crashing mid-suite.
- `compare()` error messages prefix `baseline:` or `current:` so it's clear which input is malformed.
- `Runner` rejects `iterations=0` with a `PyValueError` instead of panicking in release builds.
- `compare()` rejects baseline/current payloads >50 MB before handing them to `json.loads`.
- CI workflow now uses `pip install -e .` (PEP 517 hook → maturin) instead of `maturin develop`, which required a virtualenv that `actions/setup-python` doesn't provide.

### Removed
- `Runner` removed from `pybench.__all__` (still importable; not part of the stability contract).

### Security
- Release workflow gained `permissions: id-token: write` and `environment: pypi` for PyPI trusted publishing.

### Internal
- 107 Python tests + 12 Rust unit tests + 9 criterion bench groups; Python coverage 100% (225/225 statements).
- v0.1.0 retroactively tagged at the last pre-rewrite commit.

## [1.0.0a1] — 2026-05-19

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
- `compare()` returns a `ComparisonReport` instead of `list[dict]`.
- `Bench.report(json_output=True)` → `Bench.report(format="json")`.
- CLI `--json` → `--format json`.

### Removed
- The pure-Python core. There is no fallback for platforms without a Rust toolchain (sdist + `pip install` requires Rust).

### Deprecated
- `pybench.legacy.*` provides v0.1.0-compatible shapes (decorator, context manager, `BenchmarkResult.to_dict`, `Bench.report(json_output=)`). Removed in v1.1.
- CLI `--json` flag — replaced by `--format json`, removed in v1.1.

## [0.1.0] — 2026-02-13

Initial pure-Python release. `Bench` class with `@benchmark` decorator and `measure()` context manager; `Runner` with warmup + auto-calibration; mean / median / stddev / min / max stats; table and JSON reporters; `pybench run` / `pybench compare` CLI. Zero required runtime dependencies.

[Unreleased]: https://github.com/quinnjr/pybench/compare/v1.0.0...develop
[1.0.0]: https://github.com/quinnjr/pybench/compare/v1.0.0a1...v1.0.0
[1.0.0a1]: https://github.com/quinnjr/pybench/compare/v0.1.0...v1.0.0a1
[0.1.0]: https://github.com/quinnjr/pybench/releases/tag/v0.1.0
