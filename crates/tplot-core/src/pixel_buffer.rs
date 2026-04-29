use tplot_protocol::RgbColor;

/// Sub-pixel buffer. Width is in sub-pixel columns; height is in sub-pixel rows.
/// Each terminal cell covers 2 sub-pixels horizontally × 2 vertically (for
/// half-blocks). For Octants/Braille the same buffer is consumed at higher
/// resolution by the renderer (handled in later plans).
#[derive(Debug, Clone)]
pub struct PixelBuffer {
    width: usize,
    height: usize,
    pixels: Vec<Option<RgbColor>>,
}

impl PixelBuffer {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            pixels: vec![None; width * height],
        }
    }

    pub fn pixel_width(&self) -> usize {
        self.width
    }
    pub fn pixel_height(&self) -> usize {
        self.height
    }
    pub fn cell_width(&self) -> usize {
        self.width
    }
    /// For half-blocks: 2 vertical sub-pixels per cell.
    pub fn cell_height(&self) -> usize {
        self.height.div_ceil(2)
    }

    pub fn get(&self, x: usize, y: usize) -> Option<RgbColor> {
        if x >= self.width || y >= self.height {
            return None;
        }
        self.pixels[y * self.width + x]
    }

    pub fn set(&mut self, x: usize, y: usize, color: RgbColor) {
        if x >= self.width || y >= self.height {
            return;
        }
        self.pixels[y * self.width + x] = Some(color);
    }

    pub fn fill_rect(&mut self, x0: usize, y0: usize, x1: usize, y1: usize, color: RgbColor) {
        let x_lo = x0.min(x1);
        let x_hi = x0.max(x1).min(self.width.saturating_sub(1));
        let y_lo = y0.min(y1);
        let y_hi = y0.max(y1).min(self.height.saturating_sub(1));
        for y in y_lo..=y_hi {
            for x in x_lo..=x_hi {
                self.pixels[y * self.width + x] = Some(color);
            }
        }
    }

    /// Bresenham's line algorithm. Inclusive endpoints. Pixels outside the
    /// buffer are silently clipped.
    pub fn draw_line(&mut self, x0: usize, y0: usize, x1: usize, y1: usize, color: RgbColor) {
        let mut x = x0 as isize;
        let mut y = y0 as isize;
        let xe = x1 as isize;
        let ye = y1 as isize;
        let dx = (xe - x).abs();
        let dy = -(ye - y).abs();
        let sx: isize = if x < xe { 1 } else { -1 };
        let sy: isize = if y < ye { 1 } else { -1 };
        let mut err = dx + dy;

        loop {
            if x >= 0 && y >= 0 {
                self.set(x as usize, y as usize, color);
            }
            if x == xe && y == ye {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tplot_protocol::RgbColor;

    const RED: RgbColor = RgbColor {
        r: 0xff,
        g: 0x00,
        b: 0x00,
    };

    #[test]
    fn new_buffer_is_transparent() {
        let buf = PixelBuffer::new(10, 8);
        assert_eq!(buf.cell_width(), 10);
        assert_eq!(buf.cell_height(), 4); // 8 sub-pixel rows = 4 cells (2 per cell)
        assert_eq!(buf.get(0, 0), None);
    }

    #[test]
    fn set_then_get_round_trips() {
        let mut buf = PixelBuffer::new(4, 4);
        buf.set(2, 1, RED);
        assert_eq!(buf.get(2, 1), Some(RED));
        assert_eq!(buf.get(0, 0), None);
    }

    #[test]
    fn fill_rect_paints_inclusive_bounds() {
        let mut buf = PixelBuffer::new(8, 4);
        buf.fill_rect(2, 1, 5, 2, RED);
        for y in 1..=2 {
            for x in 2..=5 {
                assert_eq!(buf.get(x, y), Some(RED));
            }
        }
        assert_eq!(buf.get(1, 1), None);
        assert_eq!(buf.get(6, 1), None);
    }

    #[test]
    fn draw_horizontal_line() {
        let mut buf = PixelBuffer::new(10, 4);
        buf.draw_line(1, 2, 8, 2, RED);
        for x in 1..=8 {
            assert_eq!(buf.get(x, 2), Some(RED), "missing pixel at x={x}");
        }
        assert_eq!(buf.get(0, 2), None);
        assert_eq!(buf.get(9, 2), None);
    }

    #[test]
    fn draw_vertical_line() {
        let mut buf = PixelBuffer::new(4, 10);
        buf.draw_line(2, 1, 2, 8, RED);
        for y in 1..=8 {
            assert_eq!(buf.get(2, y), Some(RED), "missing pixel at y={y}");
        }
    }

    #[test]
    fn draw_diagonal_line() {
        let mut buf = PixelBuffer::new(8, 8);
        buf.draw_line(0, 0, 7, 7, RED);
        for i in 0..=7 {
            assert_eq!(buf.get(i, i), Some(RED), "missing diagonal at ({i},{i})");
        }
    }

    #[test]
    fn draw_line_works_in_either_direction() {
        let mut a = PixelBuffer::new(10, 10);
        let mut b = PixelBuffer::new(10, 10);
        a.draw_line(2, 1, 7, 8, RED);
        b.draw_line(7, 8, 2, 1, RED);
        for y in 0..10 {
            for x in 0..10 {
                assert_eq!(a.get(x, y), b.get(x, y), "asymmetry at ({x},{y})");
            }
        }
    }
}
