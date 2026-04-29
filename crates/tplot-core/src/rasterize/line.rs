use crate::PixelBuffer;
use crate::layout::LineLayout;
use std::collections::HashMap;
use tplot_protocol::RgbColor;

/// Rasterize all series into the buffer. The focal series (if Some) is
/// painted LAST so it appears on top at crossings.
pub fn rasterize_line(
    layout: &LineLayout,
    focal: Option<&str>,
    palette: &HashMap<String, RgbColor>,
    buf: &mut PixelBuffer,
) {
    // Paint non-focal first.
    for s in &layout.series {
        if Some(s.key.as_str()) == focal {
            continue;
        }
        paint_series(s, palette, buf);
    }
    // Then paint the focal series on top.
    if let Some(name) = focal
        && let Some(s) = layout.series.iter().find(|s| s.key == name)
    {
        paint_series(s, palette, buf);
    }
}

fn paint_series(
    s: &crate::layout::LineSeries,
    palette: &HashMap<String, RgbColor>,
    buf: &mut PixelBuffer,
) {
    let color = palette.get(&s.key).copied().unwrap_or(RgbColor {
        r: 0x76,
        g: 0x76,
        b: 0x76,
    });
    for w in s.points.windows(2) {
        let (x0, y0) = w[0];
        let (x1, y1) = w[1];
        buf.draw_line(x0, y0, x1, y1, color);
    }
    // Also stamp each individual point so very-short series stay visible.
    for &(x, y) in &s.points {
        buf.set(x, y, color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{LineLayout, LineSeries, PlotBox};
    use std::collections::HashMap;
    use tplot_protocol::RgbColor;

    const ORANGE: RgbColor = RgbColor {
        r: 0xee,
        g: 0x7b,
        b: 0x3d,
    };
    const GRAY: RgbColor = RgbColor {
        r: 0x76,
        g: 0x76,
        b: 0x76,
    };

    fn fake_layout() -> LineLayout {
        LineLayout {
            plot_box: PlotBox {
                pixel_width: 20,
                pixel_height: 8,
            },
            series: vec![
                LineSeries {
                    key: "A".into(),
                    points: vec![(0, 7), (10, 4), (19, 0)],
                },
                LineSeries {
                    key: "B".into(),
                    points: vec![(0, 4), (10, 4), (19, 4)],
                },
            ],
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
    fn paints_each_series_in_its_color() {
        let mut buf = crate::PixelBuffer::new(20, 8);
        let mut palette: HashMap<String, RgbColor> = HashMap::new();
        palette.insert("A".into(), ORANGE);
        palette.insert("B".into(), GRAY);
        rasterize_line(&fake_layout(), Some("A"), &palette, &mut buf);

        // Series A endpoint at (0, 7) should be ORANGE.
        assert_eq!(buf.get(0, 7), Some(ORANGE));
        // Series B at (5, 4) should be GRAY (somewhere along the horizontal line).
        assert_eq!(buf.get(5, 4), Some(GRAY));
    }

    #[test]
    fn focal_series_paints_last_so_it_wins_at_crossings() {
        let mut buf = crate::PixelBuffer::new(20, 8);
        let mut palette: HashMap<String, RgbColor> = HashMap::new();
        palette.insert("A".into(), ORANGE);
        palette.insert("B".into(), GRAY);
        rasterize_line(&fake_layout(), Some("A"), &palette, &mut buf);

        // The lines cross around (10, 4): A passes through, B is horizontal at y=4.
        // Because A is focal, it's painted LAST, so (10,4) should be ORANGE.
        assert_eq!(buf.get(10, 4), Some(ORANGE));
    }
}
