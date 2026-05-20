use std::time::Instant;

use pyo3::ffi;
use pyo3::prelude::*;
use pyo3::types::PyTuple;

use crate::histogram::HdrHistogram;
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
    #[pyo3(get)]
    pub histogram: Option<Py<HdrHistogram>>,
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
        py: Python<'_>,
        name: String,
        times_ns: Vec<i64>,
        batch_size: usize,
        confidence_level: f64,
        outlier_method: OutlierMethod,
        throughput: Option<f64>,
        param: Option<PyObject>,
        histogram: Option<Py<HdrHistogram>>,
        rng: &mut fastrand::Rng,
        samples_scratch: &mut Vec<i64>,
        means_scratch: &mut Vec<f64>,
    ) -> Self {
        let iterations = times_ns.len();
        debug_assert!(iterations > 0, "from_times must be called with at least one sample");
        // Compute stats with the GIL released — bootstrap_ci_mean alone runs
        // 10,000 × N inner iterations of pure Rust, blocking any other Python
        // thread until it returns.
        let (mean_ns, median_ns, stddev_ns, min_ns, max_ns, clean_mean_ns, outliers,
             ci95_low_ns, ci95_high_ns) = py.allow_threads(|| {
            let mean_ns = stats::mean(&times_ns);
            let median_ns = stats::median(&times_ns, samples_scratch);
            let stddev_ns = stats::stddev(&times_ns);
            let min_ns = *times_ns.iter().min().unwrap();
            let max_ns = *times_ns.iter().max().unwrap();
            let (clean_mean_ns, outliers) =
                stats::detect_outliers(&times_ns, outlier_method, samples_scratch);
            let (ci95_low_ns, ci95_high_ns) = stats::bootstrap_ci_mean(
                &times_ns,
                confidence_level,
                BOOTSTRAP_RESAMPLES,
                rng,
                means_scratch,
            );
            (mean_ns, median_ns, stddev_ns, min_ns, max_ns,
             clean_mean_ns, outliers, ci95_low_ns, ci95_high_ns)
        });
        let ops_per_sec = if mean_ns > 0.0 {
            1_000_000_000.0 / mean_ns
        } else {
            f64::INFINITY
        };
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
            histogram,
        }
    }
}

#[pyclass]
pub struct IterBatched {
    #[pyo3(get)]
    pub setup: PyObject,
    #[pyo3(get)]
    pub routine: PyObject,
}

#[pymethods]
impl IterBatched {
    #[new]
    fn new(setup: PyObject, routine: PyObject) -> Self {
        Self { setup, routine }
    }
}

#[pyclass]
pub struct Runner {
    warmup: usize,
    target_time_ns: u128,
    iterations: Option<usize>,
    confidence_level: f64,
    outlier_method: OutlierMethod,
    #[pyo3(get)]
    overhead_ns: f64,
    histogram: bool,
    rng: fastrand::Rng,
    /// Scratch buffer for median / Tukey / MAD: reused across benchmarks so
    /// a `Bench.run()` over N benches allocates one Vec instead of N.
    samples_scratch: Vec<i64>,
    /// Scratch buffer for bootstrap_ci_mean (10k f64s = 80 KB).
    means_scratch: Vec<f64>,
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
        histogram=false,
        overhead_ns=None,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        py: Python<'_>,
        warmup: usize,
        target_time_ns: u64,
        iterations: Option<usize>,
        confidence_level: f64,
        outlier_method: &str,
        overhead_subtract: bool,
        seed: Option<u64>,
        histogram: bool,
        overhead_ns: Option<f64>,
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
        let overhead_ns = match overhead_ns {
            Some(v) => v,
            None if overhead_subtract => measure_overhead(py)?,
            None => 0.0,
        };
        Ok(Self {
            warmup,
            target_time_ns: target_time_ns as u128,
            iterations,
            confidence_level,
            outlier_method,
            overhead_ns,
            histogram,
            rng,
            samples_scratch: Vec::new(),
            means_scratch: Vec::with_capacity(BOOTSTRAP_RESAMPLES),
        })
    }

    #[pyo3(signature = (name, fn_, throughput=None, param=None))]
    fn run(
        &mut self,
        py: Python<'_>,
        name: String,
        fn_: PyObject,
        throughput: Option<f64>,
        param: Option<PyObject>,
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
        let histogram = if self.histogram {
            Some(Py::new(py, HdrHistogram::from_samples(&times))?)
        } else {
            None
        };
        let Runner {
            ref mut rng,
            ref mut samples_scratch,
            ref mut means_scratch,
            confidence_level,
            outlier_method,
            ..
        } = *self;
        Ok(BenchmarkResult::from_times(
            py,
            name,
            times,
            batch_size,
            confidence_level,
            outlier_method,
            throughput,
            param,
            histogram,
            rng,
            samples_scratch,
            means_scratch,
        ))
    }

    #[pyo3(signature = (name, setup, routine, throughput=None, param=None))]
    fn run_iter_batched(
        &mut self,
        py: Python<'_>,
        name: String,
        setup: PyObject,
        routine: PyObject,
        throughput: Option<f64>,
        param: Option<PyObject>,
    ) -> PyResult<BenchmarkResult> {
        let batch_size = self.calibrate_batched(py, &setup, &routine)?;
        for _ in 0..self.warmup {
            let state = setup.call0(py)?;
            call_routine_batch(py, &routine, state, batch_size)?;
        }
        let iters = self.iterations.unwrap_or_else(|| self.estimate_iters(batch_size));
        let mut times = Vec::with_capacity(iters);
        for _ in 0..iters {
            let state = setup.call0(py)?;
            let args = PyTuple::new_bound(py, [state]);
            let routine_ptr = routine.as_ptr();
            let args_ptr = args.as_ptr();
            let start = Instant::now();
            for _ in 0..batch_size {
                // SAFETY: routine_ptr and args_ptr remain valid through this scope
                // (the `routine` PyObject and `args` Bound own their references). A
                // returned PyObject must be DECREFed; null indicates a Python exception.
                let result = unsafe { ffi::PyObject_CallObject(routine_ptr, args_ptr) };
                if result.is_null() {
                    return Err(PyErr::fetch(py));
                }
                unsafe { ffi::Py_DECREF(result) };
            }
            let elapsed = start.elapsed().as_nanos();
            let per_call = (elapsed / batch_size as u128) as i64;
            let adjusted = (per_call as f64 - self.overhead_ns).max(0.0) as i64;
            times.push(adjusted);
        }
        let histogram = if self.histogram {
            Some(Py::new(py, HdrHistogram::from_samples(&times))?)
        } else {
            None
        };
        let Runner {
            ref mut rng,
            ref mut samples_scratch,
            ref mut means_scratch,
            confidence_level,
            outlier_method,
            ..
        } = *self;
        Ok(BenchmarkResult::from_times(
            py,
            name,
            times,
            batch_size,
            confidence_level,
            outlier_method,
            throughput,
            param,
            histogram,
            rng,
            samples_scratch,
            means_scratch,
        ))
    }
}

