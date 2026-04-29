use crate::PixelBuffer;
use crate::layout::BarLayout;
use std::collections::HashMap;
use tplot_protocol::RgbColor;

pub fn rasterize_bar(
    layout: &BarLayout,
    palette: &HashMap<String, RgbColor>,
    buf: &mut PixelBuffer,
) {
    for bar in &layout.bars {
        let color = palette.get(&bar.series_key).copied().unwrap_or(RgbColor {
            r: 0x6e,
            g: 0x76,
            b: 0x81,
        });
        if bar.pixel_width == 0 {
            continue;
        }
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
    use crate::layout::{BarLayout, BarRect, PlotBox};
    use std::collections::HashMap;
    use tplot_protocol::RgbColor;

    const ORANGE: RgbColor = RgbColor {
        r: 0xee,
        g: 0x7b,
        b: 0x3d,
    };

    fn fake_layout() -> BarLayout {
        BarLayout {
            plot_box: PlotBox {
                pixel_width: 30,
                pixel_height: 8,
            },
            bars: vec![BarRect {
                label: "EMEA".into(),
                value: 72.0,
                pixel_x: 0,
                pixel_y: 0,
                pixel_width: 30,
                pixel_height: 2,
                series_key: "EMEA".into(),
            }],
            canvas_cells_w: 60,
            canvas_cells_h: 12,
            label_margin: 8,
            value_margin: 8,
        }
    }

    #[test]
    fn paints_focal_bar_in_focal_color() {
        let mut buf = crate::PixelBuffer::new(30, 8);
        let mut palette: HashMap<String, RgbColor> = HashMap::new();
        palette.insert("EMEA".into(), ORANGE);
        rasterize_bar(&fake_layout(), &palette, &mut buf);
        assert_eq!(buf.get(0, 0), Some(ORANGE));
        assert_eq!(buf.get(29, 0), Some(ORANGE));
        assert_eq!(buf.get(0, 1), Some(ORANGE));
        // Outside the bar should still be empty.
        assert_eq!(buf.get(0, 2), None);
    }
}
