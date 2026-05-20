# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What blackbox-bench is

A Python microbenchmarking library with a Rust core via PyO3 + maturin. The Rust extension (`blackbox_bench._blackbox_bench`) provides the sampling loop and statistical primitives; the Python package at `python/blackbox_bench/` is a thin orchestrator that registers user-supplied callables, threads results through the Rust runner, and ships the CLI. Targets Python 3.10–3.13 via a single abi3 wheel.

## Build & dev commands

The crate uses two cargo features that interact in a non-obvious way; read **Crate-type / feature wiring** below before touching `Cargo.toml`.

```bash
# Set up a venv at the repo root (.venv is required by maturin develop)
python -m venv .venv
.venv/bin/pip install --upgrade pip maturin pytest coverage

# Dev build (debug profile, fast incremental rebuilds)
.venv/bin/maturin develop

# Release build (optimised; use for benches & dogfood)
.venv/bin/maturin develop --release

# Build a wheel without installing
.venv/bin/maturin build --release
```

## Tests

```bash
# All Python tests
.venv/bin/python -m pytest -q

# All Rust unit tests (stats + report helpers)
cargo test --lib

# Single test
.venv/bin/python -m pytest tests/test_runner.py::test_runner_respects_warmup -v
cargo test --lib stats::tests::median_odd

# Python coverage (expected: 100%)
.venv/bin/coverage run --source=python/blackbox_bench -m pytest -q
.venv/bin/coverage report

# Combined Python + Rust line coverage via llvm-cov
cargo llvm-cov clean
eval "$(cargo llvm-cov show-env --sh)"
.venv/bin/maturin develop                 # instrumented build (debug)
cargo test --lib
.venv/bin/python -m pytest -q
cargo llvm-cov report --summary-only
```

## Benchmarks

Two layers:

- **`benches/rust_internals.rs`** — criterion benches for the Rust hot paths (stats, formatters). Run with `cargo bench --bench rust_internals`. Criterion stores baselines under `target/criterion/`, so successive runs report deltas automatically.
- **`benches/bench_dogfood.py`** — Python-level end-to-end dogfood. Run with `.venv/bin/python -m blackbox_bench.cli run benches/bench_dogfood.py --warmup 1 --iterations 100`. Used to validate harness overhead-subtraction (`empty_pass` should report ~0–1 ns).

After perf changes, run **both** layers; a Rust-only win that regresses the Python-call orchestration is invisible to criterion.

## CLI

```bash
.venv/bin/python -m blackbox_bench.cli run [PATH] [--warmup N] [--iterations N]
    [--format {table,json,html,xml}] [--xml-style {junit,raw}]
    [--output FILE] [--save FILE] [--profile]
.venv/bin/python -m blackbox_bench.cli compare BASELINE CURRENT [--format ...] [--output FILE]
```

`--profile` shells out to `py-spy` (must be on PATH) and writes per-benchmark SVG flamegraphs. `--json` is a deprecated v0.1.0 alias for `--format json`, removed in v1.1.

## Crate-type / feature wiring

`pyo3`'s `extension-module` feature tells the linker that libpython is provided at runtime (by the host CPython process loading the `.so`). With it on, `cargo test`/`cargo bench` produce binaries that *can't* find libpython and fail to link.

The crate handles this with a feature gate:

```toml
# Cargo.toml
[features]
default = []
extension-module = ["pyo3/extension-module"]

[dependencies]
pyo3 = { version = "0.28", features = ["abi3-py310"] }

[lib]
crate-type = ["cdylib", "rlib"]  # rlib lets tests/benches link against the modules
```

```toml
# pyproject.toml
[tool.maturin]
features = ["extension-module"]   # only enabled when building the wheel
```

So:

- **`maturin develop` / `maturin build`** → cargo gets `--features extension-module` → wheel-shaped cdylib that loads correctly inside the host CPython.
- **`cargo test` / `cargo bench`** → default-features only → libpython links normally → binaries run standalone.

When touching dependencies or crate-type, verify all three modes still work (`maturin develop --release`, `cargo test --lib`, `cargo bench --no-run`).

## Code architecture

### Rust (`src/`)

