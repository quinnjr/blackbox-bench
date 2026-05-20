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

# Runner is importable for advanced users (Bench is the supported entry point)
# but not in __all__ — it is not part of the v1.0 stability contract.
__all__ = [
    "Bench",
    "BenchmarkResult",
    "ComparisonReport",
    "DiffRow",
    "HdrHistogram",
    "benchmark",
    "black_box",
    "compare",
]
__version__ = "1.0.0a1"
