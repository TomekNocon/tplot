use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RgbColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Debug, thiserror::Error)]
#[error("invalid hex color: {0}")]
pub struct ParseColorError(pub String);

impl RgbColor {
    pub fn from_hex(s: &str) -> Result<Self, ParseColorError> {
        let s = s.strip_prefix('#').ok_or_else(|| ParseColorError(s.to_string()))?;
        if s.len() != 6 {
            return Err(ParseColorError(s.to_string()));
        }
        let parse = |i: usize| u8::from_str_radix(&s[i..i + 2], 16);
        let r = parse(0).map_err(|_| ParseColorError(s.to_string()))?;
        let g = parse(2).map_err(|_| ParseColorError(s.to_string()))?;
        let b = parse(4).map_err(|_| ParseColorError(s.to_string()))?;
        Ok(RgbColor { r, g, b })
    }

    /// BT.601 luminance, returned as gray RGB.
    pub fn desaturated(self) -> RgbColor {
        let lum = (0.299 * self.r as f32 + 0.587 * self.g as f32 + 0.114 * self.b as f32)
            .round()
            .clamp(0.0, 255.0) as u8;
        // Slightly darker than pure luminance so context recedes.
        let context = (lum as f32 * 0.55).round() as u8;
        RgbColor { r: context, g: context, b: context }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hex() {
        let c = RgbColor::from_hex("#ee7b3d").unwrap();
        assert_eq!(c, RgbColor { r: 0xee, g: 0x7b, b: 0x3d });
    }

    #[test]
    fn rejects_bad_hex() {
        assert!(RgbColor::from_hex("not-a-color").is_err());
        assert!(RgbColor::from_hex("#zzz").is_err());
    }

    #[test]
    fn desaturates_to_grayscale() {
        let orange = RgbColor { r: 0xee, g: 0x7b, b: 0x3d };
        let gray = orange.desaturated();
        // BT.601 luminance check
        assert_eq!(gray.r, gray.g);
        assert_eq!(gray.g, gray.b);
    }
}
