//! Stats over `&[i64]` sample vectors. All ns-valued.

pub fn mean(xs: &[i64]) -> f64 {
    debug_assert!(!xs.is_empty());
    xs.iter().sum::<i64>() as f64 / xs.len() as f64
}

pub fn median(xs: &[i64]) -> f64 {
    debug_assert!(!xs.is_empty());
    let mut buf = xs.to_vec();
    let n = buf.len();
    let mid = n / 2;
    let (_, hi, _) = buf.select_nth_unstable(mid);
    let hi = *hi as f64;
    if n % 2 == 1 {
        hi
    } else {
        let (_, lo, _) = buf[..mid].select_nth_unstable(mid - 1);
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

/// Returns (clean_mean, outlier_count).
pub fn detect_outliers(xs: &[i64], method: OutlierMethod) -> (f64, usize) {
    match method {
        OutlierMethod::None => (mean(xs), 0),
        OutlierMethod::Tukey => tukey(xs),
        OutlierMethod::Mad => mad(xs),
    }
}

fn tukey(xs: &[i64]) -> (f64, usize) {
    let mut buf = xs.to_vec();
    let n = buf.len();
    let q1 = quantile(&mut buf, n / 4);
    let q3 = quantile(&mut buf, (3 * n) / 4);
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
    // Q1 and Q3 are interior quantiles, so at least one sample always survives.
    debug_assert!(clean_n > 0);
    (clean_sum / clean_n as f64, outliers)
}

fn mad(xs: &[i64]) -> (f64, usize) {
    let med = median(xs);
    let mut deviations: Vec<i64> = xs.iter().map(|&x| (x as f64 - med).abs() as i64).collect();
    let n = deviations.len();
    let mad_val = quantile(&mut deviations, n / 2);
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
    // The median itself is never an outlier from itself, so at least one sample survives.
    debug_assert!(clean_n > 0);
    (clean_sum / clean_n as f64, outliers)
}

fn quantile(buf: &mut [i64], k: usize) -> f64 {
    let k = k.min(buf.len() - 1);
    let (_, v, _) = buf.select_nth_unstable(k);
    *v as f64
}

/// Percentile bootstrap CI for the mean.
pub fn bootstrap_ci_mean(
    xs: &[i64],
    level: f64,
    n_resamples: usize,
    rng: &mut fastrand::Rng,
) -> (f64, f64) {
    debug_assert!(!xs.is_empty());
    debug_assert!(level > 0.0 && level < 1.0);
    let n = xs.len();
    let mut means: Vec<f64> = Vec::with_capacity(n_resamples);
    for _ in 0..n_resamples {
        let mut sum: i64 = 0;
        for _ in 0..n {
            sum += xs[rng.usize(..n)];
        }
        means.push(sum as f64 / n as f64);
    }
    means.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let alpha = (1.0 - level) / 2.0;
    let lo_idx = (alpha * n_resamples as f64) as usize;
    let hi_idx = (((1.0 - alpha) * n_resamples as f64) as usize).min(n_resamples - 1);
    (means[lo_idx], means[hi_idx])
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
        assert_eq!(median(&[1, 2, 3, 4, 5]), 3.0);
    }

    #[test]
    fn median_even() {
        assert_eq!(median(&[1, 2, 3, 4]), 2.5);
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
        let (clean, n) = detect_outliers(&xs, OutlierMethod::Tukey);
        assert_eq!(n, 1);
        assert!(clean < 30.0);
    }

    #[test]
    fn bootstrap_ci_contains_mean() {
        let mut rng = fastrand::Rng::with_seed(0xDEAD_BEEF);
        let xs: Vec<i64> = (0..1000).collect();
        let m = mean(&xs);
        let (lo, hi) = bootstrap_ci_mean(&xs, 0.95, 1000, &mut rng);
        assert!(lo <= m && m <= hi, "CI [{lo}, {hi}] should contain mean {m}");
    }
}
