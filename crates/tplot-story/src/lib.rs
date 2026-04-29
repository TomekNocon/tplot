//! Storytelling-with-Data treatment pass for the tplot toolchain.

pub mod focal;
pub mod palette;
pub mod takeaway;

pub use focal::{
    FocalChoice, FocalResult, SeriesPoint, SeriesSpread, SeriesTrend, pick_focal,
    pick_focal_by_delta, pick_focal_by_iqr,
};
pub use palette::build_palette_map;
pub use takeaway::{
    bar_takeaway, boxplot_takeaway, heatmap_takeaway, histogram_takeaway, line_takeaway,
};

use std::collections::HashMap;
use tplot_protocol::{FocusMode, Palette, RgbColor, StoryConfig};

#[derive(Debug, Clone)]
pub struct StoryAnnotated {
    pub focal: Option<String>,
    pub palette_map: HashMap<String, RgbColor>,
    pub takeaway: Option<String>,
}

/// Run the story-pass on a single-series bar chart with the given series
/// values. Returns a `StoryAnnotated` ready for the layout/render stages.
pub fn run_bar_story_pass(
    series: &[SeriesPoint],
    config: &StoryConfig,
    palette: Palette,
) -> StoryAnnotated {
    if !config.enabled {
        // Neutral mode: every series gets the focal color (deliberately —
        // user opted out of the gray-down treatment).
        let map = series
            .iter()
            .map(|p| (p.key.clone(), palette.focal_color()))
            .collect();
        return StoryAnnotated {
            focal: None,
            palette_map: map,
            takeaway: config.annotation.clone(),
        };
    }

    let focal_choice = match &config.focus {
        FocusMode::Auto => pick_focal(series),
        FocusMode::Series(name) => FocalResult {
            choice: FocalChoice::Series(name.clone()),
            trust_score: f64::INFINITY,
            reason: "user-specified",
        },
        FocusMode::None => FocalResult {
            choice: FocalChoice::None,
            trust_score: 0.0,
            reason: "user-disabled",
        },
    };

    let focal_name = match &focal_choice.choice {
        FocalChoice::Series(s) => Some(s.as_str()),
        FocalChoice::None => None,
    };

    let keys: Vec<&str> = series.iter().map(|p| p.key.as_str()).collect();
    let palette_map = build_palette_map(&keys, focal_name, palette);

    let takeaway = if !config.takeaway {
        None
    } else if let Some(custom) = &config.annotation {
        Some(custom.clone())
    } else {
        let median = {
            let mut v: Vec<f64> = series.iter().map(|p| p.value).collect();
            v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            v.get(v.len() / 2).copied().unwrap_or(0.0)
        };
        let focal_value = focal_name
            .and_then(|n| series.iter().find(|p| p.key == n).map(|p| p.value))
            .unwrap_or(0.0);
        Some(bar_takeaway(focal_name, focal_value, median, series.len()))
    };

    StoryAnnotated {
        focal: focal_name.map(String::from),
        palette_map,
        takeaway,
    }
}

/// Run the story-pass on a histogram (one bin per `SeriesPoint`). Picks the
/// modal bin via `pick_focal` (max-vs-median dominance ≥ 1.5×) and emits a
/// modal-cluster takeaway.
pub fn run_histogram_story_pass(
    bins: &[SeriesPoint],
    config: &StoryConfig,
    palette: Palette,
) -> StoryAnnotated {
    if !config.enabled {
        let map = bins
            .iter()
            .map(|p| (p.key.clone(), palette.focal_color()))
            .collect();
        return StoryAnnotated {
            focal: None,
            palette_map: map,
            takeaway: config.annotation.clone(),
        };
    }

    let total: u64 = bins.iter().map(|p| p.value as u64).sum();
    let focal_choice = match &config.focus {
        FocusMode::Auto => pick_focal(bins),
        FocusMode::Series(name) => FocalResult {
            choice: FocalChoice::Series(name.clone()),
            trust_score: f64::INFINITY,
            reason: "user-specified",
        },
        FocusMode::None => FocalResult {
            choice: FocalChoice::None,
            trust_score: 0.0,
            reason: "user-disabled",
        },
    };

    let focal_name = match &focal_choice.choice {
        FocalChoice::Series(s) => Some(s.as_str()),
        FocalChoice::None => None,
    };

    let keys: Vec<&str> = bins.iter().map(|p| p.key.as_str()).collect();
    let palette_map = build_palette_map(&keys, focal_name, palette);

    let takeaway = if !config.takeaway {
        None
    } else if let Some(custom) = &config.annotation {
        Some(custom.clone())
    } else {
        let modal_count = focal_name
            .and_then(|n| bins.iter().find(|p| p.key == n).map(|p| p.value as u64))
            .unwrap_or(0);
        Some(takeaway::histogram_takeaway(focal_name, modal_count, total))
    };

    StoryAnnotated {
        focal: focal_name.map(String::from),
        palette_map,
        takeaway,
    }
}

