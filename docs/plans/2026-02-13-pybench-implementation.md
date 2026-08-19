# pybench Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a zero-dependency Python benchmarking library with decorator/context-manager APIs, CLI, JSON export, and run comparison.

**Architecture:** `Bench` class orchestrates benchmark collection and execution. `Runner` handles timing with `time.perf_counter_ns()`, warmup, and auto-calibration. `Reporter` formats output as terminal tables or JSON. `Compare` diffs two JSON result files. CLI uses argparse with `run` and `compare` subcommands.

**Tech Stack:** Python 3.10+ stdlib only (time, statistics, argparse, dataclasses, json, gc, importlib). `rich` optional for pretty tables.

---

### Task 1: Project Scaffolding

**Files:**
- Create: `pyproject.toml`
- Create: `src/pybench/__init__.py`
- Create: `tests/__init__.py`

**Step 1: Create pyproject.toml**

```toml
[build-system]
requires = ["setuptools>=68.0"]
build-backend = "setuptools.backends._legacy:_Backend"

[project]
name = "pybench"
version = "0.1.0"
description = "A lightweight Python benchmarking library"
requires-python = ">=3.10"
license = "MIT"

[project.optional-dependencies]
rich = ["rich>=13.0"]
dev = ["pytest>=7.0"]

[project.scripts]
pybench = "pybench.cli:main"

[tool.setuptools.packages.find]
where = ["src"]

[tool.pytest.ini_options]
testpaths = ["tests"]
```

**Step 2: Create empty package files**

Create `src/pybench/__init__.py` with:
```python
"""pybench - A lightweight Python benchmarking library."""
```

Create `tests/__init__.py` as empty file.

**Step 3: Install in dev mode and verify**

Run: `cd /home/joseph/Projects/pybench && python -m venv .venv && source .venv/bin/activate && pip install -e ".[dev]" && python -c "import pybench; print('ok')"`
Expected: `ok`

**Step 4: Commit**

```bash
git add pyproject.toml src/ tests/
git commit -m "scaffold: project structure with pyproject.toml and src layout"
```

---

### Task 2: BenchmarkResult Data Model

**Files:**
- Create: `src/pybench/results.py`
- Create: `tests/test_results.py`

**Step 1: Write the failing test**

Create `tests/test_results.py`:
```python
from pybench.results import BenchmarkResult


def test_from_times_computes_statistics():
    times_ns = [100_000, 200_000, 300_000, 400_000, 500_000]
    result = BenchmarkResult.from_times("test_bench", times_ns)

    assert result.name == "test_bench"
    assert result.iterations == 5
    assert result.times_ns == times_ns
    assert result.min_ns == 100_000
    assert result.max_ns == 500_000
    assert result.mean_ns == 300_000.0
    assert result.median_ns == 300_000.0
    assert result.ops_per_sec > 0


def test_from_times_stddev():
    # All same value => stddev 0
    times_ns = [100_000, 100_000, 100_000]
    result = BenchmarkResult.from_times("steady", times_ns)
    assert result.stddev_ns == 0.0


def test_from_times_single_sample():
    times_ns = [500_000]
    result = BenchmarkResult.from_times("single", times_ns)
    assert result.mean_ns == 500_000.0
    assert result.median_ns == 500_000.0
    assert result.stddev_ns == 0.0


def test_to_dict_roundtrip():
    times_ns = [100_000, 200_000, 300_000]
    result = BenchmarkResult.from_times("roundtrip", times_ns)
    d = result.to_dict()

    assert d["name"] == "roundtrip"
    assert d["iterations"] == 3
    assert d["mean_ns"] == result.mean_ns
    assert d["median_ns"] == result.median_ns
    assert d["stddev_ns"] == result.stddev_ns
    assert d["min_ns"] == 100_000
    assert d["max_ns"] == 300_000
    assert "ops_per_sec" in d
```

**Step 2: Run test to verify it fails**

Run: `cd /home/joseph/Projects/pybench && .venv/bin/pytest tests/test_results.py -v`
Expected: FAIL with `ModuleNotFoundError` or `ImportError`

**Step 3: Write minimal implementation**

