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
