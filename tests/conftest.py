"""Shared pytest fixtures for the blackbox_bench test suite."""
import pytest

import blackbox_bench


@pytest.fixture(autouse=True)
def _restore_global_registry():
    """Snapshot blackbox_bench._bench._global_registry around each test so a test that
    appends (or clears) it cannot leak entries — or absences — to a later test
    under pytest-randomly or arbitrary ordering."""
    snapshot = list(blackbox_bench._bench._global_registry)
    yield
    blackbox_bench._bench._global_registry[:] = snapshot


@pytest.fixture
def fast_bench():
    """Factory for a Bench tuned for unit-test latency.

    Usage:
        def test_x(fast_bench):
            bench = fast_bench()                  # defaults: warmup=0, 10ms
            bench = fast_bench(iterations=3)      # override
    """

    def _build(**kwargs) -> blackbox_bench.Bench:
        defaults = {"warmup": 0, "target_time_ns": 10_000_000}
        defaults.update(kwargs)
        return blackbox_bench.Bench(**defaults)

    return _build