Create `src/pybench/results.py`:
```python
from __future__ import annotations

import statistics
from dataclasses import dataclass


@dataclass(frozen=True, slots=True)
class BenchmarkResult:
    name: str
    times_ns: list[int]
    iterations: int
    mean_ns: float
    median_ns: float
    stddev_ns: float
    min_ns: int
    max_ns: int
    ops_per_sec: float

    @classmethod
    def from_times(cls, name: str, times_ns: list[int]) -> BenchmarkResult:
        n = len(times_ns)
        mean = statistics.mean(times_ns)
        median = statistics.median(times_ns)
        stddev = statistics.stdev(times_ns) if n > 1 else 0.0
        ops = 1_000_000_000 / mean if mean > 0 else float("inf")

        return cls(
            name=name,
            times_ns=times_ns,
            iterations=n,
            mean_ns=mean,
            median_ns=median,
            stddev_ns=stddev,
            min_ns=min(times_ns),
            max_ns=max(times_ns),
            ops_per_sec=ops,
        )

    def to_dict(self) -> dict:
        return {
            "name": self.name,
            "iterations": self.iterations,
            "mean_ns": self.mean_ns,
            "median_ns": self.median_ns,
            "stddev_ns": self.stddev_ns,
            "min_ns": self.min_ns,
            "max_ns": self.max_ns,
            "ops_per_sec": self.ops_per_sec,
        }
```

**Step 4: Run test to verify it passes**

Run: `cd /home/joseph/Projects/pybench && .venv/bin/pytest tests/test_results.py -v`
Expected: All 4 tests PASS

**Step 5: Commit**

```bash
git add src/pybench/results.py tests/test_results.py
git commit -m "feat: add BenchmarkResult dataclass with statistics computation"
```

---

### Task 3: Runner (Execution Engine)

**Files:**
- Create: `src/pybench/runner.py`
- Create: `tests/test_runner.py`

**Step 1: Write the failing tests**

Create `tests/test_runner.py`:
```python
from pybench.runner import Runner
from pybench.results import BenchmarkResult


def test_run_function_returns_result():
    def noop():
        pass

    runner = Runner(warmup=1, iterations=5)
    result = runner.run("noop", noop)

    assert isinstance(result, BenchmarkResult)
    assert result.name == "noop"
    assert result.iterations == 5
    assert len(result.times_ns) == 5
    assert all(t > 0 for t in result.times_ns)


def test_run_function_with_warmup():
    call_count = 0

    def counting():
        nonlocal call_count
        call_count += 1

    runner = Runner(warmup=3, iterations=2)
    runner.run("counting", counting)

    # 3 warmup + 2 measured = 5 total calls
    assert call_count == 5


def test_auto_calibrate():
    runner = Runner(warmup=0, iterations=None, target_time_ns=10_000_000)

    def fast_fn():
        sum(range(10))

    result = runner.run("fast", fast_fn)
    # auto-calibration should pick more than 1 iteration
    assert result.iterations >= 1
    assert len(result.times_ns) == result.iterations


def test_run_respects_gc_disable():
    """GC should be disabled during measurement but restored after."""
    import gc

    gc_was_enabled = gc.isenabled()
    runner = Runner(warmup=0, iterations=3)

    gc_states_during = []

    def check_gc():
        gc_states_during.append(gc.isenabled())

    runner.run("gc_check", check_gc)

    # GC should be disabled during runs
    assert not any(gc_states_during)
    # GC should be restored to original state after
    assert gc.isenabled() == gc_was_enabled
```

**Step 2: Run tests to verify they fail**

Run: `cd /home/joseph/Projects/pybench && .venv/bin/pytest tests/test_runner.py -v`
Expected: FAIL with `ImportError`

**Step 3: Write minimal implementation**

Create `src/pybench/runner.py`:
```python
from __future__ import annotations

import gc
import time

from pybench.results import BenchmarkResult

_DEFAULT_TARGET_NS = 1_000_000_000  # 1 second


class Runner:
    def __init__(
        self,
        warmup: int = 5,
        iterations: int | None = None,
        target_time_ns: int = _DEFAULT_TARGET_NS,
    ):
        self.warmup = warmup
        self.iterations = iterations
        self.target_time_ns = target_time_ns

    def _calibrate(self, fn: callable) -> int:
        count = 1
        while True:
            start = time.perf_counter_ns()
            for _ in range(count):
                fn()
            elapsed = time.perf_counter_ns() - start
            if elapsed >= self.target_time_ns:
                return max(count, 1)
            count *= 2

    def run(self, name: str, fn: callable) -> BenchmarkResult:
        # Warmup
        for _ in range(self.warmup):
            fn()

        iterations = self.iterations if self.iterations is not None else self._calibrate(fn)

        gc_was_enabled = gc.isenabled()
        gc.disable()
        try:
            times: list[int] = []
            for _ in range(iterations):
                start = time.perf_counter_ns()
                fn()
                elapsed = time.perf_counter_ns() - start
                times.append(elapsed)
        finally:
            if gc_was_enabled:
                gc.enable()

        return BenchmarkResult.from_times(name, times)
```

