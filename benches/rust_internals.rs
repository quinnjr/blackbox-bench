//! Criterion benchmarks for pybench's internal Rust functions.
//!
//! Run with: `cargo bench --bench rust_internals`
//!
//! These exercise the pure-Rust hot paths (stats, formatters) without the
//! Python-call overhead the Python-level dogfood at benches/bench_dogfood.py
//! includes. Use them to validate optimisations to the Rust core in
//! isolation; use bench_dogfood.py to validate the end-to-end harness.

use std::hint::black_box;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use fastrand::Rng;

use _pybench::histogram::HdrHistogram;
use _pybench::report;
use _pybench::runner::BenchmarkResult;
use _pybench::stats::{self, OutlierMethod};

// ---------- Helpers ----------

fn synthetic_samples(n: usize, seed: u64) -> Vec<i64> {
    let mut rng = Rng::with_seed(seed);
    (0..n)
        .map(|_| 1_000 + (rng.i64(..) % 500).abs())
        .collect()
}

/// Build a BenchmarkResult without going through the PyO3-bound from_times
/// constructor (which requires a Python token).
fn make_dummy_result(name: String, times: Vec<i64>) -> BenchmarkResult {
    let n = times.len();
    let min_ns = *times.iter().min().unwrap_or(&0);
    let max_ns = *times.iter().max().unwrap_or(&0);
    BenchmarkResult {
        name,
        times_ns: times,
        iterations: n,
        batch_size: 1,
        mean_ns: 100.0,
        clean_mean_ns: 100.0,
        median_ns: 100.0,
        stddev_ns: 10.0,
        min_ns,
        max_ns,
        ops_per_sec: 10_000_000.0,
        outliers: 0,
        ci95_low_ns: 95.0,
        ci95_high_ns: 105.0,
        throughput_per_sec: None,
        param: None,
        histogram: None,
    }
}

fn make_results(n: usize) -> Vec<BenchmarkResult> {
    (0..n)
        .map(|i| make_dummy_result(format!("bench_{i}"), synthetic_samples(50, i as u64)))
        .collect()
}

// ---------- Stats ----------

fn bench_mean(c: &mut Criterion) {
    let mut group = c.benchmark_group("stats/mean");
    for &n in &[100_usize, 1_000, 10_000] {
        let xs = synthetic_samples(n, 0xA);
        group.throughput(Throughput::Elements(n as u64));
        group.bench_with_input(BenchmarkId::from_parameter(n), &xs, |b, xs| {
            b.iter(|| stats::mean(black_box(xs)));
        });
    }
    group.finish();
}

fn bench_median(c: &mut Criterion) {
    let mut group = c.benchmark_group("stats/median");
    for &n in &[100_usize, 1_000, 10_000] {
        let xs = synthetic_samples(n, 0xB);
        let mut scratch: Vec<i64> = Vec::with_capacity(n);
        group.throughput(Throughput::Elements(n as u64));
        group.bench_with_input(BenchmarkId::from_parameter(n), &xs, |b, xs| {
            b.iter(|| stats::median(black_box(xs), &mut scratch));
        });
    }
    group.finish();
}

fn bench_stddev(c: &mut Criterion) {
    let mut group = c.benchmark_group("stats/stddev");
    for &n in &[100_usize, 1_000, 10_000] {
        let xs = synthetic_samples(n, 0xC);
        group.throughput(Throughput::Elements(n as u64));
        group.bench_with_input(BenchmarkId::from_parameter(n), &xs, |b, xs| {
            b.iter(|| stats::stddev(black_box(xs)));
        });
    }
    group.finish();
}

fn bench_outliers(c: &mut Criterion) {
    let mut group = c.benchmark_group("stats/outliers");
    let n = 1_000_usize;
    let xs = synthetic_samples(n, 0xD);
    for (label, method) in &[("tukey", OutlierMethod::Tukey), ("mad", OutlierMethod::Mad)] {
        let mut scratch: Vec<i64> = Vec::with_capacity(n);
        group.throughput(Throughput::Elements(n as u64));
        group.bench_function(*label, |b| {
            b.iter(|| stats::detect_outliers(black_box(&xs), *method, &mut scratch));
        });
    }
    group.finish();
}

fn bench_bootstrap_ci(c: &mut Criterion) {
    let mut group = c.benchmark_group("stats/bootstrap_ci");
    let n_samples = 200_usize;
    let xs = synthetic_samples(n_samples, 0xE);
    for &n_resamples in &[1_000_usize, 5_000, 10_000] {
        let mut means: Vec<f64> = Vec::with_capacity(n_resamples);
        let mut rng = Rng::with_seed(0xBEEF);
        group.throughput(Throughput::Elements(n_resamples as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(n_resamples),
            &n_resamples,
            |b, &n_resamples| {
                b.iter(|| {
                    stats::bootstrap_ci_mean(
                        black_box(&xs),
                        0.95,
                        n_resamples,
                        &mut rng,
                        &mut means,
                    )
                });
            },
        );
    }
    group.finish();
}

// ---------- Histogram ----------

fn bench_histogram_from_samples(c: &mut Criterion) {
    let mut group = c.benchmark_group("histogram/from_samples");
    for &n in &[100_usize, 1_000, 10_000] {
        let xs = synthetic_samples(n, 0xF);
        group.throughput(Throughput::Elements(n as u64));
        group.bench_with_input(BenchmarkId::from_parameter(n), &xs, |b, xs| {
            b.iter(|| HdrHistogram::from_samples(black_box(xs)));
        });
    }
    group.finish();
}

// ---------- Report formatting ----------

fn bench_format_json(c: &mut Criterion) {
    let mut group = c.benchmark_group("report/format_json");
    for &n in &[1_usize, 10, 100] {
        let owned = make_results(n);
        let refs: Vec<&BenchmarkResult> = owned.iter().collect();
        group.throughput(Throughput::Elements(n as u64));
        group.bench_with_input(BenchmarkId::from_parameter(n), &refs, |b, refs| {
            b.iter(|| report::format_json_refs(black_box(refs), "{}"));
        });
    }
    group.finish();
}

fn bench_format_table(c: &mut Criterion) {
    let mut group = c.benchmark_group("report/format_table");
    for &n in &[1_usize, 10, 100] {
        let owned = make_results(n);
        let refs: Vec<&BenchmarkResult> = owned.iter().collect();
        group.throughput(Throughput::Elements(n as u64));
        group.bench_with_input(BenchmarkId::from_parameter(n), &refs, |b, refs| {
            b.iter(|| report::format_table_refs(black_box(refs)));
        });
    }
    group.finish();
}

fn bench_format_html(c: &mut Criterion) {
    let mut group = c.benchmark_group("report/format_html");
    for &n in &[1_usize, 10, 100] {
        let owned = make_results(n);
        let refs: Vec<&BenchmarkResult> = owned.iter().collect();
        group.throughput(Throughput::Elements(n as u64));
        group.bench_with_input(BenchmarkId::from_parameter(n), &refs, |b, refs| {
            b.iter(|| report::format_html_refs(black_box(refs), "meta"));
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_mean,
    bench_median,
    bench_stddev,
    bench_outliers,
    bench_bootstrap_ci,
    bench_histogram_from_samples,
    bench_format_json,
    bench_format_table,
    bench_format_html,
);
criterion_main!(benches);
