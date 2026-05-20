"""High-level Bench orchestrator over the Rust Runner."""
from __future__ import annotations

from contextlib import contextmanager
from typing import Any, Callable, Iterator

from pybench._pybench import BenchmarkResult, IterBatched, Runner, _synthesize

_global_registry: list[tuple[str, Callable[..., Any], dict[str, Any]]] = []


class Bench:
    def __init__(
        self,
        warmup: int = 5,
        target_time_ns: int = 1_000_000_000,
        iterations: int | None = None,
        confidence_level: float = 0.95,
        outlier_method: str = "tukey",
        overhead_subtract: bool = True,
        histogram: bool = False,
        seed: int | None = None,
    ):
        self._warmup = warmup
        self._target_time_ns = target_time_ns
        self._iterations = iterations
        self._confidence_level = confidence_level
        self._outlier_method = outlier_method
        self._overhead_subtract = overhead_subtract
        self._histogram = histogram
        self._seed = seed
        self._registered: list[tuple[str, Callable[..., Any], dict[str, Any]]] = []
        self._results: list[BenchmarkResult] = []

    def benchmark(
        self,
        fn: Callable[..., Any] | None = None,
        /,
        *,
        name: str | None = None,
        iterations: int | None = None,
        warmup: int | None = None,
        throughput: float | None = None,
        params: list[Any] | None = None,
        setup: Callable[[], Any] | None = None,
    ):
        opts: dict[str, Any] = {}
        if iterations is not None:
            opts["iterations"] = iterations
        if warmup is not None:
            opts["warmup"] = warmup
        if throughput is not None:
            opts["throughput"] = throughput
        if params is not None:
            opts["params"] = params
        if setup is not None:
            opts["setup"] = setup

        def register(f: Callable[..., Any]) -> Callable[..., Any]:
            self._registered.append((name or f.__name__, f, opts))
            return f

        if fn is not None:
            return register(fn)
        return register

    @contextmanager
    def measure(self, name: str) -> Iterator[None]:
        import time

        start = time.perf_counter_ns()
        yield
        elapsed = time.perf_counter_ns() - start
        self._results.append(_synthesize(name, elapsed))

    def iter_batched(self, setup: Callable[[], Any], routine: Callable[[Any], Any]) -> IterBatched:
        return IterBatched(setup, routine)

    def run(self) -> list[BenchmarkResult]:
        results = list(self._results)
        for name, fn, opts in self._registered:
            runner = self._make_runner(opts)
            throughput = opts.get("throughput")
            probe = fn()
            if isinstance(probe, IterBatched):
                results.append(
                    runner.run_iter_batched(name, probe.setup, probe.routine, throughput, None)
                )
            else:
                results.append(runner.run(name, fn, throughput, None))
        self._results = results
        return results

    def _make_runner(self, opts: dict[str, Any] | None = None) -> Runner:
        opts = opts or {}
        return Runner(
            warmup=opts.get("warmup", self._warmup),
            target_time_ns=self._target_time_ns,
            iterations=opts.get("iterations", self._iterations),
            confidence_level=self._confidence_level,
            outlier_method=self._outlier_method,
            overhead_subtract=self._overhead_subtract,
            seed=self._seed,
        )


def benchmark(
    fn: Callable[..., Any] | None = None,
    /,
    **opts: Any,
):
    """Module-level decorator that registers into the global registry."""
    def register(f: Callable[..., Any]) -> Callable[..., Any]:
        _global_registry.append((opts.get("name") or f.__name__, f, opts))
        return f

    if fn is not None:
        return register(fn)
    return register