**Step 4: Run tests to verify they pass**

Run: `cd /home/joseph/Projects/pybench && .venv/bin/pytest tests/test_runner.py -v`
Expected: All 4 tests PASS

**Step 5: Commit**

```bash
git add src/pybench/runner.py tests/test_runner.py
git commit -m "feat: add Runner with warmup, iteration control, and auto-calibration"
```

---

### Task 4: Bench Class (Decorator + Context Manager)

**Files:**
- Create: `src/pybench/bench.py`
- Create: `tests/test_bench.py`

**Step 1: Write the failing tests**

Create `tests/test_bench.py`:
```python
from pybench.bench import Bench
from pybench.results import BenchmarkResult


def test_decorator_registers_function():
    bench = Bench()

    @bench.benchmark
    def my_fn():
        sum(range(100))

    assert len(bench._registered) == 1
    assert bench._registered[0][0] == "my_fn"


def test_decorator_with_options():
    bench = Bench()

    @bench.benchmark(warmup=2, iterations=10)
    def my_fn():
        sum(range(100))

    assert len(bench._registered) == 1
    name, fn, opts = bench._registered[0]
    assert opts["warmup"] == 2
    assert opts["iterations"] == 10


def test_context_manager_records_timing():
    bench = Bench(warmup=0, iterations=1)

    with bench.measure("inline"):
        sum(range(100))

    assert len(bench._results) == 1
    assert bench._results[0].name == "inline"


def test_run_executes_registered_benchmarks():
    bench = Bench(warmup=0, iterations=3)

    @bench.benchmark
    def add_numbers():
        1 + 1

    results = bench.run()

    assert len(results) == 1
    assert results[0].name == "add_numbers"
    assert results[0].iterations == 3


def test_run_includes_context_manager_results():
    bench = Bench(warmup=0, iterations=3)

    with bench.measure("ctx"):
        1 + 1

    @bench.benchmark
    def deco():
        1 + 1

    results = bench.run()

    names = [r.name for r in results]
    assert "ctx" in names
    assert "deco" in names


def test_module_level_decorator():
    """Test the module-level benchmark decorator that uses a global registry."""
    from pybench.bench import benchmark

    @benchmark
    def standalone():
        sum(range(10))

    # Should be callable — decorator doesn't break the function
    standalone()
```

**Step 2: Run tests to verify they fail**

Run: `cd /home/joseph/Projects/pybench && .venv/bin/pytest tests/test_bench.py -v`
Expected: FAIL with `ImportError`

**Step 3: Write minimal implementation**

Create `src/pybench/bench.py`:
```python
from __future__ import annotations

import functools
from contextlib import contextmanager
from typing import Any

from pybench.results import BenchmarkResult
from pybench.runner import Runner

# Global registry for module-level @benchmark decorator
_global_registry: list[tuple[str, callable, dict]] = []


class Bench:
    def __init__(
        self,
        warmup: int = 5,
        iterations: int | None = None,
        target_time_ns: int = 1_000_000_000,
    ):
        self._warmup = warmup
        self._iterations = iterations
        self._target_time_ns = target_time_ns
        self._registered: list[tuple[str, callable, dict[str, Any]]] = []
        self._results: list[BenchmarkResult] = []

    def benchmark(self, fn=None, /, *, warmup=None, iterations=None):
        """Decorator to register a benchmark function.

        Can be used as @bench.benchmark or @bench.benchmark(warmup=2, iterations=10).
        """
        opts = {}
        if warmup is not None:
            opts["warmup"] = warmup
        if iterations is not None:
            opts["iterations"] = iterations

        if fn is not None:
            # Used as @bench.benchmark (no parentheses)
            self._registered.append((fn.__name__, fn, opts))
            return fn

        # Used as @bench.benchmark(warmup=2, iterations=10)
        def decorator(f):
            self._registered.append((f.__name__, f, opts))
            return f
        return decorator

    @contextmanager
    def measure(self, name: str):
        """Context manager for inline benchmarks."""
        runner = Runner(
            warmup=0,
            iterations=1,
            target_time_ns=self._target_time_ns,
        )
        import time
        start = time.perf_counter_ns()
        yield
        elapsed = time.perf_counter_ns() - start
        result = BenchmarkResult.from_times(name, [elapsed])
        self._results.append(result)

    def run(self) -> list[BenchmarkResult]:
        """Run all registered benchmarks and return results."""
        results = list(self._results)  # include context manager results

        for name, fn, opts in self._registered:
            runner = Runner(
                warmup=opts.get("warmup", self._warmup),
                iterations=opts.get("iterations", self._iterations),
                target_time_ns=self._target_time_ns,
            )
            result = runner.run(name, fn)
            results.append(result)

        self._results = results
        return results


def benchmark(fn=None, /, *, warmup=None, iterations=None):
    """Module-level decorator that registers benchmarks in the global registry."""
    opts = {}
    if warmup is not None:
        opts["warmup"] = warmup
    if iterations is not None:
        opts["iterations"] = iterations

    if fn is not None:
        _global_registry.append((fn.__name__, fn, opts))
        return fn

    def decorator(f):
        _global_registry.append((f.__name__, f, opts))
        return f
    return decorator
```

