use pyo3::prelude::*;

#[pyfunction]
pub fn black_box<'py>(value: Bound<'py, PyAny>) -> Bound<'py, PyAny> {
    std::hint::black_box(value)
}
