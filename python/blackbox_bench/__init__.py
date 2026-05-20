"""blackbox_bench — lightweight Python microbenchmarking library."""
from blackbox_bench import _bench as _bench
from blackbox_bench._bench import Bench, benchmark
from blackbox_bench._blackbox_bench import (
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
__version__ = "1.0.0"
