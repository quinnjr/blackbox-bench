"""High-level Bench orchestrator over the Rust Runner."""
from __future__ import annotations

import platform
from contextlib import contextmanager
from datetime import datetime, timezone
from typing import Any, Callable, Iterator

from pybench._pybench import (
    BenchmarkResult,
    IterBatched,
    Runner,
    _format_results_html,
    _format_results_json,
    _format_results_table,
    _format_results_xml,
    _synthesize,
)

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
        # Assign self._results before each loop iteration so partial results
        # survive an exception from a later benchmark; on failure the user
        # can call bench.report() on what completed.
        self._results = list(self._results)
        cached_overhead_ns: float | None = None
        for name, fn, opts in self._registered:
            runner = self._make_runner(opts, cached_overhead_ns)
            if cached_overhead_ns is None:
                cached_overhead_ns = runner.overhead_ns
            throughput = opts.get("throughput")
            if "params" in opts:
                for p in opts["params"]:
                    fn_p = (lambda f=fn, p=p: f(p))
                    self._results.append(runner.run(f"{name}[{p}]", fn_p, throughput, p))
                continue
            ret = fn()
            if isinstance(ret, IterBatched):
                self._results.append(
                    runner.run_iter_batched(name, ret.setup, ret.routine, throughput, None)
                )
            else:
                self._results.append(runner.run(name, fn, throughput, None))
        return self._results

    def to_table(self) -> str:
        if not self._results:
            self.run()
        return _format_results_table(self._results)

    def to_json(self) -> str:
        if not self._results:
            self.run()
        metadata = (
            '{"python_version": "%s", "platform": "%s", "timestamp": "%s", "pybench_version": "1.0.0a1"}'
            % (
                platform.python_version(),
                platform.system(),
                datetime.now(timezone.utc).isoformat(),
            )
        )
        return _format_results_json(self._results, metadata)

    def to_html(self) -> str:
        if not self._results:
            self.run()
        metadata = "%s on %s at %s" % (
            platform.python_version(),
            platform.system(),
            datetime.now(timezone.utc).isoformat(),
        )
        return _format_results_html(self._results, metadata)

    def to_xml(self, style: str = "junit") -> str:
        if not self._results:
            self.run()
        return _format_results_xml(self._results, style)

    def report(self, format: str = "table", path: str | None = None, xml_style: str = "junit") -> None:
        if format == "table":
            text = self.to_table()
        elif format == "json":
            text = self.to_json()
        elif format == "html":
            text = self.to_html()
        elif format == "xml":
            text = self.to_xml(style=xml_style)
        else:
            raise ValueError(f"unknown format: {format}")
        if path:
            with open(path, "w") as f:
                f.write(text)
        else:
            print(text)

    def _make_runner(
        self,
        opts: dict[str, Any] | None = None,
        cached_overhead_ns: float | None = None,
    ) -> Runner:
        opts = opts or {}
        return Runner(
            warmup=opts.get("warmup", self._warmup),
            target_time_ns=self._target_time_ns,
            iterations=opts.get("iterations", self._iterations),
            confidence_level=self._confidence_level,
            outlier_method=self._outlier_method,
            overhead_subtract=self._overhead_subtract,
            seed=self._seed,
            histogram=self._histogram,
            overhead_ns=cached_overhead_ns,
        )


def benchmark(
    fn: Callable[..., Any] | None = None,
    /,
    *,
    name: str | None = None,
    iterations: int | None = None,
    warmup: int | None = None,
    throughput: float | None = None,
    params: list[Any] | None = None,
):
    """Module-level decorator that registers into the global registry.

    Mirrors `Bench.benchmark`'s named kwargs so type checkers can catch
    typos and so the two surfaces stay in sync.
    """
    opts: dict[str, Any] = {}
    if iterations is not None:
        opts["iterations"] = iterations
    if warmup is not None:
        opts["warmup"] = warmup
    if throughput is not None:
        opts["throughput"] = throughput
    if params is not None:
        opts["params"] = params

    def register(f: Callable[..., Any]) -> Callable[..., Any]:
        _global_registry.append((name or f.__name__, f, opts))
        return f

    if fn is not None:
        return register(fn)
    return register
