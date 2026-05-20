"""pybench — lightweight Python microbenchmarking library."""
from pybench import _bench as _bench
from pybench._bench import Bench, benchmark
from pybench._pybench import BenchmarkResult, Runner, black_box

__all__ = ["Bench", "BenchmarkResult", "Runner", "benchmark", "black_box"]
__version__ = "1.0.0a1"
