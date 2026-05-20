"""End-to-end: register, run, report, save, compare."""
import json
from pathlib import Path

import pybench


def test_full_workflow_run_report_compare(tmp_path):
    bench = pybench.Bench(warmup=1, target_time_ns=30_000_000)

    @bench.benchmark
    def quick():
        sum(range(50))

    @bench.benchmark(name="slow", iterations=10)
    def slow():
        sum(range(1000))

    results = bench.run()
    assert len(results) == 2
    assert {r.name for r in results} == {"quick", "slow"}

    baseline_path = tmp_path / "baseline.json"
    baseline_path.write_text(bench.to_json())
    baseline_data = json.loads(baseline_path.read_text())
    assert "results" in baseline_data
    assert len(baseline_data["results"]) == 2

    bench2 = pybench.Bench(warmup=1, target_time_ns=30_000_000)

    @bench2.benchmark
    def quick():
        sum(range(50))

    @bench2.benchmark(name="slow", iterations=10)
    def slow():
        sum(range(1000))

    bench2.run()
    current_json = bench2.to_json()

    report = pybench.compare(baseline_path.read_text(), current_json)
    assert len(report.rows) == 2
    classes = {row.classification for row in report.rows}
    assert classes <= {"unchanged", "regressed", "improved"}

    table = report.format("table")
    assert "Name" in table


def test_full_workflow_html_xml_outputs(tmp_path):
    bench = pybench.Bench(warmup=0, target_time_ns=10_000_000)

    @bench.benchmark
    def t():
        pass

    bench.run()
    html_path = tmp_path / "out.html"
    xml_path = tmp_path / "out.xml"
    bench.report(format="html", path=str(html_path))
    bench.report(format="xml", path=str(xml_path))
    assert "<html" in html_path.read_text().lower()
    assert "<testsuite" in xml_path.read_text()
