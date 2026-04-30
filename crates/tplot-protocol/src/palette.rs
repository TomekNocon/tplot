use crate::Theme;
use crate::color::RgbColor;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Palette {
    Signature,
    Editorial,
    ColorblindSafe,
}

impl Palette {
    pub fn focal_color(self) -> RgbColor {
        match self {
            Palette::Signature => RgbColor::from_hex("#ee7b3d").unwrap(),
            Palette::Editorial => RgbColor::from_hex("#1f6feb").unwrap(),
            Palette::ColorblindSafe => RgbColor::from_hex("#0072b2").unwrap(),
        }
    }

    /// Theme-aware desaturated context color. On dark backgrounds, returns a
    /// medium gray (`#767676`). On light backgrounds, returns a darker gray
    /// (`#4a4a4a`) so it remains legible against white.
    pub fn context_color_for(self, theme: Theme) -> RgbColor {
        match theme {
            Theme::Dark => RgbColor {
                r: 0x76,
                g: 0x76,
                b: 0x76,
            },
            Theme::Light => RgbColor {
                r: 0x4a,
                g: 0x4a,
                b: 0x4a,
            },
        }
    }

    /// Backwards-compatible default — returns the dark-theme context gray.
    pub fn context_color(self) -> RgbColor {
        self.context_color_for(Theme::Dark)
    }

    /// Theme-aware dimmer variant of [`context_color`]. Used for axis ticks and
    /// other secondary chrome.
    pub fn dim_context_color_for(self, theme: Theme) -> RgbColor {
        match theme {
            Theme::Dark => RgbColor {
                r: 0x4f,
                g: 0x4f,
                b: 0x4f,
            },
            Theme::Light => RgbColor {
                r: 0x96,
                g: 0x96,
                b: 0x96,
            },
        }
    }

    /// Backwards-compatible default — returns the dark-theme dim context gray.
    pub fn dim_context_color(self) -> RgbColor {
        self.dim_context_color_for(Theme::Dark)
    }

    pub fn from_name(name: &str) -> Result<Self, UnknownPalette> {
        match name {
            "signature" => Ok(Palette::Signature),
            "editorial" => Ok(Palette::Editorial),
            "colorblind-safe" => Ok(Palette::ColorblindSafe),
            other => Err(UnknownPalette(other.to_string())),
        }
    }
}

#[allow(clippy::derivable_impls)]
impl Default for Palette {
    fn default() -> Self {
        Palette::Signature
    }
}

#[derive(Debug, thiserror::Error)]
#[error("unknown palette: {0}")]
pub struct UnknownPalette(pub String);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signature_palette_focal_is_burnt_orange() {
        let p = Palette::Signature;
        assert_eq!(
            p.focal_color(),
            RgbColor {
                r: 0xee,
                g: 0x7b,
                b: 0x3d
            }
        );
    }

    #[test]
    fn signature_context_color_is_desaturated() {
        let p = Palette::Signature;
        let ctx = p.context_color();
        assert_eq!(ctx.r, ctx.g);
        assert_eq!(ctx.g, ctx.b);
    }

    #[test]
    fn context_gray_for_dark_is_lighter_than_for_light() {
        let dark = Palette::Signature.context_color_for(Theme::Dark);
        let light = Palette::Signature.context_color_for(Theme::Light);
        // On dark BG, gray should be lighter (higher RGB values) for legibility;
        // on light BG, gray should be darker.
        assert!(dark.r > light.r);
    }

    #[test]
    fn context_color_default_path_is_dark() {
        // Backwards-compatible call defaults to Theme::Dark.
        let default = Palette::Signature.context_color();
        let dark = Palette::Signature.context_color_for(Theme::Dark);
        assert_eq!(default, dark);
    }

    #[test]
    fn dim_context_gray_for_dark_is_darker_than_for_light() {
        let dark = Palette::Signature.dim_context_color_for(Theme::Dark);
        let light = Palette::Signature.dim_context_color_for(Theme::Light);
        // Dim is the inverted relationship: on dark BG, dim is darker; on
        // light BG, dim is lighter.
        assert!(light.r > dark.r);
    }

    #[test]
    fn dim_context_color_default_path_is_dark() {
        let default = Palette::Signature.dim_context_color();
        let dark = Palette::Signature.dim_context_color_for(Theme::Dark);
        assert_eq!(default, dark);
    }

    #[test]
    fn parses_palette_name() {
        assert_eq!(Palette::from_name("signature").unwrap(), Palette::Signature);
        assert_eq!(Palette::from_name("editorial").unwrap(), Palette::Editorial);
        assert_eq!(
            Palette::from_name("colorblind-safe").unwrap(),
            Palette::ColorblindSafe
        );
        assert!(Palette::from_name("nope").is_err());
    }
}