**Step 4: Run tests to verify they pass**

Run: `cd /home/joseph/Projects/pybench && .venv/bin/pytest tests/test_bench.py -v`
Expected: All 6 tests PASS

**Step 5: Commit**

```bash
git add src/pybench/bench.py tests/test_bench.py
git commit -m "feat: add Bench class with decorator and context manager APIs"
```

---

### Task 5: Reporter (Terminal + JSON Output)

**Files:**
- Create: `src/pybench/reporter.py`
- Create: `tests/test_reporter.py`

**Step 1: Write the failing tests**

Create `tests/test_reporter.py`:
```python
import json
import platform
import sys

from pybench.reporter import format_table, format_json, format_time
from pybench.results import BenchmarkResult


def _make_result(name: str, times_ns: list[int]) -> BenchmarkResult:
    return BenchmarkResult.from_times(name, times_ns)


def test_format_time_nanoseconds():
    assert format_time(500) == "500.0 ns"


def test_format_time_microseconds():
    assert format_time(1_500) == "1.5 µs"


def test_format_time_milliseconds():
    assert format_time(1_500_000) == "1.5 ms"


def test_format_time_seconds():
    assert format_time(1_500_000_000) == "1.50 s"


def test_format_table_contains_header_and_names():
    results = [
        _make_result("fast", [1_000, 1_100, 1_200]),
        _make_result("slow", [100_000, 110_000, 120_000]),
    ]
    output = format_table(results)

    assert "Name" in output
    assert "Mean" in output
    assert "Ops/sec" in output
    assert "fast" in output
    assert "slow" in output


def test_format_json_structure():
    results = [_make_result("bench1", [50_000, 60_000])]
    raw = format_json(results)
    data = json.loads(raw)

    assert "metadata" in data
    assert "results" in data
    assert data["metadata"]["python_version"] == platform.python_version()
    assert data["metadata"]["platform"] == platform.system()
    assert len(data["results"]) == 1
    assert data["results"][0]["name"] == "bench1"


def test_format_json_is_valid_json():
    results = [_make_result("x", [1000])]
    raw = format_json(results)
    # Should not raise
    json.loads(raw)
```

**Step 2: Run tests to verify they fail**

Run: `cd /home/joseph/Projects/pybench && .venv/bin/pytest tests/test_reporter.py -v`
Expected: FAIL with `ImportError`

**Step 3: Write minimal implementation**

Create `src/pybench/reporter.py`:
```python
from __future__ import annotations

import json
import platform
from datetime import datetime, timezone

from pybench.results import BenchmarkResult


def format_time(ns: float) -> str:
    if ns < 1_000:
        return f"{ns:.1f} ns"
    elif ns < 1_000_000:
        return f"{ns / 1_000:.1f} µs"
    elif ns < 1_000_000_000:
        return f"{ns / 1_000_000:.1f} ms"
    else:
        return f"{ns / 1_000_000_000:.2f} s"


def format_table(results: list[BenchmarkResult]) -> str:
    if not results:
        return "No benchmark results."

    headers = ["Name", "Mean", "Median", "StdDev", "Min", "Max", "Ops/sec"]

    rows: list[list[str]] = []
    for r in results:
        rows.append([
            r.name,
            format_time(r.mean_ns),
            format_time(r.median_ns),
            format_time(r.stddev_ns),
            format_time(r.min_ns),
            format_time(r.max_ns),
            f"{r.ops_per_sec:,.0f}",
        ])

    col_widths = [len(h) for h in headers]
    for row in rows:
        for i, cell in enumerate(row):
            col_widths[i] = max(col_widths[i], len(cell))

    def fmt_row(cells: list[str]) -> str:
        parts = []
        for i, cell in enumerate(cells):
            if i == 0:
                parts.append(cell.ljust(col_widths[i]))
            else:
                parts.append(cell.rjust(col_widths[i]))
        return "  ".join(parts)

    sep = "─" * (sum(col_widths) + 2 * (len(headers) - 1))
    lines = [
        "pybench results",
        sep,
        fmt_row(headers),
        sep,
    ]
    for row in rows:
        lines.append(fmt_row(row))
    lines.append(sep)
    return "\n".join(lines)


def format_json(results: list[BenchmarkResult]) -> str:
    data = {
        "metadata": {
            "python_version": platform.python_version(),
            "platform": platform.system(),
            "timestamp": datetime.now(timezone.utc).isoformat(),
        },
        "results": [r.to_dict() for r in results],
    }
    return json.dumps(data, indent=2)
```

