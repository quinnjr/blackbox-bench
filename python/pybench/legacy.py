"""v0.1.0 compatibility shim. Removed in pybench v1.1."""
from __future__ import annotations

import warnings as _w

_w.warn(
    "pybench.legacy is deprecated and will be removed in pybench 1.1. "
    "Migrate to the new pybench v1.0 API (see MIGRATION.md).",
    DeprecationWarning,
    stacklevel=2,
)

from pybench._bench import Bench as _Bench, benchmark
from pybench._pybench import BenchmarkResult


class Bench(_Bench):
    """v0.1.0-shaped Bench: only `warmup` and `iterations` constructor args."""

    def __init__(
        self,
        warmup: int = 5,
        iterations: int | None = None,
        target_time_ns: int = 1_000_000_000,
    ):
        super().__init__(warmup=warmup, iterations=iterations, target_time_ns=target_time_ns)


__all__ = ["Bench", "BenchmarkResult", "benchmark"]
