import blackbox_bench


def test_runner_produces_result_with_required_fields():
    runner = blackbox_bench.Runner(warmup=2, target_time_ns=20_000_000)

    def noop():
        pass

    result = runner.run("noop", noop)
    assert result.name == "noop"
    assert result.iterations > 0
    assert result.batch_size >= 1
    assert result.mean_ns >= 0
    assert result.median_ns >= 0
    assert result.stddev_ns >= 0
    assert result.min_ns >= 0
    assert result.max_ns >= 0
    assert result.ops_per_sec > 0
    assert result.ci95_low_ns <= result.mean_ns <= result.ci95_high_ns
    assert isinstance(result.times_ns, list)
    assert len(result.times_ns) == result.iterations


def test_runner_batch_size_grows_for_fast_functions():
    runner = blackbox_bench.Runner(warmup=1, target_time_ns=20_000_000)

    def trivial():
        pass

    r = runner.run("trivial", trivial)
    assert r.batch_size > 1, "fast function should batch"


def test_runner_overhead_subtract_false_skips_overhead_probe():
    runner = blackbox_bench.Runner(
        warmup=0, iterations=3, target_time_ns=10_000_000,
        overhead_subtract=False, seed=42,
    )

    def noop():
        pass

    r = runner.run("no_overhead", noop)
    assert r.iterations == 3


def test_runner_outlier_method_none():
    runner = blackbox_bench.Runner(
        warmup=0, iterations=10, target_time_ns=10_000_000,
        outlier_method="none",
    )

    def noop():
        pass

    r = runner.run("none_method", noop)
    assert r.outliers == 0
    assert r.clean_mean_ns == r.mean_ns


def test_runner_outlier_method_mad():
    runner = blackbox_bench.Runner(
        warmup=0, iterations=10, target_time_ns=10_000_000,
        outlier_method="mad",
    )

    def noop():
        pass

    r = runner.run("mad_method", noop)
    assert r.iterations == 10


def test_runner_invalid_outlier_method_raises():
    import pytest

    with pytest.raises(ValueError, match="outlier_method"):
        blackbox_bench.Runner(warmup=0, iterations=3, outlier_method="bogus")


def test_runner_iterations_zero_raises():
    import pytest

    with pytest.raises(ValueError, match="iterations"):
        blackbox_bench.Runner(warmup=0, iterations=0)


def test_runner_iterations_none_uses_target_time_budget():
    """iterations=None should auto-estimate samples to fit target_time_ns."""
    # 5ms budget with calibration's 5µs minimum batch => somewhere in [10, ~1000]
    runner = blackbox_bench.Runner(warmup=0, target_time_ns=5_000_000)

    def noop():
        pass

    r = runner.run("auto_iters", noop)
    assert r.iterations >= 10
    assert r.iterations <= 100_000


def test_runner_iter_batched_returns_result():
    """Cover Runner.run_iter_batched directly (vs through Bench.run)."""
    runner = blackbox_bench.Runner(warmup=0, iterations=3, target_time_ns=10_000_000)

    def setup():
        return [3, 1, 2]

    def routine(xs):
        sorted(xs)

    r = runner.run_iter_batched("via_runner", setup, routine, None, None)
    assert r.name == "via_runner"
    assert r.iterations == 3


def test_benchmark_result_repr():
    runner = blackbox_bench.Runner(warmup=0, iterations=3, target_time_ns=10_000_000)

    def noop():
        pass

    r = runner.run("named", noop)
    s = repr(r)
    assert "named" in s and "mean_ns" in s


def test_benchmark_result_to_dict_round_trip():
    runner = blackbox_bench.Runner(warmup=0, iterations=3, target_time_ns=10_000_000)

    def noop():
        pass

    r = runner.run("dict_test", noop)
    d = r.to_dict()
    assert d["name"] == "dict_test"
    assert d["iterations"] == 3
    assert d["param"] is None
    # v0.1.0 to_dict() excluded the (potentially large) times_ns list to keep
    # dict-based serialisation cheap. Lock that in so a future helper accident
    # doesn't quietly start including it.
    assert "times_ns" not in d
    # times_ns is still accessible as an attribute.
    assert isinstance(r.times_ns, list)
    assert len(r.times_ns) == 3


def test_runner_respects_warmup():
    calls = [0]

    def counted():
        calls[0] += 1

    runner = blackbox_bench.Runner(warmup=7, iterations=3, target_time_ns=10_000_000)
    r = runner.run("counted", counted)
    assert r.iterations == 3
    # 7 warmup batches + 3 sample batches of size >= 1 each
    assert calls[0] >= 7 + 3
