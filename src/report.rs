//! Reporters: table / JSON / HTML / XML. Comparison and result variants.

use std::fmt::Write;

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

fn json_num(x: f64) -> String {
    if x.is_finite() {
        format!("{x}")
    } else {
        "null".to_string()
    }
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

fn escape_entities(s: &str, apos: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str(apos),
            c => out.push(c),
        }
    }
    out
}

fn html_escape(s: &str) -> String {
    escape_entities(s, "&#39;")
}

fn sparkline_svg(samples: &[i64]) -> String {
    // In normal use `samples` is non-empty (Runner emits a BenchmarkResult only
    // after at least one sample). Handle the empty case defensively rather than
    // relying on a release-stripped debug_assert.
    if samples.is_empty() {
        return String::new();
    }
    let width: usize = 120;
    let height: usize = 30;
    let bins = 24usize;
    let (mn, mx) = (*samples.iter().min().unwrap(), *samples.iter().max().unwrap());
    if mx == mn {
        return format!(
            "<svg class=\"spark\" width=\"{w}\" height=\"{h}\"><path d=\"M0 {y} L{w} {y}\" stroke=\"#88a\" fill=\"none\"/></svg>",
            w = width,
            h = height,
            y = height / 2
        );
    }
    let mut counts = vec![0usize; bins];
    let range = (mx - mn) as f64;
    for &x in samples {
        let mut b = (((x - mn) as f64 / range) * bins as f64) as usize;
        if b >= bins {
            b = bins - 1;
        }
        counts[b] += 1;
    }
    let cmax = *counts.iter().max().unwrap() as f64;
    let bin_w = width as f64 / bins as f64;
    let mut path = String::with_capacity(16 + bins * 24);
    let _ = write!(path, "M0 {}", height);
    for (i, &c) in counts.iter().enumerate() {
        let h = (c as f64 / cmax) * (height as f64 - 2.0);
        let x = (i as f64) * bin_w;
        let _ = write!(path, " L{:.2} {:.2}", x, height as f64 - h);
        let _ = write!(path, " L{:.2} {:.2}", x + bin_w, height as f64 - h);
    }
    let _ = write!(path, " L{} {} Z", width, height);
    format!(
        "<svg class=\"spark\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\"><path d=\"{p}\" fill=\"#88a\"/></svg>",
        w = width,
        h = height,
        p = path
    )
}

pub fn format_html_refs(results: &[&BenchmarkResult], metadata: &str) -> String {
    let mut s = String::with_capacity(4096);
    s.push_str("<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\">");
    s.push_str("<title>pybench results</title>");
    s.push_str("<style>\n");
    s.push_str("body{font-family:system-ui,-apple-system,sans-serif;margin:2rem;color:#222;}\n");
    s.push_str("table{border-collapse:collapse;width:100%;font-size:0.9rem;}\n");
    s.push_str("th,td{padding:0.4rem 0.6rem;border-bottom:1px solid #ddd;text-align:right;}\n");
    s.push_str("th:first-child,td:first-child{text-align:left;}\n");
    s.push_str("th{background:#f4f4f4;}\n");
    s.push_str(".spark{display:inline-block;vertical-align:middle;}\n");
    s.push_str(".badge{padding:0.1rem 0.5rem;border-radius:0.3rem;font-size:0.8rem;}\n");
    s.push_str(".regressed{background:#fee;color:#900;}\n");
    s.push_str(".improved{background:#efe;color:#070;}\n");
    s.push_str(".unchanged{background:#eef;color:#226;}\n");
    s.push_str("</style></head><body>\n");
    s.push_str("<h1>pybench results</h1>\n");
    let _ = write!(s, "<pre>{}</pre>\n", html_escape(metadata));
    s.push_str("<table><thead><tr>");
    for h in [
        "Name",
        "Mean",
        "Median",
        "StdDev",
        "Min",
        "Max",
        "Ops/sec",
        "CI 95%",
        "Outliers",
        "Distribution",
    ] {
        let _ = write!(s, "<th>{}</th>", html_escape(h));
    }
    s.push_str("</tr></thead><tbody>\n");
    for r in results {
        s.push_str("<tr>");
        let _ = write!(s, "<td>{}</td>", html_escape(&r.name));
        let _ = write!(s, "<td>{}</td>", fmt_time(r.mean_ns));
        let _ = write!(s, "<td>{}</td>", fmt_time(r.median_ns));
        let _ = write!(s, "<td>{}</td>", fmt_time(r.stddev_ns));
        let _ = write!(s, "<td>{}</td>", fmt_time(r.min_ns as f64));
        let _ = write!(s, "<td>{}</td>", fmt_time(r.max_ns as f64));
        let _ = write!(s, "<td>{:.0}</td>", r.ops_per_sec);
        let _ = write!(
            s,
            "<td>[{}, {}]</td>",
            fmt_time(r.ci95_low_ns),
            fmt_time(r.ci95_high_ns)
        );
        let _ = write!(s, "<td>{}</td>", r.outliers);
        let _ = write!(s, "<td>{}</td>", sparkline_svg(&r.times_ns));
        s.push_str("</tr>\n");
    }
    s.push_str("</tbody></table></body></html>\n");
    s
}

