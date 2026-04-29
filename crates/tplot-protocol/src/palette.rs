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
            Palette::Signature      => RgbColor::from_hex("#ee7b3d").unwrap(),
            Palette::Editorial      => RgbColor::from_hex("#1f6feb").unwrap(),
            Palette::ColorblindSafe => RgbColor::from_hex("#0072b2").unwrap(),
        }
    }

    pub fn context_color(self) -> RgbColor {
        // Desaturated context color. All palettes share the same gray.
        RgbColor { r: 0x76, g: 0x76, b: 0x76 }
    }

    pub fn dim_context_color(self) -> RgbColor {
        RgbColor { r: 0x4f, g: 0x4f, b: 0x4f }
    }

    pub fn from_name(name: &str) -> Result<Self, UnknownPalette> {
        match name {
            "signature"       => Ok(Palette::Signature),
            "editorial"       => Ok(Palette::Editorial),
            "colorblind-safe" => Ok(Palette::ColorblindSafe),
            other             => Err(UnknownPalette(other.to_string())),
        }
    }
}

impl Default for Palette {
    fn default() -> Self { Palette::Signature }
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
        assert_eq!(p.focal_color(), RgbColor { r: 0xee, g: 0x7b, b: 0x3d });
    }

    #[test]
    fn signature_context_color_is_desaturated() {
        let p = Palette::Signature;
        let ctx = p.context_color();
        assert_eq!(ctx.r, ctx.g);
        assert_eq!(ctx.g, ctx.b);
    }

    #[test]
    fn parses_palette_name() {
        assert_eq!(Palette::from_name("signature").unwrap(), Palette::Signature);
        assert_eq!(Palette::from_name("editorial").unwrap(), Palette::Editorial);
        assert_eq!(Palette::from_name("colorblind-safe").unwrap(), Palette::ColorblindSafe);
        assert!(Palette::from_name("nope").is_err());
    }
}
