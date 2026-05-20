//! Stats over `&[i64]` sample vectors. All ns-valued.
//!
//! Functions that need a mutable working buffer accept one as `scratch: &mut
//! Vec<i64>`; the caller owns it and can reuse it across calls so a single
//! `Vec` allocation covers `median` + `tukey`/`mad` for the whole benchmark.

pub fn mean(xs: &[i64]) -> f64 {
    debug_assert!(!xs.is_empty());
    xs.iter().sum::<i64>() as f64 / xs.len() as f64
}

pub fn median(xs: &[i64], scratch: &mut Vec<i64>) -> f64 {
    debug_assert!(!xs.is_empty());
    scratch.clear();
    scratch.extend_from_slice(xs);
    let n = scratch.len();
    let mid = n / 2;
    let (_, hi, _) = scratch.select_nth_unstable(mid);
    let hi = *hi as f64;
    if n % 2 == 1 {
        hi
    } else {
        let (_, lo, _) = scratch[..mid].select_nth_unstable(mid - 1);
        (hi + *lo as f64) / 2.0
    }
}

pub fn stddev(xs: &[i64]) -> f64 {
    if xs.len() < 2 {
        return 0.0;
    }
    let m = mean(xs);
    let n = xs.len() as f64;
    let var = xs
        .iter()
        .map(|&x| {
            let d = x as f64 - m;
            d * d
        })
        .sum::<f64>()
        / (n - 1.0);
    var.sqrt()
}

#[derive(Copy, Clone, Debug)]
pub enum OutlierMethod {
    Tukey,
    Mad,
    None,
}

/// Returns (clean_mean, outlier_count). `scratch` is overwritten.
pub fn detect_outliers(
    xs: &[i64],
    method: OutlierMethod,
    scratch: &mut Vec<i64>,
) -> (f64, usize) {
    match method {
        OutlierMethod::None => (mean(xs), 0),
        OutlierMethod::Tukey => tukey(xs, scratch),
        OutlierMethod::Mad => mad(xs, scratch),
    }
}

fn tukey(xs: &[i64], scratch: &mut Vec<i64>) -> (f64, usize) {
    scratch.clear();
    scratch.extend_from_slice(xs);
    let n = scratch.len();
    let q1 = quantile(scratch, n / 4);
    let q3 = quantile(scratch, (3 * n) / 4);
    let iqr = q3 - q1;
    let lo = q1 - 1.5 * iqr;
    let hi = q3 + 1.5 * iqr;
    let mut clean_sum = 0.0;
    let mut clean_n = 0usize;
    let mut outliers = 0usize;
    for &x in xs {
        let xf = x as f64;
        if xf < lo || xf > hi {
            outliers += 1;
        } else {
            clean_sum += xf;
            clean_n += 1;
        }
    }
    debug_assert!(clean_n > 0);
    (clean_sum / clean_n as f64, outliers)
}

fn mad(xs: &[i64], scratch: &mut Vec<i64>) -> (f64, usize) {
    let med = median(xs, scratch);
    scratch.clear();
    scratch.extend(xs.iter().map(|&x| (x as f64 - med).abs() as i64));
    let n = scratch.len();
    let mad_val = quantile(scratch, n / 2);
    let threshold = 3.5 * mad_val;
    let mut clean_sum = 0.0;
    let mut clean_n = 0usize;
    let mut outliers = 0usize;
    for &x in xs {
        if threshold > 0.0 && (x as f64 - med).abs() > threshold {
            outliers += 1;
        } else {
            clean_sum += x as f64;
            clean_n += 1;
        }
    }
    debug_assert!(clean_n > 0);
    (clean_sum / clean_n as f64, outliers)
}

fn quantile(buf: &mut [i64], k: usize) -> f64 {
    let k = k.min(buf.len() - 1);
    let (_, v, _) = buf.select_nth_unstable(k);
    *v as f64
}

