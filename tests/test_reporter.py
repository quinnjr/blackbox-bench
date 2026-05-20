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


def test_to_html_self_contained_no_external_refs():
    bench = _bench_with_one_result()
    html = bench.to_html()
    assert "<html" in html.lower() and "</html>" in html.lower()
    assert "x" in html
    assert "ci 95%" in html.lower()
    assert "http://" not in html and "https://" not in html
    assert "cdn." not in html.lower()


def test_to_html_includes_sparkline_svg():
    bench = _bench_with_one_result()
    html = bench.to_html()
    assert "<svg" in html and "<path" in html


def test_to_xml_default_is_junit_compatible():
    import xml.etree.ElementTree as ET

    bench = _bench_with_one_result()
    xml = bench.to_xml()
    tree = ET.fromstring(xml)
    assert tree.tag == "testsuite"
    cases = tree.findall("testcase")
    assert len(cases) == 1
    assert cases[0].get("name") == "x"
    sysout = cases[0].find("system-out")
    assert sysout is not None and sysout.text and "mean_ns" in sysout.text


def test_to_table_empty_results():
    bench = pybench.Bench(warmup=0, target_time_ns=10_000_000)
    # No benchmarks registered — results stays empty.
    out = bench.to_table()
    assert "No benchmark results" in out


def test_fmt_time_units(tmp_path):
    """Benchmark something slow enough to render in larger units."""
    import time

    bench = pybench.Bench(warmup=0, iterations=3, target_time_ns=10_000_000)

    @bench.benchmark
    def slow():
        time.sleep(0.001)  # 1ms

    out = bench.to_table()
    # Should render in ms range
    assert ("µs" in out) or ("ms" in out) or ("s" in out and " ns" not in out.split("\n")[3])


def test_json_str_escapes_in_name():
    """Bench name with characters that exercise json_str's escape paths."""
    bench = pybench.Bench(warmup=0, iterations=2, target_time_ns=5_000_000)

    @bench.benchmark(name='quoted"\\back\nnewline\ttab\x01ctrl')
    def f():
        pass

    raw = bench.to_json()
    # Just check the JSON parses (escaping is correct) and the name is preserved
    data = json.loads(raw)
    assert data["results"][0]["name"] == 'quoted"\\back\nnewline\ttab\x01ctrl'


def test_html_escape_special_chars_in_name():
    bench = pybench.Bench(warmup=0, iterations=2, target_time_ns=5_000_000)

    @bench.benchmark(name='<script>&"\'')
    def f():
        pass

    html = bench.to_html()
    assert "&lt;script&gt;" in html
    assert "&amp;" in html
    assert "&quot;" in html
    assert "&#39;" in html


def test_xml_escape_special_chars_in_name():
    bench = pybench.Bench(warmup=0, iterations=2, target_time_ns=5_000_000)

    @bench.benchmark(name='<x>&"\'')
    def f():
        pass

    xml = bench.to_xml()
    assert "&lt;x&gt;" in xml
    assert "&amp;" in xml
    assert "&quot;" in xml
    assert "&apos;" in xml


def test_json_includes_throughput_when_set():
    # overhead_subtract=False so the tiny benchmark isn't zeroed out (which
    # would yield ops_per_sec=Infinity and a null throughput in JSON).
    bench = pybench.Bench(
        warmup=0, iterations=3, target_time_ns=10_000_000, overhead_subtract=False,
    )

    @bench.benchmark(throughput=1024.0)
    def hashing():
        b"x" * 1024

    data = json.loads(bench.to_json())
    assert data["results"][0]["throughput_per_sec"] is not None
    assert data["results"][0]["throughput_per_sec"] > 0


def test_to_xml_raw_mirrors_json_structure():
    import xml.etree.ElementTree as ET

    bench = _bench_with_one_result()
    xml = bench.to_xml(style="raw")
    tree = ET.fromstring(xml)
    assert tree.tag == "pybench"
    results = tree.find("results")
    assert results is not None
    rows = results.findall("result")
    assert len(rows) == 1
    assert rows[0].get("name") == "x"
