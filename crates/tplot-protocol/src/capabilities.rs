use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ColorDepth {
    Mono,
    Ansi16,
    Ansi256,
    Truecolor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GlyphSet {
    /// ASCII-only fallback (no Unicode block characters).
    Ascii,
    /// Half-blocks `▀▄█`.
    HalfBlocks,
    /// Half-blocks + Braille (assumed in modern terminals).
    Braille,
    /// Half-blocks + Braille + Unicode 16 octants/sextants.
    Octants,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GraphicsProtocol {
    None,
    Kitty,
    ITerm2,
    Sixel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    #[default]
    Dark,
    Light,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Capabilities {
    pub color_depth: ColorDepth,
    pub glyph_set: GlyphSet,
    pub graphics_protocol: GraphicsProtocol,
    pub theme: Theme,
}

impl Capabilities {
    pub fn conservative() -> Self {
        Self {
            color_depth: ColorDepth::Ansi256,
            glyph_set: GlyphSet::HalfBlocks,
            graphics_protocol: GraphicsProtocol::None,
            theme: Theme::Dark,
        }
    }

    /// Detect from environment variables. Caller supplies a getter so tests
    /// can inject env without touching std::env.
    pub fn from_vars<F>(get: F) -> Self
    where
        F: Fn(&str) -> Option<String>,
    {
        let term = get("TERM").unwrap_or_default();
        let colorterm = get("COLORTERM").unwrap_or_default();
        let term_program = get("TERM_PROGRAM").unwrap_or_default();

        let color_depth = if matches!(colorterm.as_str(), "truecolor" | "24bit") {
            ColorDepth::Truecolor
        } else if term.contains("256") {
            ColorDepth::Ansi256
        } else if term.is_empty() || term == "dumb" {
            ColorDepth::Mono
        } else {
            ColorDepth::Ansi16
        };

        // Default to Octants (Unicode 16) on truecolor terminals — most modern
        // fonts ship them by 2025. Probing via OSC happens in plan 5.
        let glyph_set = if matches!(color_depth, ColorDepth::Truecolor) {
            GlyphSet::Octants
        } else if matches!(color_depth, ColorDepth::Ansi256) {
            GlyphSet::Braille
        } else {
            GlyphSet::HalfBlocks
        };

        let graphics_protocol = if term == "xterm-kitty" {
            GraphicsProtocol::Kitty
        } else if term_program == "iTerm.app" {
            GraphicsProtocol::ITerm2
        } else {
            GraphicsProtocol::None
        };

        let theme = detect_theme(&get);

        Self {
            color_depth,
            glyph_set,
            graphics_protocol,
            theme,
        }
    }
}

fn detect_theme<F>(get: &F) -> Theme
where
    F: Fn(&str) -> Option<String>,
{
    // 1. $COLORFGBG = "fg;bg" or "fg;default;bg". A bg digit ≥ 7 (light gray
    //    or white) is a light terminal; ≤ 6 is dark.
    if let Some(raw) = get("COLORFGBG") {
        let parts: Vec<&str> = raw.split(';').collect();
        if parts.len() >= 2
            && let Some(bg_str) = parts.last()
            && let Ok(bg) = bg_str.trim().parse::<u8>()
        {
            return if bg >= 7 { Theme::Light } else { Theme::Dark };
        }
    }
    // 2. Apple Terminal defaults to a light theme.
    if get("TERM_PROGRAM").as_deref() == Some("Apple_Terminal") {
        return Theme::Light;
    }
    // 3. Default to Dark for everything else (most modern dev terminals).
    Theme::Dark
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conservative_is_safe_for_unknown_terminals() {
        let c = Capabilities::conservative();
        assert_eq!(c.color_depth, ColorDepth::Ansi256);
        assert_eq!(c.glyph_set, GlyphSet::HalfBlocks);
        assert_eq!(c.graphics_protocol, GraphicsProtocol::None);
    }

    #[test]
    fn detects_truecolor_from_env() {
        let c = Capabilities::from_vars(|name| match name {
            "COLORTERM" => Some("truecolor".into()),
            "TERM" => Some("xterm-256color".into()),
            _ => None,
        });
        assert_eq!(c.color_depth, ColorDepth::Truecolor);
    }

    #[test]
    fn detects_kitty_graphics() {
        let c = Capabilities::from_vars(|name| match name {
            "TERM" => Some("xterm-kitty".into()),
            "COLORTERM" => Some("truecolor".into()),
            _ => None,
        });
        assert_eq!(c.graphics_protocol, GraphicsProtocol::Kitty);
    }

    #[test]
    fn theme_default_is_dark() {
        let c = Capabilities::conservative();
        assert_eq!(c.theme, Theme::Dark);
    }

    #[test]
    fn theme_dark_when_colorfgbg_says_dark_bg() {
        // $COLORFGBG="15;0" means fg=white(15), bg=black(0).
        let c = Capabilities::from_vars(|name| match name {
            "COLORFGBG" => Some("15;0".into()),
            _ => None,
        });
        assert_eq!(c.theme, Theme::Dark);
    }

    #[test]
    fn theme_light_when_colorfgbg_says_light_bg() {
        // $COLORFGBG="0;15" means fg=black, bg=white(15).
        let c = Capabilities::from_vars(|name| match name {
            "COLORFGBG" => Some("0;15".into()),
            _ => None,
        });
        assert_eq!(c.theme, Theme::Light);
    }

    #[test]
    fn theme_apple_terminal_default_inferred_light() {
        // Apple Terminal defaults to a light theme out of the box.
        let c = Capabilities::from_vars(|name| match name {
            "TERM_PROGRAM" => Some("Apple_Terminal".into()),
            _ => None,
        });
        assert_eq!(c.theme, Theme::Light);
    }

    #[test]
    fn theme_falls_back_to_dark_for_unknown_terminal() {
        let c = Capabilities::from_vars(|_| None);
        assert_eq!(c.theme, Theme::Dark);
    }

    #[test]
    fn theme_falls_back_to_dark_for_malformed_colorfgbg() {
        // Single segment — no bg piece. Fall through to default.
        let c = Capabilities::from_vars(|name| match name {
            "COLORFGBG" => Some("15".into()),
            _ => None,
        });
        assert_eq!(c.theme, Theme::Dark);
    }
}
