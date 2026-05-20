use pyo3::prelude::*;

#[pyfunction]
fn _hello() -> &'static str {
    "pybench v1.0"
}

#[pymodule]
fn _pybench(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(_hello, m)?)?;
    Ok(())
}