/// Run the story-pass on a line chart. `trends` is the per-series first/last
/// summary; the focal series is picked by largest absolute delta, gated by a
/// 1.5× trust threshold against the median delta.
pub fn run_line_story_pass(
    trends: &[SeriesTrend],
    config: &StoryConfig,
    palette: Palette,
) -> StoryAnnotated {
    if !config.enabled {
        let map = trends
            .iter()
            .map(|t| (t.key.clone(), palette.focal_color()))
            .collect();
        return StoryAnnotated {
            focal: None,
            palette_map: map,
            takeaway: config.annotation.clone(),
        };
    }

    let focal_choice = match &config.focus {
        FocusMode::Auto => pick_focal_by_delta(trends),
        FocusMode::Series(name) => FocalResult {
            choice: FocalChoice::Series(name.clone()),
            trust_score: f64::INFINITY,
            reason: "user-specified",
        },
        FocusMode::None => FocalResult {
            choice: FocalChoice::None,
            trust_score: 0.0,
            reason: "user-disabled",
        },
    };

    let focal_name = match &focal_choice.choice {
        FocalChoice::Series(s) => Some(s.as_str()),
        FocalChoice::None => None,
    };

    let keys: Vec<&str> = trends.iter().map(|t| t.key.as_str()).collect();
    let palette_map = build_palette_map(&keys, focal_name, palette);

    let takeaway = if !config.takeaway {
        None
    } else if let Some(custom) = &config.annotation {
        Some(custom.clone())
    } else {
        let (first, last) = focal_name
            .and_then(|n| {
                trends
                    .iter()
                    .find(|t| t.key == n)
                    .map(|t| (t.first, t.last))
            })
            .unwrap_or((0.0, 0.0));
        Some(takeaway::line_takeaway(focal_name, first, last))
    };

    StoryAnnotated {
        focal: focal_name.map(String::from),
        palette_map,
        takeaway,
    }
}

/// Run the story-pass on a box plot. `spreads` is the per-series IQR; the
/// focal series is picked by largest IQR, gated by a 1.5× trust threshold
/// against the median IQR. Takeaway is `None` — the binary composes it with
/// full Q1/Q3/min/max info from the layout.
pub fn run_boxplot_story_pass(
    spreads: &[SeriesSpread],
    config: &StoryConfig,
    palette: Palette,
) -> StoryAnnotated {
    if !config.enabled {
        let map = spreads
            .iter()
            .map(|s| (s.key.clone(), palette.focal_color()))
            .collect();
        return StoryAnnotated {
            focal: None,
            palette_map: map,
            takeaway: config.annotation.clone(),
        };
    }
    let focal_choice = match &config.focus {
        FocusMode::Auto => pick_focal_by_iqr(spreads),
        FocusMode::Series(name) => FocalResult {
            choice: FocalChoice::Series(name.clone()),
            trust_score: f64::INFINITY,
            reason: "user-specified",
        },
        FocusMode::None => FocalResult {
            choice: FocalChoice::None,
            trust_score: 0.0,
            reason: "user-disabled",
        },
    };
    let focal_name = match &focal_choice.choice {
        FocalChoice::Series(s) => Some(s.as_str()),
        FocalChoice::None => None,
    };
    let keys: Vec<&str> = spreads.iter().map(|s| s.key.as_str()).collect();
    let palette_map = build_palette_map(&keys, focal_name, palette);
    StoryAnnotated {
        focal: focal_name.map(String::from),
        palette_map,
        takeaway: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pts(v: &[(&str, f64)]) -> Vec<SeriesPoint> {
        v.iter()
            .map(|(k, v)| SeriesPoint {
                key: k.to_string(),
                value: *v,
            })
            .collect()
    }

    #[test]
    fn end_to_end_default_config() {
        let series = pts(&[
            ("NA", 179.0),
            ("EMEA", 193.0),
            ("LATAM", 78.0),
            ("APAC", 97.0),
            ("AU", 53.0),
        ]);
        let s = run_bar_story_pass(&series, &StoryConfig::default(), Palette::Signature);
        assert_eq!(s.focal.as_deref(), Some("EMEA"));
        assert!(s.takeaway.unwrap().contains("EMEA"));
    }

    #[test]
    fn neutral_mode_skips_story_pass() {
        let series = pts(&[("a", 1.0), ("b", 2.0)]);
        let cfg = StoryConfig {
            enabled: false,
            ..StoryConfig::default()
        };
        let s = run_bar_story_pass(&series, &cfg, Palette::Signature);
        assert!(s.focal.is_none());
        let unique: std::collections::HashSet<_> = s.palette_map.values().copied().collect();
        assert_eq!(unique.len(), 1);
    }

    #[test]
    fn user_focus_overrides_auto() {
        let series = pts(&[("a", 100.0), ("b", 1.0), ("c", 1.0)]);
        let cfg = StoryConfig {
            focus: FocusMode::Series("b".into()),
            ..StoryConfig::default()
        };
        let s = run_bar_story_pass(&series, &cfg, Palette::Signature);
        assert_eq!(s.focal.as_deref(), Some("b"));
    }

    #[test]
    fn histogram_story_pass_picks_modal_bin() {
        let bins = vec![
            SeriesPoint {
                key: "0–10".into(),
                value: 1.0,
            },
            SeriesPoint {
                key: "10–20".into(),
                value: 2.0,
            },
            SeriesPoint {
                key: "20–30".into(),
                value: 8.0,
            },
            SeriesPoint {
                key: "30–40".into(),
                value: 4.0,
            },
            SeriesPoint {
                key: "40–50".into(),
                value: 1.0,
            },
        ];
        let s = run_histogram_story_pass(&bins, &StoryConfig::default(), Palette::Signature);
        assert_eq!(s.focal.as_deref(), Some("20–30"));
        assert!(s.takeaway.unwrap().contains("20–30"));
    }

    #[test]
    fn histogram_story_pass_neutral_when_uniform() {
        let bins = vec![
            SeriesPoint {
                key: "a".into(),
                value: 5.0,
            },
            SeriesPoint {
                key: "b".into(),
                value: 5.0,
            },
            SeriesPoint {
                key: "c".into(),
                value: 5.0,
            },
            SeriesPoint {
                key: "d".into(),
                value: 6.0,
            },
        ];
        let s = run_histogram_story_pass(&bins, &StoryConfig::default(), Palette::Signature);
        assert!(s.focal.is_none());
    }
}
