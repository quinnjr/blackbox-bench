"""pybench benchmarking pybench.

Run with:
    pybench run benches/bench_self.py --warmup 5 --iterations 200

This file uses the public pybench API (the `@pybench.benchmark` decorator)
to time pybench's own primitives. Useful as a sanity check that the
harness reports sane numbers for the operations it exposes to users, and
as a tracker for perf regressions in CI alongside benches/bench_dogfood.py.
"""
from __future__ import annotations

import json

import pybench
# IterBatched is the underlying pyclass used by Bench.iter_batched(). At
# module-level (without a Bench instance) we construct it directly so the
# Runner sees the same setup/routine sentinel it does from the decorator.
from pybench._pybench import IterBatched

# ---------- Reusable inputs (built once at import) ----------

_SAMPLES = list(range(1_000))

_RESULT_JSON = json.dumps(
    {
        "metadata": {},
        "results": [
            {
                "name": f"bench_{i}",
                "iterations": 100,
                "batch_size": 1,
                "mean_ns": 1_000.0 + i,
                "clean_mean_ns": 1_000.0 + i,
                "median_ns": 1_000.0 + i,
                "stddev_ns": 10.0,
                "min_ns": int(1_000 + i),
                "max_ns": int(1_100 + i),
                "ops_per_sec": 1_000_000.0,
                "outliers": 0,
                "ci95_low_ns": 995.0 + i,
                "ci95_high_ns": 1_005.0 + i,
                "throughput_per_sec": None,
                "param": None,
            }
            for i in range(10)
        ],
    }
)

_HISTOGRAM_FILLED = pybench.HdrHistogram()
for _v in _SAMPLES:
    _HISTOGRAM_FILLED.record(_v + 1)


# ---------- black_box ----------

@pybench.benchmark
def black_box_int():
    pybench.black_box(42)


@pybench.benchmark
def black_box_list():
    pybench.black_box(_SAMPLES)


# ---------- HdrHistogram ----------

@pybench.benchmark
def histogram_new():
    pybench.HdrHistogram()


@pybench.benchmark
def histogram_record():
    # setup builds a fresh HdrHistogram per sample (untimed); routine times
    # the single record() call.
    return IterBatched(
        setup=pybench.HdrHistogram,
        routine=lambda h: h.record(1_234),
    )


@pybench.benchmark
def histogram_percentile_p50():
    _HISTOGRAM_FILLED.percentile(50.0)


@pybench.benchmark
def histogram_percentile_p99():
    _HISTOGRAM_FILLED.percentile(99.0)


@pybench.benchmark
def histogram_to_dict():
    _HISTOGRAM_FILLED.to_dict()


# ---------- compare ----------

@pybench.benchmark
def compare_identical_10():
    """Self-compare: every row classifies as `unchanged`."""
    pybench.compare(_RESULT_JSON, _RESULT_JSON)


@pybench.benchmark
def comparison_report_format_table():
    report = pybench.compare(_RESULT_JSON, _RESULT_JSON)
    report.format("table")


@pybench.benchmark
def comparison_report_format_json():
    report = pybench.compare(_RESULT_JSON, _RESULT_JSON)
    report.format("json")


# ---------- Bench.measure context manager (synthesises a 1-sample result) ----------

@pybench.benchmark
def measure_context_manager():
    # A throwaway Bench just for the context manager path; we don't .run() it
    # because that would nest measurement loops.
    b = pybench.Bench(warmup=0, target_time_ns=1, overhead_subtract=False)
    with b.measure("inner"):
        pass


# ---------- Public reporter throughput (uses a fully-built Bench) ----------

def _populated_bench():
    """Build a Bench with 5 synthesised single-sample results — no timing involved."""
    from pybench._pybench import _synthesize

    b = pybench.Bench(warmup=0, target_time_ns=1, overhead_subtract=False)
    for i in range(5):
        b._results.append(_synthesize(f"row_{i}", 1_000 + i))
    return b


@pybench.benchmark
def report_to_table():
    _populated_bench().to_table()


@pybench.benchmark
def report_to_json():
    _populated_bench().to_json()


@pybench.benchmark
def report_to_html():
    _populated_bench().to_html()


@pybench.benchmark
def report_to_xml_junit():
    _populated_bench().to_xml()