fn xml_escape(s: &str) -> String {
    escape_entities(s, "&apos;")
}

/// CDATA sections terminate at `]]>`. Splitting the trigraph as `]]]]><![CDATA[>`
/// preserves the literal text while reopening a fresh CDATA section.
fn cdata_safe(s: &str) -> String {
    s.replace("]]>", "]]]]><![CDATA[>")
}

pub fn format_xml_refs(results: &[&BenchmarkResult], style: &str) -> String {
    match style {
        "raw" => format_xml_raw(results),
        _ => format_xml_junit(results),
    }
}

fn result_to_json_inline(r: &BenchmarkResult) -> String {
    format!(
        "{{\"name\":{name},\"mean_ns\":{m},\"median_ns\":{med},\"ci95_low_ns\":{lo},\"ci95_high_ns\":{hi}}}",
        name = json_str(&r.name),
        m = r.mean_ns,
        med = r.median_ns,
        lo = r.ci95_low_ns,
        hi = r.ci95_high_ns,
    )
}

fn format_xml_junit(results: &[&BenchmarkResult]) -> String {
    let mut s = String::with_capacity(1024);
    s.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    let _ = write!(
        s,
        "<testsuite name=\"pybench\" tests=\"{}\" failures=\"0\">\n",
        results.len(),
    );
    for r in results {
        let _ = write!(
            s,
            "  <testcase name=\"{}\" time=\"{:.9}\">\n",
            xml_escape(&r.name),
            r.mean_ns / 1_000_000_000.0,
        );
        s.push_str("    <system-out><![CDATA[");
        s.push_str(&cdata_safe(&result_to_json_inline(r)));
        s.push_str("]]></system-out>\n");
        s.push_str("  </testcase>\n");
    }
    s.push_str("</testsuite>\n");
    s
}