**Step 4: Run tests to verify they pass**

Run: `cd /home/joseph/Projects/pybench && .venv/bin/pytest tests/test_reporter.py -v`
Expected: All 7 tests PASS

**Step 5: Commit**

```bash
git add src/pybench/reporter.py tests/test_reporter.py
git commit -m "feat: add reporter with terminal table and JSON output"
```

---

### Task 6: Compare Module

**Files:**
- Create: `src/pybench/compare.py`
- Create: `tests/test_compare.py`

**Step 1: Write the failing tests**

Create `tests/test_compare.py`:
```python
import json

from pybench.compare import compare_results, format_comparison


def _make_json(results: list[dict]) -> str:
    return json.dumps({"metadata": {}, "results": results})


def test_compare_results_matching_benchmarks():
    baseline = _make_json([
        {"name": "sort", "mean_ns": 1000.0, "median_ns": 1000.0, "stddev_ns": 10.0,
         "min_ns": 990, "max_ns": 1010, "iterations": 100, "ops_per_sec": 1_000_000.0},
    ])
    current = _make_json([
        {"name": "sort", "mean_ns": 1200.0, "median_ns": 1200.0, "stddev_ns": 12.0,
         "min_ns": 1190, "max_ns": 1210, "iterations": 100, "ops_per_sec": 833_333.0},
    ])

    diffs = compare_results(baseline, current)

    assert len(diffs) == 1
    assert diffs[0]["name"] == "sort"
    assert diffs[0]["baseline_mean_ns"] == 1000.0
    assert diffs[0]["current_mean_ns"] == 1200.0
    assert diffs[0]["change_pct"] == 20.0  # 20% slower


def test_compare_results_improvement():
    baseline = _make_json([
        {"name": "hash", "mean_ns": 2000.0, "median_ns": 2000.0, "stddev_ns": 20.0,
         "min_ns": 1990, "max_ns": 2010, "iterations": 50, "ops_per_sec": 500_000.0},
    ])
    current = _make_json([
        {"name": "hash", "mean_ns": 1000.0, "median_ns": 1000.0, "stddev_ns": 10.0,
         "min_ns": 990, "max_ns": 1010, "iterations": 50, "ops_per_sec": 1_000_000.0},
    ])

    diffs = compare_results(baseline, current)

    assert diffs[0]["change_pct"] == -50.0  # 50% faster


def test_compare_results_missing_in_current():
    baseline = _make_json([
        {"name": "gone", "mean_ns": 1000.0, "median_ns": 1000.0, "stddev_ns": 10.0,
         "min_ns": 990, "max_ns": 1010, "iterations": 50, "ops_per_sec": 1_000_000.0},
    ])
    current = _make_json([])

    diffs = compare_results(baseline, current)

    assert len(diffs) == 1
    assert diffs[0]["name"] == "gone"
    assert diffs[0]["current_mean_ns"] is None
    assert diffs[0]["change_pct"] is None


def test_format_comparison_contains_names():
    baseline = _make_json([
        {"name": "sort", "mean_ns": 1000.0, "median_ns": 1000.0, "stddev_ns": 10.0,
         "min_ns": 990, "max_ns": 1010, "iterations": 100, "ops_per_sec": 1_000_000.0},
    ])
    current = _make_json([
        {"name": "sort", "mean_ns": 1100.0, "median_ns": 1100.0, "stddev_ns": 11.0,
         "min_ns": 1090, "max_ns": 1110, "iterations": 100, "ops_per_sec": 909_091.0},
    ])

    output = format_comparison(baseline, current)
    assert "sort" in output
    assert "%" in output
```

**Step 2: Run tests to verify they fail**

Run: `cd /home/joseph/Projects/pybench && .venv/bin/pytest tests/test_compare.py -v`
Expected: FAIL with `ImportError`

**Step 3: Write minimal implementation**

