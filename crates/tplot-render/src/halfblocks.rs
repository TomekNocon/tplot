use crate::ansi::{bg, fg, reset};
use std::fmt::Write as _;
use tplot_core::PixelBuffer;
use tplot_protocol::{Capabilities, RgbColor};

const UPPER: char = '\u{2580}'; // ▀
const LOWER: char = '\u{2584}'; // ▄
const FULL: char = '\u{2588}'; // █

pub fn render_halfblocks(buf: &PixelBuffer, caps: Capabilities) -> String {
    let cells_w = buf.cell_width();
    let cells_h = buf.cell_height();
    let mut out = String::with_capacity(cells_w * cells_h * 12);

    for cy in 0..cells_h {
        let py_top = cy * 2;
        let py_bot = py_top + 1;

        for cx in 0..cells_w {
            let top = buf.get(cx, py_top);
            let bot = buf.get(cx, py_bot);

            match (top, bot) {
                (None, None) => out.push(' '),
                (Some(t), Some(b)) if t == b => {
                    write!(out, "{}{}{}", fg(t, caps.color_depth), FULL, reset()).unwrap();
                }
                (Some(t), Some(b)) => {
                    write!(
                        out,
                        "{}{}{}{}",
                        fg(t, caps.color_depth),
                        bg(b, caps.color_depth),
                        UPPER,
                        reset()
                    )
                    .unwrap();
                }
                (Some(t), None) => {
                    write!(out, "{}{}{}", fg(t, caps.color_depth), UPPER, reset()).unwrap();
                }
                (None, Some(b)) => {
                    write!(out, "{}{}{}", fg(b, caps.color_depth), LOWER, reset()).unwrap();
                }
            }
        }
        out.push('\n');
    }
    out
}

#[allow(dead_code)]
fn _typecheck(_: RgbColor) {} // keep RgbColor import live for future variants

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
            glyph_set: GlyphSet::HalfBlocks,
            graphics_protocol: GraphicsProtocol::None,
            theme: Theme::Dark,
        }
    }

    #[test]
    fn empty_buffer_renders_only_newlines_and_resets() {
        let buf = PixelBuffer::new(4, 2);
        let s = render_halfblocks(&buf, caps());
        assert!(s.lines().count() >= 1);
        // No color escape sequences for empty cells.
        assert!(!s.contains("\x1b[38;2"));
    }

    #[test]
    fn upper_pixel_only_emits_upper_half_block() {
        let mut buf = PixelBuffer::new(2, 2);
        buf.set(0, 0, ORANGE);
        let s = render_halfblocks(&buf, caps());
        // ▀ (upper half block) with orange foreground.
        assert!(s.contains('\u{2580}'));
        assert!(s.contains("\x1b[38;2;238;123;61m"));
    }

    #[test]
    fn both_pixels_set_emit_full_block_when_same_color() {
        let mut buf = PixelBuffer::new(2, 2);
        buf.set(0, 0, ORANGE);
        buf.set(0, 1, ORANGE);
        let s = render_halfblocks(&buf, caps());
        assert!(s.contains('\u{2588}'));
    }

    #[test]
    fn output_size_for_typical_row_is_compact() {
        // 60 same-color cells should produce <= 300 bytes (vs ~1500 before).
        let mut buf = PixelBuffer::new(60, 2);
        for x in 0..60 {
            buf.set(
                x,
                0,
                RgbColor {
                    r: 0xee,
                    g: 0x7b,
                    b: 0x3d,
                },
            );
        }
        let out = render_halfblocks(&buf, caps());
        assert!(out.len() < 300, "output too long: {} bytes", out.len());
    }
}