/// Percentile bootstrap CI for the mean. `means_scratch` is overwritten and
/// must outlive the caller — reuse it across benchmarks to avoid 80KB of
/// allocator churn per call.
pub fn bootstrap_ci_mean(
    xs: &[i64],
    level: f64,
    n_resamples: usize,
    rng: &mut fastrand::Rng,
    means_scratch: &mut Vec<f64>,
) -> (f64, f64) {
    debug_assert!(!xs.is_empty());
    debug_assert!(level > 0.0 && level < 1.0);
    let n = xs.len();
    means_scratch.clear();
    means_scratch.reserve(n_resamples);
    for _ in 0..n_resamples {
        // i128 accumulator: a single u64 sum could plausibly approach i64::MAX
        // for very long benchmarks (e.g. 10^9 ns × 10^9 samples). i128 has
        // headroom for any realistic timing run.
        let mut sum: i128 = 0;
        for _ in 0..n {
            sum += xs[rng.usize(..n)] as i128;
        }
        means_scratch.push(sum as f64 / n as f64);
    }
    let alpha = (1.0 - level) / 2.0;
    let lo_idx = (alpha * n_resamples as f64) as usize;
    let hi_idx = (((1.0 - alpha) * n_resamples as f64) as usize).min(n_resamples - 1);
    // total_cmp is NaN-safe; partial_cmp().unwrap() would panic if any mean
    // ever ended up NaN.
    let cmp = |a: &f64, b: &f64| a.total_cmp(b);
    let (_, lo, _) = means_scratch.select_nth_unstable_by(lo_idx, cmp);
    let lo_value = *lo;
    let (_, hi, _) = means_scratch.select_nth_unstable_by(hi_idx, cmp);
    (lo_value, *hi)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mean_of_constant() {
        assert_eq!(mean(&[10, 10, 10, 10]), 10.0);
    }

    #[test]
    fn median_odd() {
        let mut s = Vec::new();
        assert_eq!(median(&[1, 2, 3, 4, 5], &mut s), 3.0);
    }

    #[test]
    fn median_even() {
        let mut s = Vec::new();
        assert_eq!(median(&[1, 2, 3, 4], &mut s), 2.5);
    }

    #[test]
    fn stddev_one_sample_is_zero() {
        assert_eq!(stddev(&[42]), 0.0);
    }

    #[test]
    fn stddev_known() {
        let v = stddev(&[2, 4, 4, 4, 5, 5, 7, 9]);
        assert!((v - 2.138).abs() < 0.01);
    }

    #[test]
    fn tukey_flags_obvious_outlier() {
        let xs: Vec<i64> = (10..30).chain(std::iter::once(10_000)).collect();
        let mut s = Vec::new();
        let (clean, n) = detect_outliers(&xs, OutlierMethod::Tukey, &mut s);
        assert_eq!(n, 1);
        assert!(clean < 30.0);
    }

    #[test]
    fn mad_flags_obvious_outlier() {
        // 19 tightly-clustered samples plus one huge spike.
        let mut xs: Vec<i64> = vec![10, 11, 9, 10, 12, 8, 10, 11, 9, 10, 12, 8, 9, 11, 10, 9, 11, 10, 12];
        xs.push(10_000);
        let mut s = Vec::new();
        let (clean, n) = detect_outliers(&xs, OutlierMethod::Mad, &mut s);
        assert!(n >= 1, "MAD should flag the 10_000 spike against a ~10 baseline");
        assert!(clean < 20.0);
    }

    #[test]
    fn bootstrap_ci_contains_mean() {
        let mut rng = fastrand::Rng::with_seed(0xDEAD_BEEF);
        let xs: Vec<i64> = (0..1000).collect();
        let m = mean(&xs);
        let mut means = Vec::new();
        let (lo, hi) = bootstrap_ci_mean(&xs, 0.95, 1000, &mut rng, &mut means);
        assert!(lo <= m && m <= hi, "CI [{lo}, {hi}] should contain mean {m}");
    }

    #[test]
    fn scratch_buffer_can_be_reused_across_calls() {
        let xs1: Vec<i64> = (0..50).collect();
        let xs2: Vec<i64> = (100..200).collect();
        let mut scratch = Vec::new();

        let m1 = median(&xs1, &mut scratch);
        let m2 = median(&xs2, &mut scratch);
        assert_eq!(m1, 24.5);
        assert_eq!(m2, 149.5);
    }
}