fn format_xml_raw(results: &[&BenchmarkResult]) -> String {
    let mut s = String::with_capacity(1024);
    s.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<pybench>\n  <results>\n");
    for r in results {
        let _ = write!(
            s,
            "    <result name=\"{}\" iterations=\"{}\" mean_ns=\"{}\" median_ns=\"{}\" \
             ci95_low_ns=\"{}\" ci95_high_ns=\"{}\" outliers=\"{}\"/>\n",
            xml_escape(&r.name),
            r.iterations,
            r.mean_ns,
            r.median_ns,
            r.ci95_low_ns,
            r.ci95_high_ns,
            r.outliers,
        );
    }
    s.push_str("  </results>\n</pybench>\n");
    s
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
        let _ = write!(s, "\"name\":{},", json_str(&d.name));
        match d.baseline_mean_ns {
            Some(v) => {
                let _ = write!(s, "\"baseline_mean_ns\":{},", v);
            }
            None => s.push_str("\"baseline_mean_ns\":null,"),
        }
        match d.current_mean_ns {
            Some(v) => {
                let _ = write!(s, "\"current_mean_ns\":{},", v);
            }
            None => s.push_str("\"current_mean_ns\":null,"),
        }
        match d.change_pct {
            Some(v) => {
                let _ = write!(s, "\"change_pct\":{},", v);
            }
            None => s.push_str("\"change_pct\":null,"),
        }
        let _ = write!(s, "\"classification\":{}", json_str(&d.classification));
        s.push('}');
        if i + 1 < rows.len() {
            s.push(',');
        }
    }
    s.push_str("]}");
    s
}

pub fn format_comparison_html(rows: &[DiffRowView]) -> String {
    let mut s = String::with_capacity(2048);
    s.push_str("<!doctype html><html><head><meta charset=\"utf-8\"><title>pybench comparison</title>");
    s.push_str("<style>");
    s.push_str("body{font-family:system-ui,sans-serif;margin:2rem;}");
    s.push_str("table{border-collapse:collapse;width:100%;}");
    s.push_str("th,td{padding:0.4rem 0.6rem;border-bottom:1px solid #ddd;text-align:right;}");
    s.push_str("th:first-child,td:first-child{text-align:left;}");
    s.push_str(".regressed{color:#900;font-weight:600;}.improved{color:#070;font-weight:600;}.unchanged{color:#446;}");
    s.push_str("</style></head><body><h1>pybench comparison</h1><table><thead><tr>");
    for h in ["Name", "Baseline", "Current", "Change", "Status"] {
        let _ = write!(s, "<th>{}</th>", html_escape(h));
    }
    s.push_str("</tr></thead><tbody>");
    for d in rows {
        s.push_str("<tr>");
        let _ = write!(s, "<td>{}</td>", html_escape(&d.name));
        let _ = write!(
            s,
            "<td>{}</td>",
            d.baseline_mean_ns
                .map(fmt_time)
                .unwrap_or_else(|| "N/A".into())
        );
        let _ = write!(
            s,
            "<td>{}</td>",
            d.current_mean_ns
                .map(fmt_time)
                .unwrap_or_else(|| "N/A".into())
        );
        let _ = write!(
            s,
            "<td>{}</td>",
            d.change_pct
                .map(|p| format!("{:+.1}%", p))
                .unwrap_or_else(|| "N/A".into())
        );
        let _ = write!(s, "<td class=\"{cls}\">{cls}</td>", cls = d.classification);
        s.push_str("</tr>");
    }
    s.push_str("</tbody></table></body></html>");
    s
}

pub fn format_comparison_xml(rows: &[DiffRowView]) -> String {
    // JUnit-style: regressed rows become <failure>. We synthesize the testsuite
    // shell inline rather than threading a Vec<(name, message)> through
    // format_xml_junit, because the comparison case has its own per-row data.
    let mut s = String::with_capacity(1024);
    s.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    let failures: Vec<&DiffRowView> = rows
        .iter()
        .filter(|d| d.classification == "regressed")
        .collect();
    let _ = write!(
        s,
        "<testsuite name=\"pybench\" tests=\"{}\" failures=\"{}\">\n",
        rows.len(),
        failures.len(),
    );
    for d in rows {
        let mean = d.current_mean_ns.unwrap_or(0.0);
        let _ = write!(
            s,
            "  <testcase name=\"{}\" time=\"{:.9}\">\n",
            xml_escape(&d.name),
            mean / 1_000_000_000.0,
        );
        if d.classification == "regressed" {
            let _ = write!(
                s,
                "    <failure message=\"regression: {:+.1}% (CI disjoint from baseline)\"/>\n",
                d.change_pct.unwrap_or(0.0),
            );
        }
        let _ = write!(
            s,
            "    <system-out><![CDATA[{{\"classification\":{}}}]]></system-out>\n",
            json_str(&d.classification),
        );
        s.push_str("  </testcase>\n");
    }
    s.push_str("</testsuite>\n");
    s
}

