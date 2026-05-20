//! Reporters: table / JSON / HTML / XML. Comparison and result variants.

use pyo3::prelude::*;

use crate::compare::DiffRowView;
use crate::runner::BenchmarkResult;

fn fmt_time(ns: f64) -> String {
    if ns < 1_000.0 {
        format!("{:.1} ns", ns)
    } else if ns < 1_000_000.0 {
        format!("{:.1} µs", ns / 1_000.0)
    } else if ns < 1_000_000_000.0 {
        format!("{:.1} ms", ns / 1_000_000.0)
    } else {
        format!("{:.2} s", ns / 1_000_000_000.0)
    }
}

fn layout_table(title: &str, headers: &[&str], rows: &[Vec<String>]) -> String {
    let mut widths: Vec<usize> = headers.iter().map(|h| h.len()).collect();
    for row in rows {
        for (i, c) in row.iter().enumerate() {
            widths[i] = widths[i].max(c.len());
        }
    }
    let fmt_row = |cells: &[String]| -> String {
        let mut parts = Vec::with_capacity(cells.len());
        for (i, c) in cells.iter().enumerate() {
            if i == 0 {
                parts.push(format!("{:<w$}", c, w = widths[i]));
            } else {
                parts.push(format!("{:>w$}", c, w = widths[i]));
            }
        }
        parts.join("  ")
    };
    let total: usize = widths.iter().sum::<usize>() + 2 * (widths.len().saturating_sub(1));
    let sep = "─".repeat(total);
    let mut out = format!("{title}\n{sep}\n");
    let header_strs: Vec<String> = headers.iter().map(|s| s.to_string()).collect();
    out.push_str(&fmt_row(&header_strs));
    out.push('\n');
    out.push_str(&sep);
    out.push('\n');
    for row in rows {
        out.push_str(&fmt_row(row));
        out.push('\n');
    }
    out.push_str(&sep);
    out
}

fn json_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

pub fn format_table(results: &[BenchmarkResult]) -> String {
    if results.is_empty() {
        return "No benchmark results.".into();
    }
    let headers = [
        "Name", "Mean", "Median", "StdDev", "Min", "Max", "Ops/sec", "CI 95%", "Outliers",
    ];
    let mut rows: Vec<Vec<String>> = Vec::new();
    for r in results {
        rows.push(vec![
            r.name.clone(),
            fmt_time(r.mean_ns),
            fmt_time(r.median_ns),
            fmt_time(r.stddev_ns),
            fmt_time(r.min_ns as f64),
            fmt_time(r.max_ns as f64),
            format!("{:.0}", r.ops_per_sec),
            format!(
                "[{}, {}]",
                fmt_time(r.ci95_low_ns),
                fmt_time(r.ci95_high_ns)
            ),
            r.outliers.to_string(),
        ]);
    }
    layout_table("pybench results", &headers, &rows)
}

pub fn format_json(results: &[BenchmarkResult], metadata: &str) -> String {
    let mut s = String::with_capacity(1024);
    s.push_str("{\n  \"metadata\": ");
    s.push_str(metadata);
    s.push_str(",\n  \"results\": [\n");
    for (i, r) in results.iter().enumerate() {
        s.push_str("    {");
        s.push_str(&format!("\"name\":{},", json_str(&r.name)));
        s.push_str(&format!("\"iterations\":{},", r.iterations));
        s.push_str(&format!("\"batch_size\":{},", r.batch_size));
        s.push_str(&format!("\"mean_ns\":{},", r.mean_ns));
        s.push_str(&format!("\"clean_mean_ns\":{},", r.clean_mean_ns));
        s.push_str(&format!("\"median_ns\":{},", r.median_ns));
        s.push_str(&format!("\"stddev_ns\":{},", r.stddev_ns));
        s.push_str(&format!("\"min_ns\":{},", r.min_ns));
        s.push_str(&format!("\"max_ns\":{},", r.max_ns));
        s.push_str(&format!("\"ops_per_sec\":{},", r.ops_per_sec));
        s.push_str(&format!("\"outliers\":{},", r.outliers));
        s.push_str(&format!("\"ci95_low_ns\":{},", r.ci95_low_ns));
        s.push_str(&format!("\"ci95_high_ns\":{},", r.ci95_high_ns));
        match r.throughput_per_sec {
            Some(t) => s.push_str(&format!("\"throughput_per_sec\":{},", t)),
            None => s.push_str("\"throughput_per_sec\":null,"),
        }
        s.push_str("\"param\":null}");
        if i + 1 < results.len() {
            s.push(',');
        }
        s.push('\n');
    }
    s.push_str("  ]\n}");
    s
}

pub fn format_html(_results: &[BenchmarkResult], _metadata: &str) -> String {
    String::new()
}

pub fn format_xml(_results: &[BenchmarkResult], _style: &str) -> String {
    String::new()
}

