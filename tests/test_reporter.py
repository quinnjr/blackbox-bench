import json

import pybench


def _bench_with_one_result():
    bench = pybench.Bench(warmup=0, target_time_ns=10_000_000)

    @bench.benchmark
    def x():
        pass

    bench.run()
    return bench


def test_to_table_has_header_and_row():
    bench = _bench_with_one_result()
    out = bench.to_table()
    assert "Name" in out and "Mean" in out and "CI 95%" in out
    assert "x" in out


def test_to_json_round_trips():
    bench = _bench_with_one_result()
    data = json.loads(bench.to_json())
    assert "metadata" in data and "results" in data
    assert data["results"][0]["name"] == "x"
    assert "ci95_low_ns" in data["results"][0]
