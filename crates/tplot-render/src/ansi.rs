use tplot_protocol::{ColorDepth, RgbColor};

pub fn reset() -> &'static str {
    "\x1b[0m"
}

pub fn fg(c: RgbColor, depth: ColorDepth) -> String {
    match depth {
        ColorDepth::Truecolor => format!("\x1b[38;2;{};{};{}m", c.r, c.g, c.b),
        ColorDepth::Ansi256 => format!("\x1b[38;5;{}m", to_256(c)),
        ColorDepth::Ansi16 => format!("\x1b[{}m", to_16_fg(c)),
        ColorDepth::Mono => String::new(),
    }
}

pub fn bg(c: RgbColor, depth: ColorDepth) -> String {
    match depth {
        ColorDepth::Truecolor => format!("\x1b[48;2;{};{};{}m", c.r, c.g, c.b),
        ColorDepth::Ansi256 => format!("\x1b[48;5;{}m", to_256(c)),
        ColorDepth::Ansi16 => format!("\x1b[{}m", to_16_fg(c) + 10),
        ColorDepth::Mono => String::new(),
    }
}

fn to_256(c: RgbColor) -> u8 {
    // 6×6×6 color cube starts at 16. Each channel mapped to 0..=5.
    let q = |v: u8| -> u8 {
        if v < 48 {
            0
        } else if v < 115 {
            1
        } else {
            ((v as u16) / 51).min(5) as u8
        }
    };
    16 + 36 * q(c.r) + 6 * q(c.g) + q(c.b)
}

fn to_16_fg(c: RgbColor) -> u32 {
    // Crude perceptual nearest of standard ANSI 0..7 (+8 for bright).
    let (r, g, b) = (c.r as i32, c.g as i32, c.b as i32);
    let bright_threshold = 0xb0;
    let bright = r > bright_threshold || g > bright_threshold || b > bright_threshold;
    let base = match (r > 0x60, g > 0x60, b > 0x60) {
        (false, false, false) => 30,
        (true, false, false) => 31,
        (false, true, false) => 32,
        (true, true, false) => 33,
        (false, false, true) => 34,
        (true, false, true) => 35,
        (false, true, true) => 36,
        (true, true, true) => 37,
    };
    if bright { base + 60 } else { base }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tplot_protocol::{ColorDepth, RgbColor};

    const ORANGE: RgbColor = RgbColor {
        r: 0xee,
        g: 0x7b,
        b: 0x3d,
    };

    #[test]
    fn truecolor_emits_38_2_form() {
        assert_eq!(fg(ORANGE, ColorDepth::Truecolor), "\x1b[38;2;238;123;61m");
    }

    #[test]
    fn ansi256_uses_color_cube_index() {
        let s = fg(ORANGE, ColorDepth::Ansi256);
        // Should match the 6×6×6 color cube (16 + 36*r6 + 6*g6 + b6).
        // For (238,123,61) → r6=4, g6=2, b6=1 → 16 + 144 + 12 + 1 = 173.
        assert_eq!(s, "\x1b[38;5;173m");
    }

    #[test]
    fn reset_is_constant() {
        assert_eq!(reset(), "\x1b[0m");
    }
}
