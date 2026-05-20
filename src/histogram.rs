use hdrhistogram::Histogram;
use pyo3::prelude::*;

#[pyclass]
pub struct HdrHistogram {
    inner: Histogram<u64>,
}

#[pymethods]
impl HdrHistogram {
    #[new]
    fn new() -> PyResult<Self> {
        Histogram::<u64>::new_with_bounds(1, 60_000_000_000, 3)
            .map(|h| Self { inner: h })
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn record(&mut self, value: u64) -> PyResult<()> {
        self.inner
            .record(value)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn percentile(&self, p: f64) -> u64 {
        self.inner.value_at_percentile(p)
    }

    fn min(&self) -> u64 {
        self.inner.min()
    }

    fn max(&self) -> u64 {
        self.inner.max()
    }

    fn count(&self) -> u64 {
        self.inner.len()
    }

    fn to_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, pyo3::types::PyDict>> {
        let d = pyo3::types::PyDict::new_bound(py);
        d.set_item("min", self.inner.min())?;
        d.set_item("max", self.inner.max())?;
        d.set_item("count", self.inner.len())?;
        for &p in &[50.0_f64, 90.0, 95.0, 99.0, 99.9] {
            d.set_item(format!("p{p}"), self.inner.value_at_percentile(p))?;
        }
        Ok(d)
    }
}

impl HdrHistogram {
    pub fn from_samples(samples: &[i64]) -> Self {
        let mut h = Histogram::<u64>::new_with_bounds(1, 60_000_000_000, 3).unwrap();
        for &s in samples {
            let _ = h.record(s.max(1) as u64);
        }
        Self { inner: h }
    }
}