// --- PyO3 helpers exported to Python -----------------------------------------

fn as_refs<'a>(results: &'a [PyRef<'a, BenchmarkResult>]) -> Vec<&'a BenchmarkResult> {
    results.iter().map(|r| &**r).collect()
}

#[pyfunction]
pub fn _format_results_table(results: Vec<PyRef<BenchmarkResult>>) -> String {
    format_table_refs(&as_refs(&results))
}

#[pyfunction]
pub fn _format_results_json(results: Vec<PyRef<BenchmarkResult>>, metadata: &str) -> String {
    format_json_refs(&as_refs(&results), metadata)
}

#[pyfunction]
pub fn _format_results_html(results: Vec<PyRef<BenchmarkResult>>, metadata: &str) -> String {
    format_html_refs(&as_refs(&results), metadata)
}

#[pyfunction]
pub fn _format_results_xml(results: Vec<PyRef<BenchmarkResult>>, style: &str) -> String {
    format_xml_refs(&as_refs(&results), style)
}

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
        // write! into the pre-allocated String to skip the intermediate
        // String each format!() allocation would produce — ~14 allocs per
        // result avoided.
        let _ = write!(s, "\"name\":{},", json_str(&r.name));
        let _ = write!(s, "\"iterations\":{},", r.iterations);
        let _ = write!(s, "\"batch_size\":{},", r.batch_size);
        let _ = write!(s, "\"mean_ns\":{},", json_num(r.mean_ns));
        let _ = write!(s, "\"clean_mean_ns\":{},", json_num(r.clean_mean_ns));
        let _ = write!(s, "\"median_ns\":{},", json_num(r.median_ns));
        let _ = write!(s, "\"stddev_ns\":{},", json_num(r.stddev_ns));
        let _ = write!(s, "\"min_ns\":{},", r.min_ns);
        let _ = write!(s, "\"max_ns\":{},", r.max_ns);
        let _ = write!(s, "\"ops_per_sec\":{},", json_num(r.ops_per_sec));
        let _ = write!(s, "\"outliers\":{},", r.outliers);
        let _ = write!(s, "\"ci95_low_ns\":{},", json_num(r.ci95_low_ns));
        let _ = write!(s, "\"ci95_high_ns\":{},", json_num(r.ci95_high_ns));
        match r.throughput_per_sec {
            Some(t) => {
                let _ = write!(s, "\"throughput_per_sec\":{},", json_num(t));
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fmt_time_picks_units_by_magnitude() {
        assert!(fmt_time(500.0).contains("ns"));
        assert!(fmt_time(5_000.0).contains("µs"));
        assert!(fmt_time(5_000_000.0).contains("ms"));
        assert!(fmt_time(5_000_000_000.0).contains(" s"));
    }

    #[test]
    fn json_num_finite_is_number_non_finite_is_null() {
        assert_eq!(json_num(1.5), "1.5");
        assert_eq!(json_num(f64::INFINITY), "null");
        assert_eq!(json_num(f64::NEG_INFINITY), "null");
        assert_eq!(json_num(f64::NAN), "null");
    }

    #[test]
    fn json_str_escapes_all_control_chars() {
        assert_eq!(json_str("a\rb"), "\"a\\rb\"");
        assert_eq!(json_str("a\nb"), "\"a\\nb\"");
        assert_eq!(json_str("a\tb"), "\"a\\tb\"");
        assert_eq!(json_str("a\"b"), "\"a\\\"b\"");
        assert_eq!(json_str("a\\b"), "\"a\\\\b\"");
        assert_eq!(json_str("\x01"), "\"\\u0001\"");
    }
}
