//! Storytelling-with-Data treatment pass for the tplot toolchain.

pub mod focal;
pub mod palette;
pub mod takeaway;

pub use focal::{pick_focal, FocalChoice, FocalResult, SeriesPoint};
pub use palette::build_palette_map;
pub use takeaway::bar_takeaway;

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
    series:   &[SeriesPoint],
    config:   &StoryConfig,
    palette:  Palette,
) -> StoryAnnotated {
    if !config.enabled {
        // Neutral mode: every series gets the focal color (deliberately —
        // user opted out of the gray-down treatment).
        let map = series.iter()
            .map(|p| (p.key.clone(), palette.focal_color()))
            .collect();
        return StoryAnnotated {
            focal: None,
            palette_map: map,
            takeaway: config.annotation.clone(),
        };
    }

    let focal_choice = match &config.focus {
        FocusMode::Auto             => pick_focal(series),
        FocusMode::Series(name)     => FocalResult {
            choice: FocalChoice::Series(name.clone()),
            trust_score: f64::INFINITY,
            reason: "user-specified",
        },
        FocusMode::None             => FocalResult {
            choice: FocalChoice::None,
            trust_score: 0.0,
            reason: "user-disabled",
        },
    };

    let focal_name = match &focal_choice.choice {
        FocalChoice::Series(s) => Some(s.as_str()),
        FocalChoice::None      => None,
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
        let focal_value = focal_name.and_then(|n| series.iter()
            .find(|p| p.key == n).map(|p| p.value)).unwrap_or(0.0);
        Some(bar_takeaway(focal_name, focal_value, median, series.len()))
    };

    StoryAnnotated {
        focal: focal_name.map(String::from),
        palette_map,
        takeaway,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pts(v: &[(&str, f64)]) -> Vec<SeriesPoint> {
        v.iter().map(|(k, v)| SeriesPoint { key: k.to_string(), value: *v }).collect()
    }

    #[test]
    fn end_to_end_default_config() {
        let series = pts(&[("NA", 179.0), ("EMEA", 193.0), ("LATAM", 78.0), ("APAC", 97.0), ("AU", 53.0)]);
        let s = run_bar_story_pass(&series, &StoryConfig::default(), Palette::Signature);
        assert_eq!(s.focal.as_deref(), Some("EMEA"));
        assert!(s.takeaway.unwrap().contains("EMEA"));
    }

    #[test]
    fn neutral_mode_skips_story_pass() {
        let series = pts(&[("a", 1.0), ("b", 2.0)]);
        let cfg = StoryConfig { enabled: false, ..StoryConfig::default() };
        let s = run_bar_story_pass(&series, &cfg, Palette::Signature);
        assert!(s.focal.is_none());
        let unique: std::collections::HashSet<_> = s.palette_map.values().copied().collect();
        assert_eq!(unique.len(), 1);
    }

    #[test]
    fn user_focus_overrides_auto() {
        let series = pts(&[("a", 100.0), ("b", 1.0), ("c", 1.0)]);
        let cfg = StoryConfig { focus: FocusMode::Series("b".into()), ..StoryConfig::default() };
        let s = run_bar_story_pass(&series, &cfg, Palette::Signature);
        assert_eq!(s.focal.as_deref(), Some("b"));
    }
}
