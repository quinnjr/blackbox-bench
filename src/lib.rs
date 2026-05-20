use pyo3::prelude::*;

mod black_box;
mod compare;
pub mod histogram;
pub mod report;
pub mod runner;
pub mod stats;

#[pymodule]
fn _pybench(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(black_box::black_box, m)?)?;
    m.add_class::<runner::Runner>()?;
    m.add_class::<runner::BenchmarkResult>()?;
    m.add_class::<runner::IterBatched>()?;
    m.add_class::<histogram::HdrHistogram>()?;
    m.add_function(wrap_pyfunction!(runner::_synthesize, m)?)?;
    m.add_class::<compare::ComparisonReport>()?;
    m.add_class::<compare::DiffRow>()?;
    m.add_function(wrap_pyfunction!(compare::compare, m)?)?;
    m.add_function(wrap_pyfunction!(report::_format_results_table, m)?)?;
    m.add_function(wrap_pyfunction!(report::_format_results_json, m)?)?;
    m.add_function(wrap_pyfunction!(report::_format_results_html, m)?)?;
    m.add_function(wrap_pyfunction!(report::_format_results_xml, m)?)?;
    Ok(())
}
