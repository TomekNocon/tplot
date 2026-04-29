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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Capabilities {
    pub color_depth: ColorDepth,
    pub glyph_set: GlyphSet,
    pub graphics_protocol: GraphicsProtocol,
}

impl Capabilities {
    pub fn conservative() -> Self {
        Self {
            color_depth: ColorDepth::Ansi256,
            glyph_set: GlyphSet::HalfBlocks,
            graphics_protocol: GraphicsProtocol::None,
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

        Self {
            color_depth,
            glyph_set,
            graphics_protocol,
        }
    }
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
}
