import importlib
import sys
import warnings


def test_legacy_imports_emit_deprecation_warning():
    # Ensure a fresh import for the warning capture
    sys.modules.pop("pybench.legacy", None)
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        legacy = importlib.import_module("pybench.legacy")
        assert any(issubclass(w.category, DeprecationWarning) for w in caught)
    assert hasattr(legacy, "Bench")
    assert hasattr(legacy, "benchmark")
    assert hasattr(legacy, "BenchmarkResult")


def test_legacy_bench_decorator_still_works():
    from pybench import legacy

    bench = legacy.Bench(warmup=1, iterations=3)

    @bench.benchmark
    def f():
        pass

    results = bench.run()
    assert results[0].name == "f"
    assert "mean_ns" in results[0].to_dict()


def test_legacy_report_accepts_json_output(capsys):
    """v0.1.0 callers using `bench.report(json_output=True)` must still work."""
    import json as _json

    from pybench import legacy

    bench = legacy.Bench(warmup=0, iterations=3)

    @bench.benchmark
    def f():
        pass

    bench.run()
    bench.report(json_output=True)
    data = _json.loads(capsys.readouterr().out)
    assert data["results"][0]["name"] == "f"


def test_legacy_report_default_is_table(capsys):
    from pybench import legacy

    bench = legacy.Bench(warmup=0, iterations=3)

    @bench.benchmark
    def g():
        pass

    bench.run()
    bench.report()
    assert "Name" in capsys.readouterr().out
