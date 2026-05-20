"""pybench — lightweight Python microbenchmarking library."""
from pybench import _bench as _bench
from pybench._bench import Bench, benchmark
from pybench._pybench import (
    BenchmarkResult,
    ComparisonReport,
    DiffRow,
    HdrHistogram,
    Runner,
    black_box,
    compare,
)

__all__ = [
    "Bench",
    "BenchmarkResult",
    "ComparisonReport",
    "DiffRow",
    "HdrHistogram",
    "Runner",
    "benchmark",
    "black_box",
    "compare",
]
__version__ = "1.0.0a1"
