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


def test_hdrhistogram_direct_construction_and_methods():
    h = pybench.HdrHistogram()
    h.record(1)
    h.record(100)
    h.record(10_000)
    h.record(1_000_000)
    assert h.count() == 4
    assert h.min() >= 1
    assert h.max() <= 1_500_000
    assert h.percentile(50.0) > 0
    assert h.percentile(99.0) >= h.percentile(50.0)


def test_hdrhistogram_to_dict_has_percentiles():
    h = pybench.HdrHistogram()
    for v in [10, 20, 30, 40, 50, 60, 70, 80, 90, 100]:
        h.record(v)
    d = h.to_dict()
    assert d["count"] == 10
    assert "p50" in d and "p99" in d and "p99.9" in d
    assert d["p50"] > 0


def test_hdrhistogram_record_rejects_out_of_range():
    import pytest

    h = pybench.HdrHistogram()
    # 60_000_000_000 is the upper bound; values above it should error
    with pytest.raises(ValueError):
        h.record(10**15)
