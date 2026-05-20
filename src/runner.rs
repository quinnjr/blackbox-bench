use std::time::Instant;

use pyo3::prelude::*;

use crate::stats::{self, OutlierMethod};

const MIN_BATCH_TIME_NS: u128 = 5_000; // 5µs per batch minimum
const DEFAULT_TARGET_TIME_NS: u64 = 1_000_000_000; // 1s budget
const BOOTSTRAP_RESAMPLES: usize = 10_000;

#[pyclass(frozen)]
pub struct BenchmarkResult {
    #[pyo3(get)]
    pub name: String,
    #[pyo3(get)]
    pub times_ns: Vec<i64>,
    #[pyo3(get)]
    pub iterations: usize,
    #[pyo3(get)]
    pub batch_size: usize,
    #[pyo3(get)]
    pub mean_ns: f64,
    #[pyo3(get)]
    pub clean_mean_ns: f64,
    #[pyo3(get)]
    pub median_ns: f64,
    #[pyo3(get)]
    pub stddev_ns: f64,
    #[pyo3(get)]
    pub min_ns: i64,
    #[pyo3(get)]
    pub max_ns: i64,
    #[pyo3(get)]
    pub ops_per_sec: f64,
    #[pyo3(get)]
    pub outliers: usize,
    #[pyo3(get)]
    pub ci95_low_ns: f64,
    #[pyo3(get)]
    pub ci95_high_ns: f64,
    #[pyo3(get)]
    pub throughput_per_sec: Option<f64>,
    #[pyo3(get)]
    pub param: Option<PyObject>,
}

#[pymethods]
impl BenchmarkResult {
    fn to_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, pyo3::types::PyDict>> {
        let d = pyo3::types::PyDict::new_bound(py);
        d.set_item("name", &self.name)?;
        d.set_item("iterations", self.iterations)?;
        d.set_item("batch_size", self.batch_size)?;
        d.set_item("mean_ns", self.mean_ns)?;
        d.set_item("clean_mean_ns", self.clean_mean_ns)?;
        d.set_item("median_ns", self.median_ns)?;
        d.set_item("stddev_ns", self.stddev_ns)?;
        d.set_item("min_ns", self.min_ns)?;
        d.set_item("max_ns", self.max_ns)?;
        d.set_item("ops_per_sec", self.ops_per_sec)?;
        d.set_item("outliers", self.outliers)?;
        d.set_item("ci95_low_ns", self.ci95_low_ns)?;
        d.set_item("ci95_high_ns", self.ci95_high_ns)?;
        d.set_item("throughput_per_sec", self.throughput_per_sec)?;
        d.set_item("param", self.param.as_ref())?;
        Ok(d)
    }

    fn __repr__(&self) -> String {
        format!(
            "BenchmarkResult(name={:?}, iterations={}, mean_ns={:.2}, ci95=[{:.2}, {:.2}])",
            self.name, self.iterations, self.mean_ns, self.ci95_low_ns, self.ci95_high_ns,
        )
    }
}

impl BenchmarkResult {
    #[allow(clippy::too_many_arguments)]
    pub fn from_times(
        name: String,
        times_ns: Vec<i64>,
        batch_size: usize,
        confidence_level: f64,
        outlier_method: OutlierMethod,
        throughput: Option<f64>,
        param: Option<PyObject>,
        rng: &mut fastrand::Rng,
    ) -> Self {
        let iterations = times_ns.len();
        if iterations == 0 {
            return Self {
                name,
                times_ns,
                iterations: 0,
                batch_size,
                mean_ns: 0.0,
                clean_mean_ns: 0.0,
                median_ns: 0.0,
                stddev_ns: 0.0,
                min_ns: 0,
                max_ns: 0,
                ops_per_sec: 0.0,
                outliers: 0,
                ci95_low_ns: 0.0,
                ci95_high_ns: 0.0,
                throughput_per_sec: throughput,
                param,
            };
        }
        let mean_ns = stats::mean(&times_ns);
        let median_ns = stats::median(&times_ns);
        let stddev_ns = stats::stddev(&times_ns);
        let min_ns = *times_ns.iter().min().unwrap();
        let max_ns = *times_ns.iter().max().unwrap();
        let ops_per_sec = if mean_ns > 0.0 {
            1_000_000_000.0 / mean_ns
        } else {
            f64::INFINITY
        };
        let (clean_mean_ns, outliers) = stats::detect_outliers(&times_ns, outlier_method);
        let (ci95_low_ns, ci95_high_ns) =
            stats::bootstrap_ci_mean(&times_ns, confidence_level, BOOTSTRAP_RESAMPLES, rng);
        let throughput_per_sec = throughput.map(|bytes| bytes * ops_per_sec);
        Self {
            name,
            times_ns,
            iterations,
            batch_size,
            mean_ns,
            clean_mean_ns,
            median_ns,
            stddev_ns,
            min_ns,
            max_ns,
            ops_per_sec,
            outliers,
            ci95_low_ns,
            ci95_high_ns,
            throughput_per_sec,
            param,
        }
    }
}

