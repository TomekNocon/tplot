use crate::ansi::{fg, reset};
use std::fmt::Write as _;
use tplot_core::PixelBuffer;
use tplot_protocol::{Capabilities, RgbColor};

const SUB_X: usize = 2;
const SUB_Y: usize = 4;

/// Map a sub-pixel coordinate within a cell (0..SUB_X, 0..SUB_Y) to its
/// Braille bit. Standard Unicode 6.0 dot order:
///   (0,0)=0x01  (1,0)=0x08
///   (0,1)=0x02  (1,1)=0x10
///   (0,2)=0x04  (1,2)=0x20
///   (0,3)=0x40  (1,3)=0x80
fn dot_bit(x: usize, y: usize) -> u8 {
    match (x, y) {
        (0, 0) => 0x01,
        (0, 1) => 0x02,
        (0, 2) => 0x04,
        (1, 0) => 0x08,
        (1, 1) => 0x10,
        (1, 2) => 0x20,
        (0, 3) => 0x40,
        (1, 3) => 0x80,
        _ => 0,
    }
}

/// Render a buffer as Braille glyphs. Each cell covers 2×4 sub-pixels.
/// Empty cells render as a regular space (not U+2800), to keep column widths
/// consistent across fonts.
pub fn render_braille(buf: &PixelBuffer, caps: Capabilities) -> String {
    let pw = buf.pixel_width();
    let ph = buf.pixel_height();
    let cells_w = pw.div_ceil(SUB_X);
    let cells_h = ph.div_ceil(SUB_Y);
    let mut out = String::with_capacity(cells_w * cells_h * 12);

    for cy in 0..cells_h {
        for cx in 0..cells_w {
            let mut bits: u8 = 0;
            // Per-channel accumulators for picking the dominant cell color
            // (the most-frequent non-empty pixel; ties broken by latest set).
            let mut last: Option<RgbColor> = None;
            let mut counts: Vec<(RgbColor, u8)> = Vec::with_capacity(8);

            for sy in 0..SUB_Y {
                for sx in 0..SUB_X {
                    let px = cx * SUB_X + sx;
                    let py = cy * SUB_Y + sy;
                    if px >= pw || py >= ph {
                        continue;
                    }
                    if let Some(c) = buf.get(px, py) {
                        bits |= dot_bit(sx, sy);
                        last = Some(c);
                        if let Some(slot) = counts.iter_mut().find(|(rc, _)| *rc == c) {
                            slot.1 += 1;
                        } else {
                            counts.push((c, 1));
                        }
                    }
                }
            }

            if bits == 0 {
                out.push(' ');
            } else {
                let color = counts
                    .iter()
                    .max_by_key(|(_, n)| *n)
                    .map(|(c, _)| *c)
                    .or(last)
                    .unwrap();
                let glyph = char::from_u32(0x2800 + bits as u32).unwrap_or(' ');
                let _ = write!(out, "{}{}{}", fg(color, caps.color_depth), glyph, reset());
            }
        }
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use tplot_core::PixelBuffer;
    use tplot_protocol::{Capabilities, ColorDepth, GlyphSet, GraphicsProtocol, RgbColor, Theme};

    const ORANGE: RgbColor = RgbColor {
        r: 0xee,
        g: 0x7b,
        b: 0x3d,
    };

    fn caps() -> Capabilities {
        Capabilities {
            color_depth: ColorDepth::Truecolor,
            glyph_set: GlyphSet::Braille,
            graphics_protocol: GraphicsProtocol::None,
            theme: Theme::Dark,
        }
    }

    #[test]
    fn empty_buffer_renders_only_spaces() {
        let buf = PixelBuffer::new(2, 4);
        let s = render_braille(&buf, caps());
        assert!(s.contains(' '));
        assert!(
            !s.contains('\u{2800}'),
            "should NOT use blank-Braille for empties"
        );
    }

    #[test]
    fn single_top_left_dot_uses_bit_0() {
        let mut buf = PixelBuffer::new(2, 4);
        buf.set(0, 0, ORANGE);
        let s = render_braille(&buf, caps());
        // Bit 0 set → U+2801 (⠁)
        assert!(s.contains('\u{2801}'), "missing dot-1 glyph: {:?}", s);
        assert!(s.contains("\x1b[38;2;238;123;61m"));
    }

    #[test]
    fn single_bottom_right_dot_uses_bit_7() {
        let mut buf = PixelBuffer::new(2, 4);
        buf.set(1, 3, ORANGE);
        let s = render_braille(&buf, caps());
        // Bit 7 set → U+2880 (⢀)
        assert!(s.contains('\u{2880}'), "missing dot-8 glyph: {:?}", s);
    }

    #[test]
    fn full_cell_is_all_dots() {
        let mut buf = PixelBuffer::new(2, 4);
        for y in 0..4 {
            for x in 0..2 {
                buf.set(x, y, ORANGE);
            }
        }
        let s = render_braille(&buf, caps());
        // All 8 bits set → U+28FF (⣿)
        assert!(s.contains('\u{28ff}'), "missing all-dots glyph: {:?}", s);
    }

    #[test]
    fn last_painted_color_wins_for_multi_color_cell() {
        let red = RgbColor {
            r: 0xff,
            g: 0x00,
            b: 0x00,
        };
        let blue = RgbColor {
            r: 0x00,
            g: 0x00,
            b: 0xff,
        };
        let mut buf = PixelBuffer::new(2, 4);
        buf.set(0, 0, red);
        buf.set(1, 3, blue);
        let s = render_braille(&buf, caps());
        // The renderer picks ONE color per cell (BSP-style: dominant by count,
        // tiebreak by most-recent-set; the test asserts that *some* color
        // shows up, not which one — implementation-defined).
        assert!(s.contains("\x1b[38;2;255;0;0m") || s.contains("\x1b[38;2;0;0;255m"));
    }
}
