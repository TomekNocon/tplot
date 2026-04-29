//! Quantile / 5-number-summary statistics.
//!
//! Uses R's "type 7" linear-interpolation method (also numpy's default and
//! Excel's `PERCENTILE.INC`).

/// Linear-interpolation quantile (R's "type 7" method, default in numpy/R/Excel).
/// Input slice is cloned and sorted internally.
pub fn quantile(v: &[f64], p: f64) -> f64 {
    if v.is_empty() {
        return f64::NAN;
    }
    let p = p.clamp(0.0, 1.0);
    let mut s: Vec<f64> = v.to_vec();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = s.len();
    if n == 1 {
        return s[0];
    }
    // h = (n - 1) * p (fractional rank, 0-indexed)
    let h = (n - 1) as f64 * p;
    let lo = h.floor() as usize;
    let hi = (lo + 1).min(n - 1);
    let frac = h - h.floor();
    s[lo] + (s[hi] - s[lo]) * frac
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FiveNumberSummary {
    pub min: f64,
    pub q1: f64,
    pub median: f64,
    pub q3: f64,
    pub max: f64,
}

impl FiveNumberSummary {
    pub fn iqr(&self) -> f64 {
        self.q3 - self.q1
    }
}

pub fn five_number_summary(v: &[f64]) -> Option<FiveNumberSummary> {
    if v.is_empty() {
        return None;
    }
    Some(FiveNumberSummary {
        min: quantile(v, 0.0),
        q1: quantile(v, 0.25),
        median: quantile(v, 0.5),
        q3: quantile(v, 0.75),
        max: quantile(v, 1.0),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-6, "expected {b}, got {a}");
    }

    #[test]
    fn quantile_at_0_is_min() {
        let v = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        approx(quantile(&v, 0.0), 1.0);
    }

    #[test]
    fn quantile_at_1_is_max() {
        let v = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        approx(quantile(&v, 1.0), 5.0);
    }

    #[test]
    fn median_of_odd_length_is_middle() {
        approx(quantile(&[1.0, 2.0, 3.0, 4.0, 5.0], 0.5), 3.0);
    }

    #[test]
    fn median_of_even_length_is_average() {
        approx(quantile(&[1.0, 2.0, 3.0, 4.0], 0.5), 2.5);
    }

    #[test]
    fn quartiles_use_linear_interpolation() {
        // 1..=8: Q1 = 2.75, Q3 = 6.25 (R's type-7 method)
        let v: Vec<f64> = (1..=8).map(|x| x as f64).collect();
        approx(quantile(&v, 0.25), 2.75);
        approx(quantile(&v, 0.75), 6.25);
    }

    #[test]
    fn five_number_summary_sorts_internally() {
        let v = vec![5.0, 1.0, 3.0, 4.0, 2.0];
        let s = five_number_summary(&v).unwrap();
        approx(s.min, 1.0);
        approx(s.q1, 2.0);
        approx(s.median, 3.0);
        approx(s.q3, 4.0);
        approx(s.max, 5.0);
    }

    #[test]
    fn five_number_summary_rejects_empty() {
        let v: Vec<f64> = vec![];
        assert!(five_number_summary(&v).is_none());
    }
}
