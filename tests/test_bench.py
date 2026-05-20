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


def test_parameterized_benchmark_records_param():
    bench = pybench.Bench(warmup=0, target_time_ns=10_000_000)

    @bench.benchmark(params=[10, 100, 1000])
    def hashing(n):
        b"x" * n

    results = bench.run()
    assert len(results) == 3
    assert [r.param for r in results] == [10, 100, 1000]
    assert [r.name for r in results] == ["hashing[10]", "hashing[100]", "hashing[1000]"]


def test_throughput_recorded_on_result():
    bench = pybench.Bench(warmup=0, target_time_ns=10_000_000)

    @bench.benchmark(throughput=1024.0)
    def hashing():
        b"x" * 1024

    results = bench.run()
    assert results[0].throughput_per_sec is not None
    assert results[0].throughput_per_sec > 0


def test_bench_decorator_per_benchmark_warmup_override():
    bench = pybench.Bench(warmup=0, target_time_ns=10_000_000)

    @bench.benchmark(warmup=3, iterations=2)
    def f():
        pass

    results = bench.run()
    assert results[0].iterations == 2


def test_bench_decorator_parenthesised_no_args():
    bench = pybench.Bench(warmup=0, target_time_ns=10_000_000)

    @bench.benchmark()
    def f():
        pass

    results = bench.run()
    assert results[0].name == "f"


def test_module_level_benchmark_parenthesised_no_args():
    pybench._bench._global_registry.clear()

    @pybench.benchmark()
    def k():
        pass

    assert any(name == "k" for name, _, _ in pybench._bench._global_registry)


def test_to_table_auto_runs_when_not_yet_run():
    bench = pybench.Bench(warmup=0, target_time_ns=10_000_000)

    @bench.benchmark
    def f():
        pass

    out = bench.to_table()
    assert "f" in out


def test_to_json_auto_runs_when_not_yet_run():
    import json as _json

    bench = pybench.Bench(warmup=0, target_time_ns=10_000_000)

    @bench.benchmark
    def f():
        pass

    data = _json.loads(bench.to_json())
    assert data["results"][0]["name"] == "f"


def test_to_html_auto_runs_when_not_yet_run():
    bench = pybench.Bench(warmup=0, target_time_ns=10_000_000)

    @bench.benchmark
    def f():
        pass

    out = bench.to_html()
    assert "<html" in out.lower() and "f" in out


def test_to_xml_auto_runs_when_not_yet_run():
    bench = pybench.Bench(warmup=0, target_time_ns=10_000_000)

    @bench.benchmark
    def f():
        pass

    out = bench.to_xml()
    assert "<testsuite" in out


def test_report_each_format_to_stdout(capsys):
    bench = pybench.Bench(warmup=0, target_time_ns=10_000_000)

    @bench.benchmark
    def f():
        pass

    bench.run()

    for fmt in ("table", "json", "html", "xml"):
        bench.report(format=fmt)
        captured = capsys.readouterr()
        assert captured.out, f"no output for {fmt}"


def test_report_to_file(tmp_path):
    bench = pybench.Bench(warmup=0, target_time_ns=10_000_000)

    @bench.benchmark
    def f():
        pass

    bench.run()
    out = tmp_path / "out.txt"
    bench.report(format="json", path=str(out))
    assert "results" in out.read_text()


def test_report_unknown_format_raises():
    bench = pybench.Bench(warmup=0, target_time_ns=10_000_000)

    @bench.benchmark
    def f():
        pass

    bench.run()
    import pytest

    with pytest.raises(ValueError, match="unknown format"):
        bench.report(format="bogus")


def test_partial_results_survive_benchmark_exception():
    """When a later benchmark raises, the earlier completed results stay on
    self._results so the user can call .report() / .to_json() on what ran."""
    bench = pybench.Bench(warmup=0, iterations=3, target_time_ns=10_000_000)

    @bench.benchmark
    def ok():
        pass

    @bench.benchmark
    def broken():
        raise RuntimeError("boom")

    import pytest

    with pytest.raises(RuntimeError, match="boom"):
        bench.run()

    names = [r.name for r in bench._results]
    assert "ok" in names
    assert "broken" not in names


def test_runner_not_in_public_all():
    """Runner is importable but not part of the stability contract."""
    assert "Runner" not in pybench.__all__
    assert hasattr(pybench, "Runner")  # still importable for advanced users


def test_iter_batched_with_warmup_runs_setup_in_warmup_phase():
    bench = pybench.Bench(warmup=2, iterations=3, target_time_ns=10_000_000)
    setup_calls = [0]

    @bench.benchmark
    def t():
        def setup():
            setup_calls[0] += 1
            return None

        def routine(_):
            pass

        return bench.iter_batched(setup=setup, routine=routine)

    bench.run()
    # 1 calibrate + 2 warmup samples + 3 measurement samples
    assert setup_calls[0] == 1 + 2 + 3


def test_iter_batched_with_histogram_populates_result():
    bench = pybench.Bench(warmup=0, iterations=3, target_time_ns=10_000_000, histogram=True)

    @bench.benchmark
    def t():
        def setup():
            return None

        def routine(_):
            pass

        return bench.iter_batched(setup=setup, routine=routine)

    results = bench.run()
    assert results[0].histogram is not None


def test_iter_batched_setup_runs_per_sample_not_per_call():
    bench = pybench.Bench(warmup=0, iterations=5, target_time_ns=10_000_000)
    setup_calls = [0]
    routine_calls = [0]

    @bench.benchmark
    def sort_random():
        def setup():
            setup_calls[0] += 1
            return [3, 1, 2]

        def routine(xs):
            routine_calls[0] += 1
            sorted(xs)

        return bench.iter_batched(setup=setup, routine=routine)

    results = bench.run()
    n = results[0].iterations
    assert results[0].name == "sort_random"
    # setup runs once during calibration plus once per sample (warmup=0).
    assert setup_calls[0] == 1 + n
    # routine runs at least once per sample; batching typically multiplies it.
    assert routine_calls[0] >= n