impl Runner {
    fn calibrate(&self, py: Python<'_>, fn_: &PyObject) -> PyResult<usize> {
        // The loop is bounded: a fn that took less than 5µs per call at batch=2^62
        // would have to be physically impossible (sub-attosecond), so we don't
        // guard the multiplication.
        let mut batch: usize = 1;
        loop {
            let elapsed = run_batch(py, fn_, batch)?;
            if elapsed >= MIN_BATCH_TIME_NS {
                return Ok(batch);
            }
            batch *= 2;
        }
    }

    fn estimate_iters(&self, _batch_size: usize) -> usize {
        100
    }

    fn calibrate_batched(
        &self,
        py: Python<'_>,
        setup: &PyObject,
        routine: &PyObject,
    ) -> PyResult<usize> {
        let state = setup.call0(py)?;
        let args = PyTuple::new_bound(py, [state]);
        let mut batch: usize = 1;
        loop {
            let start = Instant::now();
            call_routine_batch_ptr(py, routine.as_ptr(), args.as_ptr(), batch)?;
            let elapsed = start.elapsed().as_nanos();
            if elapsed >= MIN_BATCH_TIME_NS {
                return Ok(batch);
            }
            batch *= 2;
        }
    }
}

/// Invoke `routine(state)` `batch_size` times. Used by warmup and calibration —
/// not the timed measurement path (which inlines the loop to keep `Instant::now()`
/// adjacent to the calls).
fn call_routine_batch(
    py: Python<'_>,
    routine: &PyObject,
    state: PyObject,
    batch_size: usize,
) -> PyResult<()> {
    let args = PyTuple::new_bound(py, [state]);
    call_routine_batch_ptr(py, routine.as_ptr(), args.as_ptr(), batch_size)
}

fn call_routine_batch_ptr(
    py: Python<'_>,
    routine_ptr: *mut ffi::PyObject,
    args_ptr: *mut ffi::PyObject,
    batch_size: usize,
) -> PyResult<()> {
    for _ in 0..batch_size {
        let result = unsafe { ffi::PyObject_CallObject(routine_ptr, args_ptr) };
        if result.is_null() {
            return Err(PyErr::fetch(py));
        }
        unsafe { ffi::Py_DECREF(result) };
    }
    Ok(())
}

#[pyfunction]
pub fn _synthesize(py: Python<'_>, name: String, elapsed_ns: i64) -> BenchmarkResult {
    let mut rng = fastrand::Rng::with_seed(0);
    let mut samples_scratch = Vec::new();
    let mut means_scratch = Vec::new();
    BenchmarkResult::from_times(
        py,
        name,
        vec![elapsed_ns],
        1,
        0.95,
        OutlierMethod::None,
        None,
        None,
        None,
        &mut rng,
        &mut samples_scratch,
        &mut means_scratch,
    )
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
    let mut scratch = Vec::new();
    Ok(stats::median(&samples, &mut scratch))
}
