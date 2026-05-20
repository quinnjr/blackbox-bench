use pyo3::prelude::*;

mod black_box;
mod histogram;
mod runner;
mod stats;

#[pymodule]
fn _pybench(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(black_box::black_box, m)?)?;
    m.add_class::<runner::Runner>()?;
    m.add_class::<runner::BenchmarkResult>()?;
    m.add_class::<runner::IterBatched>()?;
    m.add_class::<histogram::HdrHistogram>()?;
    m.add_function(wrap_pyfunction!(runner::_synthesize, m)?)?;
    Ok(())
}
