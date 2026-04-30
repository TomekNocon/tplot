//! Gaussian kernel density estimation with Silverman's bandwidth rule.
//!
//! Used by violin and ridgeline plots to draw a smoothed distribution shape
//! per group. Pure-functional and dependency-free.

use std::f64::consts::PI;

/// Sample standard deviation. Returns 0.0 for n < 2.
pub fn std_dev(data: &[f64]) -> f64 {
    if data.len() < 2 {
        return 0.0;
    }
    let n = data.len() as f64;
    let mean = data.iter().sum::<f64>() / n;
    let var = data.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0);
    var.sqrt()
}

/// Silverman's rule of thumb for Gaussian-kernel bandwidth.
/// Falls back to a small positive value when the sample has zero variance
/// (single point or all-equal data) so the KDE remains defined.
pub fn silverman_bandwidth(data: &[f64]) -> f64 {
    if data.is_empty() {
        return 1.0;
    }
    let n = data.len() as f64;
    let s = std_dev(data);
    let h = 1.06 * s * n.powf(-1.0 / 5.0);
    if h > 1e-12 {
        h
    } else {
        // Degenerate case: use a fraction of the data range, or a constant if
        // even that is zero.
        let max = data.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let min = data.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let range = (max - min).max(1.0);
        range * 0.05
    }
}

/// Gaussian KDE evaluated at one point.
pub fn kde_at(y: f64, data: &[f64], h: f64) -> f64 {
    if data.is_empty() || h <= 0.0 {
        return 0.0;
    }
    let n = data.len() as f64;
    let inv_h = 1.0 / h;
    let inv_sqrt_2pi = 1.0 / (2.0 * PI).sqrt();
    let sum: f64 = data
        .iter()
        .map(|&x| {
            let z = (y - x) * inv_h;
            inv_sqrt_2pi * (-0.5 * z * z).exp()
        })
        .sum();
    sum * inv_h / n
}

/// Evaluate the KDE on every point in the given grid.
pub fn kde_evaluate(data: &[f64], grid: &[f64]) -> Vec<f64> {
    if data.is_empty() {
        return vec![0.0; grid.len()];
    }
    let h = silverman_bandwidth(data);
    grid.iter().map(|&y| kde_at(y, data, h)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64, eps: f64) {
        assert!((a - b).abs() < eps, "expected ≈{b}, got {a}");
    }

    #[test]
    fn silverman_bandwidth_for_unit_normal() {
        // Ten samples drawn roughly from N(0, 1). Silverman's rule:
        // h = 1.06 * stddev * n^(-1/5)
        let data = vec![-1.5, -1.0, -0.5, 0.0, 0.0, 0.0, 0.5, 1.0, 1.5, 2.0];
        let n = data.len() as f64;
        // stddev of this sample is roughly 1.04
        let h = silverman_bandwidth(&data);
        let expected = 1.06 * std_dev(&data) * n.powf(-1.0 / 5.0);
        approx(h, expected, 1e-6);
    }

    #[test]
    fn kde_at_a_point_in_a_dense_region_is_higher() {
        let data = vec![0.0, 0.1, -0.1, 0.2, -0.2, 5.0];
        let h = silverman_bandwidth(&data);
        let dense = kde_at(0.0, &data, h);
        let sparse = kde_at(5.0, &data, h);
        let outside = kde_at(15.0, &data, h);
        assert!(dense > sparse, "density at 0 should exceed density at 5");
        assert!(sparse > outside, "density at 5 should exceed density at 15");
    }

    #[test]
    fn kde_evaluate_grid_returns_one_value_per_grid_point() {
        let data = vec![0.0, 1.0, 2.0];
        let grid = vec![-1.0, 0.0, 1.0, 2.0, 3.0];
        let densities = kde_evaluate(&data, &grid);
        assert_eq!(densities.len(), 5);
        for d in &densities {
            assert!(*d >= 0.0, "density must be non-negative");
        }
    }

    #[test]
    fn kde_handles_empty_data_returns_zeros() {
        let densities = kde_evaluate(&[], &[0.0, 1.0]);
        assert_eq!(densities.len(), 2);
        assert!(densities.iter().all(|d| *d == 0.0));
    }

    #[test]
    fn kde_handles_single_point_with_finite_bandwidth() {
        let data = vec![5.0];
        let h = silverman_bandwidth(&data);
        // For a single point, stddev = 0; bandwidth defaults to a small positive
        // value so the KDE is still defined (a sharp peak around the point).
        assert!(h > 0.0);
        let at_point = kde_at(5.0, &data, h);
        let far_away = kde_at(50.0, &data, h);
        assert!(at_point > far_away);
    }
}
