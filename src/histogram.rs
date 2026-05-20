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
        let d = pyo3::types::PyDict::new(py);
        d.set_item("min", self.inner.min())?;
        d.set_item("max", self.inner.max())?;
        d.set_item("count", self.inner.len())?;
        for &p in &[50.0_f64, 90.0, 95.0, 99.0, 99.9] {
            d.set_item(format!("p{p}"), self.inner.value_at_percentile(p))?;
        }
        Ok(d)
    }
}

const HISTOGRAM_MAX_NS: u64 = 60_000_000_000;

impl HdrHistogram {
    pub fn from_samples(samples: &[i64]) -> Self {
        let mut h = Histogram::<u64>::new_with_bounds(1, HISTOGRAM_MAX_NS, 3).unwrap();
        for &s in samples {
            // Saturate to the histogram's max bound so samples above 60s are
            // bucketed at the ceiling rather than silently dropped. With the
            // clamp `record` can no longer fail for finite inputs.
            let clamped = (s.max(1) as u64).min(HISTOGRAM_MAX_NS);
            h.record(clamped).expect("clamped value is within histogram bounds");
        }
        Self { inner: h }
    }
}