Create `src/pybench/compare.py`:
```python
from __future__ import annotations

import json

from pybench.reporter import format_time


def compare_results(baseline_json: str, current_json: str) -> list[dict]:
    baseline_data = json.loads(baseline_json)
    current_data = json.loads(current_json)

    current_by_name = {r["name"]: r for r in current_data["results"]}
    diffs = []

    for b in baseline_data["results"]:
        name = b["name"]
        c = current_by_name.pop(name, None)

        if c is None:
            diffs.append({
                "name": name,
                "baseline_mean_ns": b["mean_ns"],
                "current_mean_ns": None,
                "change_pct": None,
            })
        else:
            change = ((c["mean_ns"] - b["mean_ns"]) / b["mean_ns"]) * 100
            diffs.append({
                "name": name,
                "baseline_mean_ns": b["mean_ns"],
                "current_mean_ns": c["mean_ns"],
                "change_pct": round(change, 1),
            })

    # New benchmarks not in baseline
    for name, c in current_by_name.items():
        diffs.append({
            "name": name,
            "baseline_mean_ns": None,
            "current_mean_ns": c["mean_ns"],
            "change_pct": None,
        })

    return diffs


def format_comparison(baseline_json: str, current_json: str) -> str:
    diffs = compare_results(baseline_json, current_json)

    if not diffs:
        return "No benchmarks to compare."

    headers = ["Name", "Baseline", "Current", "Change"]

    rows: list[list[str]] = []
    for d in diffs:
        baseline_str = format_time(d["baseline_mean_ns"]) if d["baseline_mean_ns"] is not None else "N/A"
        current_str = format_time(d["current_mean_ns"]) if d["current_mean_ns"] is not None else "N/A"

        if d["change_pct"] is None:
            change_str = "N/A"
        elif d["change_pct"] > 0:
            change_str = f"+{d['change_pct']:.1f}% (slower)"
        elif d["change_pct"] < 0:
            change_str = f"{d['change_pct']:.1f}% (faster)"
        else:
            change_str = "0.0% (same)"

        rows.append([d["name"], baseline_str, current_str, change_str])

    col_widths = [len(h) for h in headers]
    for row in rows:
        for i, cell in enumerate(row):
            col_widths[i] = max(col_widths[i], len(cell))

    def fmt_row(cells: list[str]) -> str:
        parts = []
        for i, cell in enumerate(cells):
            if i == 0:
                parts.append(cell.ljust(col_widths[i]))
            else:
                parts.append(cell.rjust(col_widths[i]))
        return "  ".join(parts)

    sep = "─" * (sum(col_widths) + 2 * (len(headers) - 1))
    lines = [
        "pybench comparison",
        sep,
        fmt_row(headers),
        sep,
    ]
    for row in rows:
        lines.append(fmt_row(row))
    lines.append(sep)
    return "\n".join(lines)
```

**Step 4: Run tests to verify they pass**

Run: `cd /home/joseph/Projects/pybench && .venv/bin/pytest tests/test_compare.py -v`
Expected: All 4 tests PASS

**Step 5: Commit**

```bash
git add src/pybench/compare.py tests/test_compare.py
git commit -m "feat: add comparison module for diffing benchmark runs"
```

---

### Task 7: CLI

**Files:**
- Create: `src/pybench/cli.py`
- Create: `tests/test_cli.py`

**Step 1: Write the failing tests**

