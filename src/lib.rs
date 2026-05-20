use pyo3::prelude::*;

mod black_box;

#[pymodule]
fn _pybench(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(black_box::black_box, m)?)?;
    Ok(())
}