pub fn format_comparison_table(rows: &[DiffRowView]) -> String {
    if rows.is_empty() {
        return "No benchmarks to compare.".into();
    }
    let headers = ["Name", "Baseline", "Current", "Change", "Status"];
    let mut display_rows: Vec<Vec<String>> = Vec::new();
    for d in rows {
        display_rows.push(vec![
            d.name.clone(),
            d.baseline_mean_ns
                .map(fmt_time)
                .unwrap_or_else(|| "N/A".into()),
            d.current_mean_ns
                .map(fmt_time)
                .unwrap_or_else(|| "N/A".into()),
            d.change_pct
                .map(|p| format!("{:+.1}%", p))
                .unwrap_or_else(|| "N/A".into()),
            d.classification.clone(),
        ]);
    }
    layout_table("pybench comparison", &headers, &display_rows)
}

pub fn format_comparison_json(rows: &[DiffRowView]) -> String {
    let mut s = String::from("{\"rows\":[");
    for (i, d) in rows.iter().enumerate() {
        s.push_str("{");
        s.push_str(&format!("\"name\":{},", json_str(&d.name)));
        match d.baseline_mean_ns {
            Some(v) => s.push_str(&format!("\"baseline_mean_ns\":{},", v)),
            None => s.push_str("\"baseline_mean_ns\":null,"),
        }
        match d.current_mean_ns {
            Some(v) => s.push_str(&format!("\"current_mean_ns\":{},", v)),
            None => s.push_str("\"current_mean_ns\":null,"),
        }
        match d.change_pct {
            Some(v) => s.push_str(&format!("\"change_pct\":{},", v)),
            None => s.push_str("\"change_pct\":null,"),
        }
        s.push_str(&format!(
            "\"classification\":{}",
            json_str(&d.classification)
        ));
        s.push('}');
        if i + 1 < rows.len() {
            s.push(',');
        }
    }
    s.push_str("]}");
    s
}

pub fn format_comparison_html(_: &[DiffRowView]) -> String {
    String::new()
}

pub fn format_comparison_xml(_: &[DiffRowView], _: &str) -> String {
    String::new()
}

// --- PyO3 helpers exported to Python -----------------------------------------

#[pyfunction]
pub fn _format_results_table(results: Vec<PyRef<BenchmarkResult>>) -> String {
    let refs: Vec<&BenchmarkResult> = results.iter().map(|r| &**r).collect();
    format_table_refs(&refs)
}

#[pyfunction]
pub fn _format_results_json(results: Vec<PyRef<BenchmarkResult>>, metadata: &str) -> String {
    let refs: Vec<&BenchmarkResult> = results.iter().map(|r| &**r).collect();
    format_json_refs(&refs, metadata)
}

// Helpers that take &[&BenchmarkResult] (the form PyRef gives us). The
// non-ref variants above still exist for the unit-test path that has
// owned BenchmarkResult values directly.

pub fn format_table_refs(results: &[&BenchmarkResult]) -> String {
    if results.is_empty() {
        return "No benchmark results.".into();
    }
    let headers = [
        "Name", "Mean", "Median", "StdDev", "Min", "Max", "Ops/sec", "CI 95%", "Outliers",
    ];
    let mut rows: Vec<Vec<String>> = Vec::new();
    for r in results {
        rows.push(vec![
            r.name.clone(),
            fmt_time(r.mean_ns),
            fmt_time(r.median_ns),
            fmt_time(r.stddev_ns),
            fmt_time(r.min_ns as f64),
            fmt_time(r.max_ns as f64),
            format!("{:.0}", r.ops_per_sec),
            format!(
                "[{}, {}]",
                fmt_time(r.ci95_low_ns),
                fmt_time(r.ci95_high_ns)
            ),
            r.outliers.to_string(),
        ]);
    }
    layout_table("pybench results", &headers, &rows)
}

pub fn format_json_refs(results: &[&BenchmarkResult], metadata: &str) -> String {
    let mut s = String::with_capacity(1024);
    s.push_str("{\n  \"metadata\": ");
    s.push_str(metadata);
    s.push_str(",\n  \"results\": [\n");
    for (i, r) in results.iter().enumerate() {
        s.push_str("    {");
        s.push_str(&format!("\"name\":{},", json_str(&r.name)));
        s.push_str(&format!("\"iterations\":{},", r.iterations));
        s.push_str(&format!("\"batch_size\":{},", r.batch_size));
        s.push_str(&format!("\"mean_ns\":{},", r.mean_ns));
        s.push_str(&format!("\"clean_mean_ns\":{},", r.clean_mean_ns));
        s.push_str(&format!("\"median_ns\":{},", r.median_ns));
        s.push_str(&format!("\"stddev_ns\":{},", r.stddev_ns));
        s.push_str(&format!("\"min_ns\":{},", r.min_ns));
        s.push_str(&format!("\"max_ns\":{},", r.max_ns));
        s.push_str(&format!("\"ops_per_sec\":{},", r.ops_per_sec));
        s.push_str(&format!("\"outliers\":{},", r.outliers));
        s.push_str(&format!("\"ci95_low_ns\":{},", r.ci95_low_ns));
        s.push_str(&format!("\"ci95_high_ns\":{},", r.ci95_high_ns));
        match r.throughput_per_sec {
            Some(t) => s.push_str(&format!("\"throughput_per_sec\":{},", t)),
            None => s.push_str("\"throughput_per_sec\":null,"),
        }
        s.push_str("\"param\":null}");
        if i + 1 < results.len() {
            s.push(',');
        }
        s.push('\n');
    }
    s.push_str("  ]\n}");
    s
}