Create `tests/test_cli.py`:
```python
import json
import os
import textwrap
import tempfile
from pathlib import Path
from unittest.mock import patch

from pybench.cli import main


def _create_bench_file(tmp_path: Path, content: str) -> Path:
    f = tmp_path / "bench_example.py"
    f.write_text(content)
    return f


def test_cli_run_discovers_bench_files(tmp_path):
    _create_bench_file(tmp_path, textwrap.dedent("""\
        import pybench

        @pybench.benchmark
        def bench_add():
            1 + 1
    """))

    with patch("sys.argv", ["pybench", "run", str(tmp_path), "--iterations", "3", "--warmup", "0"]):
        # Should not raise
        main()


def test_cli_run_json_output(tmp_path, capsys):
    _create_bench_file(tmp_path, textwrap.dedent("""\
        import pybench

        @pybench.benchmark
        def bench_add():
            1 + 1
    """))

    with patch("sys.argv", ["pybench", "run", str(tmp_path), "--json", "--iterations", "3", "--warmup", "0"]):
        main()

    captured = capsys.readouterr()
    data = json.loads(captured.out)
    assert "results" in data
    assert len(data["results"]) >= 1


def test_cli_run_save_json(tmp_path):
    _create_bench_file(tmp_path, textwrap.dedent("""\
        import pybench

        @pybench.benchmark
        def bench_add():
            1 + 1
    """))

    out_file = tmp_path / "results.json"
    with patch("sys.argv", ["pybench", "run", str(tmp_path), "--save", str(out_file), "--iterations", "3", "--warmup", "0"]):
        main()

    assert out_file.exists()
    data = json.loads(out_file.read_text())
    assert "results" in data


def test_cli_compare(tmp_path, capsys):
    baseline = {"metadata": {}, "results": [
        {"name": "sort", "mean_ns": 1000.0, "median_ns": 1000.0, "stddev_ns": 10.0,
         "min_ns": 990, "max_ns": 1010, "iterations": 100, "ops_per_sec": 1_000_000.0},
    ]}
    current = {"metadata": {}, "results": [
        {"name": "sort", "mean_ns": 1100.0, "median_ns": 1100.0, "stddev_ns": 11.0,
         "min_ns": 1090, "max_ns": 1110, "iterations": 100, "ops_per_sec": 909_091.0},
    ]}

    base_file = tmp_path / "baseline.json"
    curr_file = tmp_path / "current.json"
    base_file.write_text(json.dumps(baseline))
    curr_file.write_text(json.dumps(current))

    with patch("sys.argv", ["pybench", "compare", str(base_file), str(curr_file)]):
        main()

    captured = capsys.readouterr()
    assert "sort" in captured.out
    assert "%" in captured.out
```

**Step 2: Run tests to verify they fail**

Run: `cd /home/joseph/Projects/pybench && .venv/bin/pytest tests/test_cli.py -v`
Expected: FAIL with `ImportError`

**Step 3: Write minimal implementation**

Create `src/pybench/cli.py`:
```python
from __future__ import annotations

import argparse
import importlib.util
import sys
from pathlib import Path

from pybench.bench import Bench, _global_registry
from pybench.compare import format_comparison
from pybench.reporter import format_json, format_table
from pybench.runner import Runner


def _discover_benchmarks(path: Path) -> list[tuple[str, callable, dict]]:
    """Import bench_*.py / *_bench.py files and collect globally registered benchmarks."""
    if path.is_file():
        files = [path]
    else:
        files = sorted(
            list(path.glob("bench_*.py")) + list(path.glob("*_bench.py"))
        )

    _global_registry.clear()

    for f in files:
        spec = importlib.util.spec_from_file_location(f.stem, f)
        if spec and spec.loader:
            mod = importlib.util.module_from_spec(spec)
            sys.modules[f.stem] = mod
            spec.loader.exec_module(mod)

    return list(_global_registry)


def _cmd_run(args: argparse.Namespace) -> None:
    path = Path(args.path)
    benchmarks = _discover_benchmarks(path)

    if not benchmarks:
        print("No benchmarks found.", file=sys.stderr)
        sys.exit(1)

    bench = Bench(
        warmup=args.warmup,
        iterations=args.iterations,
    )

    for name, fn, opts in benchmarks:
        runner = Runner(
            warmup=opts.get("warmup", args.warmup),
            iterations=opts.get("iterations", args.iterations),
        )
        result = runner.run(name, fn)
        bench._results.append(result)

    results = bench._results

    if args.json:
        output = format_json(results)
        print(output)
    else:
        print(format_table(results))

    if args.save:
        Path(args.save).write_text(format_json(results))


def _cmd_compare(args: argparse.Namespace) -> None:
    baseline = Path(args.baseline).read_text()
    current = Path(args.current).read_text()
    print(format_comparison(baseline, current))


def main() -> None:
    parser = argparse.ArgumentParser(prog="pybench", description="Python benchmarking tool")
    subparsers = parser.add_subparsers(dest="command", required=True)

    # run
    run_parser = subparsers.add_parser("run", help="Run benchmarks")
    run_parser.add_argument("path", nargs="?", default=".", help="Path to benchmark file or directory")
    run_parser.add_argument("--json", action="store_true", help="Output JSON instead of table")
    run_parser.add_argument("--save", metavar="FILE", help="Save JSON results to file")
    run_parser.add_argument("--warmup", type=int, default=5, help="Warmup iterations (default: 5)")
    run_parser.add_argument("--iterations", type=int, default=None, help="Measurement iterations (default: auto)")

    # compare
    cmp_parser = subparsers.add_parser("compare", help="Compare two benchmark results")
    cmp_parser.add_argument("baseline", help="Baseline JSON results file")
    cmp_parser.add_argument("current", help="Current JSON results file")

    args = parser.parse_args()

    match args.command:
        case "run":
            _cmd_run(args)
        case "compare":
            _cmd_compare(args)
```

