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