- `lib.rs` — `#[pymodule]` surface; declares modules as `pub mod` (not just `mod`) so criterion benches under `benches/` can link to them.
- `runner.rs` — `Runner` and `BenchmarkResult` `#[pyclass]`es, plus the timed sampling loop.
  - `run_batch` and the iter_batched inner loop use raw `pyo3::ffi::PyObject_CallNoArgs` / `PyObject_CallObject` + manual `Py_DECREF` to skip PyO3 dispatch layers. These are the only `unsafe` blocks in the codebase; they have explicit SAFETY comments.
  - `BenchmarkResult::from_times` wraps the stats block in `py.detach(|| ...)` (the renamed `allow_threads` in pyo3 ≥0.28) so the bootstrap loop doesn't hold the GIL.
  - The `Runner` owns two scratch buffers (`samples_scratch: Vec<i64>`, `means_scratch: Vec<f64>`) that are reused across every benchmark in a `Bench.run()` session.
  - `PyObject` is aliased locally as `type PyObject = Py<PyAny>;` because the pyo3 prelude no longer exports it as of 0.28.
- `stats.rs` — mean / median / stddev / Tukey / MAD / bootstrap CI. Functions that need a sortable buffer accept a `scratch: &mut Vec<i64>` parameter so the caller controls allocation. `bootstrap_ci_mean` uses an `unsafe` pointer load inside its hot loop; the safety invariant is that `rng.usize(..n)` always returns an index in `[0, n)`.
- `histogram.rs` — wraps the `hdrhistogram` crate; samples above 60 s are saturated to the histogram max (never silently dropped).
- `compare.rs` — JSON-comparison classifier. Uses Python's `json.loads` via `py.import("json")` (no serde dep). Classification is CI-overlap based; `c.ci_low > b.ci_high` is the strict-regression check — touching CIs are deliberately `unchanged`.
- `report.rs` — table / JSON / HTML (with inline SVG sparklines) / JUnit-XML reporters. All `String`-building uses `write!(s, ...)` rather than `s.push_str(&format!(...))` to avoid transient allocations. The HTML's CDATA payload is run through `cdata_safe()` so `]]>` in a benchmark name can't terminate the section early.
- `black_box.rs` — `blackbox_bench.black_box` opaque pass-through.

### Python (`python/blackbox_bench/`)

- `__init__.py` — re-exports the public API. **`Runner` is intentionally importable but not in `__all__`** because its constructor is implementation detail.
- `_bench.py` — the `Bench` orchestrator and the module-level `@benchmark` decorator (which writes to a `_global_registry` list consumed by the CLI). `Bench.run()` assigns to `self._results` after each benchmark so partial results survive an exception from a later one.
- `cli.py` — argparse + importlib discovery. Reconfigures stdout/stderr to UTF-8 at entry (Windows cp1252 can't print `─`/`µ`). `--profile` writes its harness to a `tempfile.NamedTemporaryFile` and passes the bench path/name via env vars; never string-interpolated into `python -c` source.
- `legacy.py` — v0.1.0 compatibility shim. Overrides `Bench.report(json_output=)` (the v1.0 signature is `report(format=, path=, xml_style=)`). Removed in v1.1.

### Tests (`tests/`)

- `conftest.py` — autouse fixture that snapshots and restores `_global_registry` around every test, plus a `fast_bench` factory.
- `test_cli.py` (subprocess-based) **and** `test_cli_unit.py` (in-process `main(argv=...)` calls) overlap intentionally. The in-process tests give coverage; the subprocess tests cover the entry-point wiring. The `PATH=""` subprocess test is Windows-skipped because clearing PATH crashes the Windows interpreter at startup.

### Trickier files to know about

- The probe call in `_bench.Bench.run`: it invokes the registered function once outside the timed loop to detect whether the return is an `IterBatched` instance. Side effects in user benchmarks may run an extra time.
- `target_time_ns` is the budget for `estimate_iters` to derive an iteration count when `iterations` is `None`. Clamped to `[10, 100_000]`.
- The `_synthesize` Rust function builds a 1-sample `BenchmarkResult` from a single `elapsed_ns`. Used by `Bench.measure()` (the context manager).

## Workflow

- Default branch is `develop`; `main` is the release branch.
- CI (`.github/workflows/test.yml`) builds via `pip install -e .` (PEP 517 hook into maturin), not `maturin develop` — that command requires a venv that `actions/setup-python` doesn't create.
- Releases go through `.github/workflows/release.yml`, triggered by a tag matching `v*`. Trusted PyPI publishing — the publish job needs `permissions: id-token: write` and `environment: pypi`.

## Specs and design docs

`docs/superpowers/specs/` and `docs/superpowers/plans/` hold the v1.0 rewrite design and implementation plan. They reflect intent at design time; check the code as source of truth when they conflict.