**Step 4: Run tests to verify they pass**

Run: `cd /home/joseph/Projects/pybench && .venv/bin/pytest tests/test_cli.py -v`
Expected: All 4 tests PASS

**Step 5: Commit**

```bash
git add src/pybench/cli.py tests/test_cli.py
git commit -m "feat: add CLI with run and compare subcommands"
```

---

### Task 8: Public API and Bench.report()

**Files:**
- Modify: `src/pybench/__init__.py`
- Modify: `src/pybench/bench.py` — add `report()` and `to_json()` methods
- Create: `tests/test_init.py`

**Step 1: Write the failing tests**

Create `tests/test_init.py`:
```python
def test_public_api_imports():
    from pybench import Bench, BenchmarkResult, benchmark

    assert Bench is not None
    assert BenchmarkResult is not None
    assert callable(benchmark)


def test_bench_report(capsys):
    from pybench import Bench

    bench = Bench(warmup=0, iterations=3)

    @bench.benchmark
    def trivial():
        1 + 1

    bench.run()
    bench.report()

    captured = capsys.readouterr()
    assert "trivial" in captured.out


def test_bench_to_json():
    import json
    from pybench import Bench

    bench = Bench(warmup=0, iterations=3)

    @bench.benchmark
    def trivial():
        1 + 1

    bench.run()
    raw = bench.to_json()
    data = json.loads(raw)
    assert "results" in data
```

**Step 2: Run tests to verify they fail**

Run: `cd /home/joseph/Projects/pybench && .venv/bin/pytest tests/test_init.py -v`
Expected: FAIL with `ImportError`

**Step 3: Implement public API**

Update `src/pybench/__init__.py`:
```python
"""pybench - A lightweight Python benchmarking library."""

from pybench.bench import Bench, benchmark
from pybench.results import BenchmarkResult

__all__ = ["Bench", "BenchmarkResult", "benchmark"]
```

Add `report()` and `to_json()` to the `Bench` class in `src/pybench/bench.py`. Add these two methods at the end of the class body:

```python
    def report(self, json_output: bool = False) -> None:
        """Print benchmark results to stdout."""
        from pybench.reporter import format_json, format_table

        if not self._results:
            self.run()

        if json_output:
            print(format_json(self._results))
        else:
            print(format_table(self._results))

    def to_json(self) -> str:
        """Return benchmark results as a JSON string."""
        from pybench.reporter import format_json

        if not self._results:
            self.run()

        return format_json(self._results)
```

**Step 4: Run tests to verify they pass**

Run: `cd /home/joseph/Projects/pybench && .venv/bin/pytest tests/test_init.py -v`
Expected: All 3 tests PASS

**Step 5: Run full test suite**

Run: `cd /home/joseph/Projects/pybench && .venv/bin/pytest -v`
Expected: All tests PASS

**Step 6: Commit**

```bash
git add src/pybench/__init__.py src/pybench/bench.py tests/test_init.py
git commit -m "feat: add public API re-exports and Bench.report()/to_json() methods"
```

---

### Task 9: Final Integration Test

**Files:**
- Create: `tests/test_integration.py`

**Step 1: Write integration test**

Create `tests/test_integration.py`:
```python
"""End-to-end integration test exercising the full pybench workflow."""

import json


def test_full_workflow():
    from pybench import Bench

    bench = Bench(warmup=1, iterations=10)

    # Decorator API
    @bench.benchmark
    def list_sort():
        sorted([3, 1, 4, 1, 5, 9, 2, 6])

    @bench.benchmark(warmup=0, iterations=5)
    def string_concat():
        "".join(["hello"] * 100)

    # Context manager API
    with bench.measure("dict_creation"):
        {i: i * 2 for i in range(100)}

    # Run all
    results = bench.run()

    assert len(results) == 3
    names = [r.name for r in results]
    assert "list_sort" in names
    assert "string_concat" in names
    assert "dict_creation" in names

    # JSON roundtrip
    raw = bench.to_json()
    data = json.loads(raw)
    assert len(data["results"]) == 3
    assert "metadata" in data
```

**Step 2: Run integration test**

Run: `cd /home/joseph/Projects/pybench && .venv/bin/pytest tests/test_integration.py -v`
Expected: PASS

**Step 3: Run full suite one final time**

Run: `cd /home/joseph/Projects/pybench && .venv/bin/pytest -v`
Expected: All tests PASS

**Step 4: Commit**

```bash
git add tests/test_integration.py
git commit -m "test: add end-to-end integration test"
```
