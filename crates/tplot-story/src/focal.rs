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
        return FocalResult {
            choice: FocalChoice::None,
            trust_score: 0.0,
            reason: "empty",
        };
    }

    let mut sorted: Vec<f64> = points.iter().map(|p| p.value).collect();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    // Statistical median: average of two middle values for even-length lists.
    let median = if sorted.len().is_multiple_of(2) {
        let mid = sorted.len() / 2;
        (sorted[mid - 1] + sorted[mid]) / 2.0
    } else {
        sorted[sorted.len() / 2]
    };

    let max_point = points
        .iter()
        .max_by(|a, b| {
            a.value
                .partial_cmp(&b.value)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .unwrap();

    let trust = if median.abs() < 1e-9 {
        // Median is zero — any nonzero leader dominates trivially.
        if max_point.value.abs() > 0.0 {
            f64::INFINITY
        } else {
            0.0
        }
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

#[derive(Debug, Clone)]
pub struct SeriesTrend {
    pub key: String,
    pub first: f64,
    pub last: f64,
}

/// Pick the series with the largest absolute delta (last - first), normalized
/// against the median absolute delta. Returns FocalChoice::None if no series
/// dominates (max delta < 1.5× median).
pub fn pick_focal_by_delta(trends: &[SeriesTrend]) -> FocalResult {
    if trends.is_empty() {
        return FocalResult {
            choice: FocalChoice::None,
            trust_score: 0.0,
            reason: "empty",
        };
    }

    let abs_deltas: Vec<f64> = trends.iter().map(|t| (t.last - t.first).abs()).collect();
    let mut sorted = abs_deltas.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    // Statistical median: average of two middle values for even-length lists
    // so a single dominant series isn't overshadowed by itself sitting at the
    // upper-half index.
    let median = if sorted.len().is_multiple_of(2) {
        let mid = sorted.len() / 2;
        (sorted[mid - 1] + sorted[mid]) / 2.0
    } else {
        sorted[sorted.len() / 2]
    };

    let max_idx = abs_deltas
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(i, _)| i)
        .unwrap();

    let trust = if median.abs() < 1e-9 {
        if abs_deltas[max_idx] > 0.0 {
            f64::INFINITY
        } else {
            0.0
        }
    } else {
        abs_deltas[max_idx] / median
    };

    if trust >= TRUST_THRESHOLD {
        FocalResult {
            choice: FocalChoice::Series(trends[max_idx].key.clone()),
            trust_score: trust,
            reason: "delta",
        }
    } else {
        FocalResult {
            choice: FocalChoice::None,
            trust_score: trust,
            reason: "no-clear-trend",
        }
    }
}

#[derive(Debug, Clone)]
pub struct SeriesSpread {
    pub key: String,
    pub iqr: f64,
}

/// Pick the series with the largest IQR (interquartile range), normalized
/// against the median IQR. Returns FocalChoice::None if no series dominates
/// (max IQR < 1.5× median).
pub fn pick_focal_by_iqr(spreads: &[SeriesSpread]) -> FocalResult {
    if spreads.is_empty() {
        return FocalResult {
            choice: FocalChoice::None,
            trust_score: 0.0,
            reason: "empty",
        };
    }
    let mut sorted: Vec<f64> = spreads.iter().map(|s| s.iqr).collect();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    // Standard median (mean of two middles for even-length).
    let n = sorted.len();
    let median = if n.is_multiple_of(2) {
        (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
    } else {
        sorted[n / 2]
    };
    let max_idx = spreads
        .iter()
        .enumerate()
        .max_by(|a, b| {
            a.1.iqr
                .partial_cmp(&b.1.iqr)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(i, _)| i)
        .unwrap();
    let trust = if median.abs() < 1e-9 {
        if spreads[max_idx].iqr > 0.0 {
            f64::INFINITY
        } else {
            0.0
        }
    } else {
        spreads[max_idx].iqr / median
    };
    if trust >= TRUST_THRESHOLD {
        FocalResult {
            choice: FocalChoice::Series(spreads[max_idx].key.clone()),
            trust_score: trust,
            reason: "iqr",
        }
    } else {
        FocalResult {
            choice: FocalChoice::None,
            trust_score: trust,
            reason: "uniform-spread",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(values: &[(&str, f64)]) -> Vec<SeriesPoint> {
        values
            .iter()
            .map(|(k, v)| SeriesPoint {
                key: k.to_string(),
                value: *v,
            })
            .collect()
    }

    #[test]
    fn picks_clear_max_when_dominant() {
        let r = pick_focal(&rows(&[
            ("a", 10.0),
            ("b", 11.0),
            ("c", 9.5),
            ("d", 72.0),
            ("e", 12.0),
        ]));
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

    #[test]
    fn picks_largest_delta_series() {
        // EMEA grew most in absolute terms; APAC stayed flat.
        let trends = vec![
            SeriesTrend {
                key: "NA".into(),
                first: 100.0,
                last: 105.0,
            },
            SeriesTrend {
                key: "EMEA".into(),
                first: 50.0,
                last: 200.0,
            },
            SeriesTrend {
                key: "LATAM".into(),
                first: 30.0,
                last: 28.0,
            },
            SeriesTrend {
                key: "APAC".into(),
                first: 80.0,
                last: 81.0,
            },
        ];
        let r = pick_focal_by_delta(&trends);
        assert_eq!(r.choice, FocalChoice::Series("EMEA".into()));
        assert!(r.trust_score > 1.5);
    }

    #[test]
    fn admits_no_focal_when_trends_uniform() {
        // All series move by similar magnitude (max-vs-median < 1.5×).
        let trends = vec![
            SeriesTrend {
                key: "a".into(),
                first: 50.0,
                last: 51.0,
            },
            SeriesTrend {
                key: "b".into(),
                first: 50.0,
                last: 51.4,
            },
            SeriesTrend {
                key: "c".into(),
                first: 50.0,
                last: 49.0,
            },
        ];
        let r = pick_focal_by_delta(&trends);
        assert_eq!(r.choice, FocalChoice::None);
    }

    #[test]
    fn picks_widest_iqr_series() {
        let spreads = vec![
            SeriesSpread {
                key: "A".into(),
                iqr: 10.0,
            },
            SeriesSpread {
                key: "B".into(),
                iqr: 12.0,
            },
            SeriesSpread {
                key: "C".into(),
                iqr: 80.0,
            }, // dominates
            SeriesSpread {
                key: "D".into(),
                iqr: 8.0,
            },
        ];
        let r = pick_focal_by_iqr(&spreads);
        assert_eq!(r.choice, FocalChoice::Series("C".into()));
    }

    #[test]
    fn admits_no_focal_when_iqr_uniform() {
        let spreads = vec![
            SeriesSpread {
                key: "A".into(),
                iqr: 10.0,
            },
            SeriesSpread {
                key: "B".into(),
                iqr: 11.0,
            },
            SeriesSpread {
                key: "C".into(),
                iqr: 9.0,
            },
            SeriesSpread {
                key: "D".into(),
                iqr: 10.5,
            },
        ];
        let r = pick_focal_by_iqr(&spreads);
        assert_eq!(r.choice, FocalChoice::None);
    }
}
