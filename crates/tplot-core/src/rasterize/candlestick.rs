//! Candlestick rasterizer — paints a 1-pixel wick from high to low and a
//! filled body rectangle from open to close, colored up vs down.

use crate::PixelBuffer;
use crate::layout::CandlestickLayout;
use tplot_protocol::RgbColor;

pub fn rasterize_candlestick(
    layout: &CandlestickLayout,
    up_color: RgbColor,
    down_color: RgbColor,
    buf: &mut PixelBuffer,
) {
    for c in &layout.candles {
        let color = if c.is_up { up_color } else { down_color };

        // 1. Wick: vertical 1-pixel line at pixel_x.
        buf.draw_line(c.pixel_x, c.wick_top_y, c.pixel_x, c.wick_bottom_y, color);

        // 2. Body: filled rectangle centered on pixel_x.
        let half = layout.bar_cell_width / 2;
        let x0 = c.pixel_x.saturating_sub(half);
        let x1 = c.pixel_x + half;
        buf.fill_rect(x0, c.body_top_y, x1, c.body_bottom_y, color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{Candle, CandlestickLayout, PlotBox};
    use tplot_protocol::RgbColor;

    const GREEN: RgbColor = RgbColor {
        r: 0x3f,
        g: 0xb9,
        b: 0x50,
    };
    const RED: RgbColor = RgbColor {
        r: 0xf8,
        g: 0x51,
        b: 0x49,
    };

    fn fake_layout() -> CandlestickLayout {
        CandlestickLayout {
            plot_box: PlotBox {
                pixel_width: 20,
                pixel_height: 20,
            },
            candles: vec![
                Candle {
                    label: "d1".into(),
                    open: 100.0,
                    high: 112.0,
                    low: 98.0,
                    close: 110.0,
                    pixel_x: 5,
                    wick_top_y: 0,     // pixel for high
                    wick_bottom_y: 19, // pixel for low
                    body_top_y: 5,     // pixel for max(open, close) = 110
                    body_bottom_y: 14, // pixel for min(open, close) = 100
                    is_up: true,
                },
                Candle {
                    label: "d2".into(),
                    open: 110.0,
                    high: 113.0,
                    low: 99.0,
                    close: 105.0,
                    pixel_x: 15,
                    wick_top_y: 1,
                    wick_bottom_y: 18,
                    body_top_y: 6,
                    body_bottom_y: 12,
                    is_up: false,
                },
            ],
            canvas_cells_w: 30,
            canvas_cells_h: 12,
            left_margin: 6,
            bottom_reserve: 3,
            bar_cell_width: 5,
            gap_cell_width: 2,
            y_min: 95.0,
            y_max: 115.0,
        }
    }

    #[test]
    fn up_candle_body_painted_in_green() {
        let mut buf = crate::PixelBuffer::new(20, 20);
        rasterize_candlestick(&fake_layout(), GREEN, RED, &mut buf);
        // Up candle body spans (3..=7, 5..=14).
        assert_eq!(buf.get(5, 7), Some(GREEN));
        assert_eq!(buf.get(3, 10), Some(GREEN));
    }

    #[test]
    fn down_candle_body_painted_in_red() {
        let mut buf = crate::PixelBuffer::new(20, 20);
        rasterize_candlestick(&fake_layout(), GREEN, RED, &mut buf);
        // Down candle body spans (13..=17, 6..=12).
        assert_eq!(buf.get(15, 9), Some(RED));
        assert_eq!(buf.get(13, 12), Some(RED));
    }

    #[test]
    fn wick_extends_above_and_below_body() {
        let mut buf = crate::PixelBuffer::new(20, 20);
        rasterize_candlestick(&fake_layout(), GREEN, RED, &mut buf);
        // Wick of up candle: pixel_x=5, from y=0 to y=19.
        // Above body (y < 5) and below body (y > 14) should be GREEN at x=5.
        assert_eq!(buf.get(5, 0), Some(GREEN));
        assert_eq!(buf.get(5, 19), Some(GREEN));
    }
}
