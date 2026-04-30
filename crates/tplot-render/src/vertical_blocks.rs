use crate::ansi::{fg, reset};
use std::fmt::Write as _;
use tplot_core::PixelBuffer;
use tplot_protocol::{Capabilities, RgbColor};

/// 8 lower-block glyphs from empty (0/8) to full (8/8) of a cell.
/// Index = number of filled sub-pixel rows in the cell, counted from the bottom.
const GLYPHS: [char; 9] = [
    ' ',        // 0/8
    '\u{2581}', // ▁ 1/8
    '\u{2582}', // ▂ 2/8
    '\u{2583}', // ▃ 3/8
    '\u{2584}', // ▄ 4/8 (lower half)
    '\u{2585}', // ▅ 5/8
    '\u{2586}', // ▆ 6/8
    '\u{2587}', // ▇ 7/8
    '\u{2588}', // █ 8/8 (full)
];

const SUB_PIXELS_PER_CELL: usize = 8;

/// Render a buffer expecting 8 vertical sub-pixels per cell. The buffer must
/// hold solid bottom-up columns (anything above the bar is empty); the
/// renderer counts filled rows from the bottom of each cell.
pub fn render_vertical_blocks(buf: &PixelBuffer, caps: Capabilities) -> String {
    let cells_w = buf.pixel_width();
    let pixel_h = buf.pixel_height();
    let cells_h = pixel_h.div_ceil(SUB_PIXELS_PER_CELL);
    let mut out = String::with_capacity(cells_w * cells_h * 12);

    for cy in 0..cells_h {
        // Cells are walked top-down (cy=0 is the topmost row of the chart).
        let py_top = cy * SUB_PIXELS_PER_CELL;
        // Track active fg color across cells in this row; emit only on change
        // and reset once at end of line.
        let mut last_fg: Option<RgbColor> = None;

        for cx in 0..cells_w {
            // Determine the dominant color in this cell (first non-empty pixel
            // wins — for solid bars they're all the same color, anyway).
            let mut color: Option<RgbColor> = None;
            let mut filled = 0usize;
            for row in 0..SUB_PIXELS_PER_CELL {
                let py = py_top + row;
                if py >= pixel_h {
                    break;
                }
                if let Some(c) = buf.get(cx, py) {
                    color = color.or(Some(c));
                    filled += 1;
                }
            }

            if filled == 0 || color.is_none() {
                if last_fg.is_some() {
                    out.push_str(reset());
                    last_fg = None;
                }
                out.push(' ');
            } else {
                let glyph = GLYPHS[filled.min(8)];
                let c = color.unwrap();
                if last_fg != Some(c) {
                    let _ = write!(out, "{}", fg(c, caps.color_depth));
                    last_fg = Some(c);
                }
                out.push(glyph);
            }
        }

        if last_fg.is_some() {
            out.push_str(reset());
        }
        out.push('\n');
    }
    out
}

#[allow(dead_code)]
fn _typecheck(_: RgbColor) {}

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

    fn paint_column_from_bottom(
        buf: &mut PixelBuffer,
        x: usize,
        fill_height: usize,
        color: RgbColor,
    ) {
        let h = buf.pixel_height();
        for y in (h - fill_height)..h {
            buf.set(x, y, color);
        }
    }

    #[test]
    fn empty_buffer_renders_only_spaces() {
        let buf = PixelBuffer::new(3, 8);
        let s = render_vertical_blocks(&buf, caps());
        assert!(!s.contains('\u{2581}')); // no ▁
        assert!(!s.contains('\u{2588}')); // no █
    }

    #[test]
    fn full_column_renders_full_block() {
        let mut buf = PixelBuffer::new(1, 8);
        paint_column_from_bottom(&mut buf, 0, 8, ORANGE);
        let s = render_vertical_blocks(&buf, caps());
        assert!(s.contains('\u{2588}'));
        assert!(s.contains("\x1b[38;2;238;123;61m"));
    }

    #[test]
    fn one_eighth_column_renders_lower_one_eighth() {
        let mut buf = PixelBuffer::new(1, 8);
        paint_column_from_bottom(&mut buf, 0, 1, ORANGE);
        let s = render_vertical_blocks(&buf, caps());
        assert!(s.contains('\u{2581}'));
    }

    #[test]
    fn five_eighths_column_renders_lower_five_eighths() {
        let mut buf = PixelBuffer::new(1, 8);
        paint_column_from_bottom(&mut buf, 0, 5, ORANGE);
        let s = render_vertical_blocks(&buf, caps());
        assert!(s.contains('\u{2585}'));
    }

    #[test]
    fn taller_than_one_cell_uses_full_blocks_for_lower_cells() {
        // 16 sub-pixels = 2 cells. Bar fills 12 sub-pixels.
        // → bottom cell (rows 8-15): full block (8 filled)
        // → top cell    (rows 0-7):  4 filled from bottom = ▄ (lower-half block, U+2584)
        let mut buf = PixelBuffer::new(1, 16);
        paint_column_from_bottom(&mut buf, 0, 12, ORANGE);
        let s = render_vertical_blocks(&buf, caps());
        assert!(s.contains('\u{2588}')); // full block somewhere
        assert!(s.contains('\u{2584}')); // half block somewhere
    }
}