#[pyclass]
pub struct Runner {
    warmup: usize,
    target_time_ns: u128,
    iterations: Option<usize>,
    confidence_level: f64,
    outlier_method: OutlierMethod,
    overhead_ns: f64,
    rng: fastrand::Rng,
}

#[pymethods]
impl Runner {
    #[new]
    #[pyo3(signature = (
        warmup=5,
        target_time_ns=DEFAULT_TARGET_TIME_NS,
        iterations=None,
        confidence_level=0.95,
        outlier_method="tukey",
        overhead_subtract=true,
        seed=None,
    ))]
    fn new(
        py: Python<'_>,
        warmup: usize,
        target_time_ns: u64,
        iterations: Option<usize>,
        confidence_level: f64,
        outlier_method: &str,
        overhead_subtract: bool,
        seed: Option<u64>,
    ) -> PyResult<Self> {
        let outlier_method = match outlier_method {
            "tukey" => OutlierMethod::Tukey,
            "mad" => OutlierMethod::Mad,
            "none" => OutlierMethod::None,
            other => {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "outlier_method must be tukey|mad|none, got {other}"
                )));
            }
        };
        let rng = match seed {
            Some(s) => fastrand::Rng::with_seed(s),
            None => fastrand::Rng::new(),
        };
        let overhead_ns = if overhead_subtract {
            measure_overhead(py)?
        } else {
            0.0
        };
        Ok(Self {
            warmup,
            target_time_ns: target_time_ns as u128,
            iterations,
            confidence_level,
            outlier_method,
            overhead_ns,
            rng,
        })
    }

    fn run(
        &mut self,
        py: Python<'_>,
        name: String,
        fn_: PyObject,
    ) -> PyResult<BenchmarkResult> {
        let batch_size = self.calibrate(py, &fn_)?;
        for _ in 0..self.warmup {
            run_batch(py, &fn_, batch_size)?;
        }
        let iters = self.iterations.unwrap_or_else(|| self.estimate_iters(batch_size));
        let mut times = Vec::with_capacity(iters);
        for _ in 0..iters {
            let elapsed = run_batch(py, &fn_, batch_size)?;
            let per_call = (elapsed / batch_size as u128) as i64;
            let adjusted = (per_call as f64 - self.overhead_ns).max(0.0) as i64;
            times.push(adjusted);
        }
        Ok(BenchmarkResult::from_times(
            name,
            times,
            batch_size,
            self.confidence_level,
            self.outlier_method,
            None,
            None,
            &mut self.rng,
        ))
    }
}

impl Runner {
    fn calibrate(&self, py: Python<'_>, fn_: &PyObject) -> PyResult<usize> {
        let mut batch: usize = 1;
        loop {
            let elapsed = run_batch(py, fn_, batch)?;
            if elapsed >= MIN_BATCH_TIME_NS {
                return Ok(batch);
            }
            if batch > (usize::MAX / 2) {
                return Ok(batch);
            }
            batch *= 2;
        }
    }

    fn estimate_iters(&self, _batch_size: usize) -> usize {
        100
    }
}

pub fn run_batch(py: Python<'_>, fn_: &PyObject, batch_size: usize) -> PyResult<u128> {
    let start = Instant::now();
    for _ in 0..batch_size {
        fn_.call0(py)?;
    }
    Ok(start.elapsed().as_nanos())
}

fn measure_overhead(py: Python<'_>) -> PyResult<f64> {
    let noop = py.eval_bound("(lambda: None)", None, None)?;
    let noop_obj: PyObject = noop.into();
    let mut samples = Vec::with_capacity(50);
    for _ in 0..50 {
        let elapsed = run_batch(py, &noop_obj, 1000)?;
        samples.push((elapsed / 1000) as i64);
    }
    Ok(stats::median(&samples))
}
