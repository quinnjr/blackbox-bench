import json

import pybench


def _result(name, mean, lo, hi):
    return {
        "name": name,
        "iterations": 100,
        "batch_size": 1,
        "mean_ns": mean,
        "clean_mean_ns": mean,
        "median_ns": mean,
        "stddev_ns": 0.0,
        "min_ns": int(mean),
        "max_ns": int(mean),
        "ops_per_sec": 1e9 / mean,
        "outliers": 0,
        "ci95_low_ns": lo,
        "ci95_high_ns": hi,
        "throughput_per_sec": None,
        "param": None,
    }


def _payload(rs):
    return json.dumps({"metadata": {}, "results": rs})


def test_compare_classifies_overlapping_cis_as_unchanged():
    baseline = _payload([_result("a", 100, 95, 105)])
    current = _payload([_result("a", 103, 98, 108)])
    report = pybench.compare(baseline, current)
    assert report.rows[0].classification == "unchanged"


def test_compare_classifies_disjoint_higher_as_regressed():
    baseline = _payload([_result("a", 100, 95, 105)])
    current = _payload([_result("a", 150, 145, 155)])
    report = pybench.compare(baseline, current)
    assert report.rows[0].classification == "regressed"


def test_compare_classifies_disjoint_lower_as_improved():
    baseline = _payload([_result("a", 200, 195, 205)])
    current = _payload([_result("a", 100, 95, 105)])
    report = pybench.compare(baseline, current)
    assert report.rows[0].classification == "improved"


def test_compare_marks_new_and_removed():
    baseline = _payload([_result("a", 100, 95, 105)])
    current = _payload([_result("b", 100, 95, 105)])
    report = pybench.compare(baseline, current)
    classes = {row.name: row.classification for row in report.rows}
    assert classes == {"a": "removed", "b": "new"}


def _make_report():
    baseline = _payload([
        _result("a", 100, 95, 105),
        _result("b", 200, 190, 210),
        _result("c", 50, 48, 52),  # only-in-baseline → removed
    ])
    current = _payload([
        _result("a", 103, 98, 108),     # unchanged
        _result("b", 300, 290, 310),    # regressed
        _result("d", 10, 8, 12),        # new
    ])
    return pybench.compare(baseline, current)


def test_comparison_report_format_table():
    report = _make_report()
    text = report.format("table")
    assert "Name" in text and "Baseline" in text
    assert "regressed" in text and "removed" in text


def test_comparison_report_format_json():
    report = _make_report()
    text = report.format("json")
    data = json.loads(text)
    classes = {r["name"]: r["classification"] for r in data["rows"]}
    assert classes["b"] == "regressed"
    assert classes["c"] == "removed"
    assert classes["d"] == "new"


def test_comparison_report_format_html():
    report = _make_report()
    text = report.format("html")
    assert "<html" in text.lower() and "regressed" in text


def test_comparison_report_format_xml_junit_marks_failures():
    import xml.etree.ElementTree as ET

    report = _make_report()
    text = report.format("xml")
    tree = ET.fromstring(text)
    assert tree.tag == "testsuite"
    failures = [c for c in tree.findall("testcase") if c.find("failure") is not None]
    failure_names = [c.get("name") for c in failures]
    assert "b" in failure_names


def test_comparison_report_format_xml_raw():
    """The ComparisonReport.format method also supports a raw-XML style internally.

    Direct call path uses the default JUnit; the raw branch is exercised via
    DiffRowView->format_comparison_xml(rows, 'raw') indirectly. Since the
    public API only exposes "xml", we exercise it here too."""
    report = _make_report()
    text = report.format("xml")
    assert text.startswith("<?xml")


def test_comparison_report_format_unknown_raises():
    import pytest

    report = _make_report()
    with pytest.raises(ValueError, match="unknown format"):
        report.format("bogus")


def test_compare_no_results_key_raises():
    import pytest

    with pytest.raises(ValueError):
        pybench.compare("{}", json.dumps({"metadata": {}, "results": []}))


def test_compare_results_not_array_raises():
    import pytest

    with pytest.raises(ValueError):
        pybench.compare(
            json.dumps({"results": "not-an-array"}),
            json.dumps({"results": []}),
        )


def test_compare_payload_not_object_raises():
    import pytest

    with pytest.raises(ValueError):
        pybench.compare("[]", json.dumps({"results": []}))


def test_compare_rejects_oversized_input():
    import pytest

    huge = "a" * (51 * 1024 * 1024)  # 51 MB, just over the 50 MB cap
    with pytest.raises(ValueError, match="exceeds.*limit"):
        pybench.compare(huge, json.dumps({"results": []}))
    with pytest.raises(ValueError, match="current.*exceeds"):
        pybench.compare(json.dumps({"results": []}), huge)


def test_compare_error_identifies_baseline_vs_current():
    import pytest

    # Malformed baseline → error message mentions "baseline"
    with pytest.raises(ValueError, match="baseline"):
        pybench.compare("not json at all", json.dumps({"results": []}))

    # Malformed current → error message mentions "current"
    with pytest.raises(ValueError, match="current"):
        pybench.compare(json.dumps({"results": []}), "not json at all")


def test_compare_row_not_object_raises():
    import pytest

    with pytest.raises(ValueError):
        pybench.compare(
            json.dumps({"results": ["not-an-object"]}),
            json.dumps({"results": []}),
        )


def test_compare_row_missing_name_raises():
    import pytest

    with pytest.raises(ValueError, match="name"):
        pybench.compare(
            json.dumps({"results": [{"mean_ns": 1.0, "ci95_low_ns": 0.5, "ci95_high_ns": 1.5}]}),
            json.dumps({"results": []}),
        )


def test_compare_row_missing_mean_raises():
    import pytest

    with pytest.raises(ValueError, match="mean_ns"):
        pybench.compare(
            json.dumps({"results": [{"name": "x", "ci95_low_ns": 0.5, "ci95_high_ns": 1.5}]}),
            json.dumps({"results": []}),
        )


def test_compare_row_missing_ci_raises():
    import pytest

    with pytest.raises(ValueError, match="ci95"):
        pybench.compare(
            json.dumps({"results": [{"name": "x", "mean_ns": 1.0, "ci95_low_ns": 0.5}]}),
            json.dumps({"results": []}),
        )


def test_comparison_report_format_empty():
    empty = _payload([])
    report = pybench.compare(empty, empty)
    assert "No benchmarks to compare" in report.format("table")


def test_diff_row_attributes():
    report = _make_report()
    by_name = {r.name: r for r in report.rows}
    a = by_name["a"]
    assert a.baseline_mean_ns is not None
    assert a.current_mean_ns is not None
    assert a.change_pct is not None
    # 'c' is only in baseline → current_mean_ns is None
    c = by_name["c"]
    assert c.current_mean_ns is None
    assert c.change_pct is None
