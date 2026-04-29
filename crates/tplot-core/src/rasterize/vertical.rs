use crate::PixelBuffer;
use crate::layout::VerticalBarLayout;
use std::collections::HashMap;
use tplot_protocol::RgbColor;

pub fn rasterize_vertical(
    layout: &VerticalBarLayout,
    palette: &HashMap<String, RgbColor>,
    buf: &mut PixelBuffer,
) {
    for bar in &layout.bars {
        if bar.pixel_height == 0 || bar.pixel_width == 0 {
            continue;
        }
        let color = palette.get(&bar.series_key).copied().unwrap_or(RgbColor {
            r: 0x76,
            g: 0x76,
            b: 0x76,
        });
        let x0 = bar.pixel_x;
        let x1 = bar.pixel_x + bar.pixel_width.saturating_sub(1);
        let y0 = bar.pixel_y;
        let y1 = bar.pixel_y + bar.pixel_height.saturating_sub(1);
        buf.fill_rect(x0, y0, x1, y1, color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{PlotBox, VerticalBarLayout, VerticalBarRect};
    use std::collections::HashMap;
    use tplot_protocol::RgbColor;

    const ORANGE: RgbColor = RgbColor {
        r: 0xee,
        g: 0x7b,
        b: 0x3d,
    };

    fn fake_layout() -> VerticalBarLayout {
        VerticalBarLayout {
            plot_box: PlotBox {
                pixel_width: 16,
                pixel_height: 16,
            },
            bars: vec![
                VerticalBarRect {
                    label: "Apr".into(),
                    value: 21.4,
                    pixel_x: 4,
                    pixel_y: 0,
                    pixel_width: 2,
                    pixel_height: 16,
                    series_key: "Apr".into(),
                },
                VerticalBarRect {
                    label: "Jan".into(),
                    value: 12.4,
                    pixel_x: 0,
                    pixel_y: 7,
                    pixel_width: 2,
                    pixel_height: 9,
                    series_key: "Jan".into(),
                },
            ],
            canvas_cells_w: 30,
            canvas_cells_h: 16,
            bar_cell_width: 2,
            gap_cell_width: 2,
            left_margin: 6,
            bottom_reserve: 3,
        }
    }

    #[test]
    fn paints_vertical_columns_of_correct_color() {
        let mut buf = crate::PixelBuffer::new(16, 16);
        let mut palette: HashMap<String, RgbColor> = HashMap::new();
        palette.insert("Apr".into(), ORANGE);
        rasterize_vertical(&fake_layout(), &palette, &mut buf);
        // Apr bar at x=4, full column (y=0..15).
        assert_eq!(buf.get(4, 0), Some(ORANGE));
        assert_eq!(buf.get(4, 15), Some(ORANGE));
        // Outside the bar, untouched.
        assert_eq!(buf.get(3, 0), None);
    }

    #[test]
    fn paints_only_below_bar_top() {
        let mut buf = crate::PixelBuffer::new(16, 16);
        let mut palette: HashMap<String, RgbColor> = HashMap::new();
        palette.insert("Jan".into(), ORANGE);
        rasterize_vertical(&fake_layout(), &palette, &mut buf);
        // Jan bar starts at pixel_y=7, height=9 → fills y=7..15.
        assert_eq!(buf.get(0, 7), Some(ORANGE));
        assert_eq!(buf.get(0, 15), Some(ORANGE));
        assert_eq!(buf.get(0, 6), None);
    }
}
