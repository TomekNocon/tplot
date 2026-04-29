use std::collections::HashMap;
use tplot_protocol::{Palette, RgbColor};

pub fn build_palette_map(
    series_keys: &[&str],
    focal: Option<&str>,
    palette: Palette,
) -> HashMap<String, RgbColor> {
    let focal_color = palette.focal_color();
    let context_color = palette.context_color();
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
}
