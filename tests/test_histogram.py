import pybench


def test_histogram_populated_when_enabled():
    bench = pybench.Bench(warmup=0, target_time_ns=20_000_000, histogram=True)

    @bench.benchmark
    def f():
        sum(range(10))

    results = bench.run()
    assert results[0].histogram is not None
    h = results[0].histogram
    p50 = h.percentile(50.0)
    p99 = h.percentile(99.0)
    assert p50 > 0
    assert p99 >= p50


def test_histogram_absent_when_disabled():
    bench = pybench.Bench(warmup=0, target_time_ns=10_000_000, histogram=False)

    @bench.benchmark
    def g():
        pass

    results = bench.run()
    assert results[0].histogram is None
