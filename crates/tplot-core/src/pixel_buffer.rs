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
}
