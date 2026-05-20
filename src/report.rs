//! Reporters: table / JSON / HTML / XML. Comparison and result variants.

use crate::compare::DiffRowView;
use crate::runner::BenchmarkResult;

pub fn format_table(_results: &[BenchmarkResult]) -> String {
    String::new()
}

pub fn format_json(_results: &[BenchmarkResult], _metadata: &str) -> String {
    String::new()
}

pub fn format_html(_results: &[BenchmarkResult], _metadata: &str) -> String {
    String::new()
}

pub fn format_xml(_results: &[BenchmarkResult], _style: &str) -> String {
    String::new()
}

pub fn format_comparison_table(_: &[DiffRowView]) -> String {
    String::new()
}

pub fn format_comparison_json(_: &[DiffRowView]) -> String {
    String::new()
}

pub fn format_comparison_html(_: &[DiffRowView]) -> String {
    String::new()
}

pub fn format_comparison_xml(_: &[DiffRowView], _: &str) -> String {
    String::new()
}
