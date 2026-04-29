use crate::PixelBuffer;
use crate::layout::ScatterLayout;
use std::collections::HashMap;
use tplot_protocol::RgbColor;

pub fn rasterize_scatter(
    layout: &ScatterLayout,
    focal: Option<&str>,
    palette: &HashMap<String, RgbColor>,
    buf: &mut PixelBuffer,
) {
    // Paint non-focal first, focal last (consistent with rasterize_line).
    for s in &layout.series {
        if Some(s.key.as_str()) == focal {
            continue;
        }
        paint_points(s, palette, buf);
    }
    if let Some(name) = focal {
        if let Some(s) = layout.series.iter().find(|s| s.key == name) {
            paint_points(s, palette, buf);
        }
    }
}

fn paint_points(
    s: &crate::layout::ScatterSeries,
    palette: &HashMap<String, RgbColor>,
    buf: &mut PixelBuffer,
) {
    let color = palette.get(&s.key).copied().unwrap_or(RgbColor {
        r: 0x76,
        g: 0x76,
        b: 0x76,
    });
    for &(x, y) in &s.points {
        buf.set(x, y, color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{PlotBox, ScatterLayout, ScatterSeries};
    use std::collections::HashMap;
    use tplot_protocol::RgbColor;

    const ORANGE: RgbColor = RgbColor {
        r: 0xee,
        g: 0x7b,
        b: 0x3d,
    };

    fn fake_layout() -> ScatterLayout {
        ScatterLayout {
            plot_box: PlotBox {
                pixel_width: 10,
                pixel_height: 8,
            },
            series: vec![ScatterSeries {
                key: "A".into(),
                points: vec![(2, 3), (5, 6)],
            }],
            canvas_cells_w: 30,
            canvas_cells_h: 8,
            left_margin: 6,
            bottom_reserve: 3,
            x_min: 0.0,
            x_max: 5.0,
            y_min: 0.0,
            y_max: 50.0,
        }
    }

    #[test]
    fn paints_each_point_in_series_color() {
        let mut buf = crate::PixelBuffer::new(10, 8);
        let mut palette: HashMap<String, RgbColor> = HashMap::new();
        palette.insert("A".into(), ORANGE);
        rasterize_scatter(&fake_layout(), None, &palette, &mut buf);
        assert_eq!(buf.get(2, 3), Some(ORANGE));
        assert_eq!(buf.get(5, 6), Some(ORANGE));
        // Non-painted pixels remain empty.
        assert_eq!(buf.get(3, 3), None);
    }
}
