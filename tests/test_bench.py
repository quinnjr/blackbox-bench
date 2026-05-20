import pybench


def test_bench_decorator_registers_and_runs():
    bench = pybench.Bench(warmup=1, target_time_ns=20_000_000)

    @bench.benchmark
    def f():
        sum(range(100))

    results = bench.run()
    assert len(results) == 1
    assert results[0].name == "f"


def test_bench_decorator_with_options():
    bench = pybench.Bench(warmup=1, target_time_ns=20_000_000)

    @bench.benchmark(name="custom", iterations=5)
    def g():
        pass

    results = bench.run()
    assert results[0].name == "custom"
    assert results[0].iterations == 5


def test_bench_measure_context_manager():
    bench = pybench.Bench(warmup=0, target_time_ns=10_000_000)
    with bench.measure("section"):
        sum(range(1000))
    results = bench.run()
    names = [r.name for r in results]
    assert "section" in names


def test_module_level_benchmark_decorator():
    pybench._bench._global_registry.clear()

    @pybench.benchmark
    def h():
        pass

    assert any(name == "h" for name, _, _ in pybench._bench._global_registry)
