import pybench


def test_runner_produces_result_with_required_fields():
    runner = pybench.Runner(warmup=2, target_time_ns=20_000_000)

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
    runner = pybench.Runner(warmup=1, target_time_ns=20_000_000)

    def trivial():
        pass

    r = runner.run("trivial", trivial)
    assert r.batch_size > 1, "fast function should batch"


def test_runner_respects_warmup():
    calls = [0]

    def counted():
        calls[0] += 1

    runner = pybench.Runner(warmup=7, iterations=3, target_time_ns=10_000_000)
    r = runner.run("counted", counted)
    assert r.iterations == 3
    # 7 warmup batches + 3 sample batches of size >= 1 each
    assert calls[0] >= 7 + 3
