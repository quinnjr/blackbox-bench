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
