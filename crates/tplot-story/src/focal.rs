#[derive(Debug, Clone)]
pub struct SeriesPoint {
    pub key: String,
    pub value: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FocalChoice {
    Series(String),
    None,
}

#[derive(Debug, Clone)]
pub struct FocalResult {
    pub choice: FocalChoice,
    pub trust_score: f64,
    /// "max" | "delta" | "outlier" — for the takeaway template.
    pub reason: &'static str,
}

const TRUST_THRESHOLD: f64 = 1.5;

/// Single-pass focal detection on an aggregated series. For v1 (single-series
/// horizontal bar) we score by max-vs-median dominance only; outlier and delta
/// signals come in plan 3 alongside line charts.
pub fn pick_focal(points: &[SeriesPoint]) -> FocalResult {
    if points.is_empty() {
        return FocalResult { choice: FocalChoice::None, trust_score: 0.0, reason: "empty" };
    }

    let mut sorted: Vec<f64> = points.iter().map(|p| p.value).collect();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = sorted[sorted.len() / 2];

    let max_point = points.iter().max_by(|a, b| a.value.partial_cmp(&b.value)
        .unwrap_or(std::cmp::Ordering::Equal)).unwrap();

    let trust = if median.abs() < 1e-9 {
        // Median is zero — any nonzero leader dominates trivially.
        if max_point.value.abs() > 0.0 { f64::INFINITY } else { 0.0 }
    } else {
        max_point.value / median
    };

    if trust >= TRUST_THRESHOLD {
        FocalResult {
            choice: FocalChoice::Series(max_point.key.clone()),
            trust_score: trust,
            reason: "max",
        }
    } else {
        FocalResult {
            choice: FocalChoice::None,
            trust_score: trust,
            reason: "no-dominant-series",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(values: &[(&str, f64)]) -> Vec<SeriesPoint> {
        values.iter().map(|(k, v)| SeriesPoint { key: k.to_string(), value: *v }).collect()
    }

    #[test]
    fn picks_clear_max_when_dominant() {
        let r = pick_focal(&rows(&[("a", 10.0), ("b", 11.0), ("c", 9.5), ("d", 72.0), ("e", 12.0)]));
        assert_eq!(r.choice, FocalChoice::Series("d".into()));
        assert!(r.trust_score > 1.5);
    }

    #[test]
    fn admits_no_focal_when_uniform() {
        let r = pick_focal(&rows(&[("a", 50.0), ("b", 51.0), ("c", 49.0), ("d", 50.5)]));
        assert_eq!(r.choice, FocalChoice::None);
        assert!(r.trust_score < 1.5);
    }

    #[test]
    fn empty_input_returns_none() {
        let r = pick_focal(&[]);
        assert_eq!(r.choice, FocalChoice::None);
    }
}
