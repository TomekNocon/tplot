use std::collections::HashMap;
use tplot_protocol::{Palette, RgbColor, Theme};

/// Backwards-compatible builder — defaults to `Theme::Dark`. Prefer
/// [`build_palette_map_for`] when the caller knows the terminal theme.
pub fn build_palette_map(
    series_keys: &[&str],
    focal: Option<&str>,
    palette: Palette,
) -> HashMap<String, RgbColor> {
    build_palette_map_for(series_keys, focal, palette, Theme::Dark)
}

/// Theme-aware builder: picks the focal color for the focal series and the
/// theme-appropriate context gray for all others.
pub fn build_palette_map_for(
    series_keys: &[&str],
    focal: Option<&str>,
    palette: Palette,
    theme: Theme,
) -> HashMap<String, RgbColor> {
    let focal_color = palette.focal_color();
    let context_color = palette.context_color_for(theme);
    series_keys
        .iter()
        .map(|k| {
            let color = if Some(*k) == focal {
                focal_color
            } else {
                context_color
            };
            (k.to_string(), color)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tplot_protocol::{Palette, RgbColor};

    #[test]
    fn focal_series_gets_focal_color() {
        let map = build_palette_map(
            &["NA", "EMEA", "LATAM", "APAC", "AU"],
            Some("EMEA"),
            Palette::Signature,
        );
        assert_eq!(
            map.get("EMEA"),
            Some(&RgbColor::from_hex("#ee7b3d").unwrap())
        );
    }

    #[test]
    fn other_series_share_context_color() {
        let map = build_palette_map(
            &["NA", "EMEA", "LATAM", "APAC", "AU"],
            Some("EMEA"),
            Palette::Signature,
        );
        let na = map.get("NA").copied().unwrap();
        let latam = map.get("LATAM").copied().unwrap();
        assert_eq!(na, latam);
        assert_ne!(na, RgbColor::from_hex("#ee7b3d").unwrap());
    }

    #[test]
    fn no_focal_paints_everyone_in_context() {
        let map = build_palette_map(&["NA", "EMEA", "LATAM"], None, Palette::Signature);
        let unique: std::collections::HashSet<_> = map.values().copied().collect();
        assert_eq!(unique.len(), 1);
    }

    #[test]
    fn light_theme_uses_darker_context_gray() {
        let dark_map =
            build_palette_map_for(&["A", "B"], Some("A"), Palette::Signature, Theme::Dark);
        let light_map =
            build_palette_map_for(&["A", "B"], Some("A"), Palette::Signature, Theme::Light);
        let dark_b = dark_map.get("B").copied().unwrap();
        let light_b = light_map.get("B").copied().unwrap();
        assert!(light_b.r < dark_b.r, "light-theme gray should be darker");
    }

    #[test]
    fn build_palette_map_default_matches_dark_theme() {
        let default_map = build_palette_map(&["A", "B"], Some("A"), Palette::Signature);
        let dark_map =
            build_palette_map_for(&["A", "B"], Some("A"), Palette::Signature, Theme::Dark);
        assert_eq!(default_map.get("B"), dark_map.get("B"));
    }
}
